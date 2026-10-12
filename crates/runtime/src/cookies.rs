//! WebKit neither stores nor sends cookies for a custom scheme, so the runtime
//! plays the browser's part.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use cookie::Cookie;
use cookie::time::OffsetDateTime;
use http::header::{COOKIE, SET_COOKIE};
use http::{HeaderValue, Request, Response};

/// The cookies PHP set, sent back with the following requests.
///
/// It lives on the PHP thread: requests are served one at a time, so the
/// cookies of a response always reach the next request.
///
/// As in a browser, the cookies with an expiry date outlive the application
/// when the jar has a file; session cookies end with it.
#[derive(Default)]
pub(crate) struct CookieJar {
    cookies: Vec<Cookie<'static>>,
    file: Option<PathBuf>,
}

impl CookieJar {
    /// The jar saved in `file`, empty if there is none yet.
    pub(crate) fn load(file: PathBuf) -> Self {
        let now = OffsetDateTime::now_utc();
        let cookies = match fs::read_to_string(&file) {
            Ok(content) => content
                .lines()
                .filter_map(|line| match Cookie::parse(line.to_owned()) {
                    Ok(cookie) => Some(cookie),
                    Err(error) => {
                        eprintln!(
                            "[cookies] Ignored an invalid line of {}: {error}.",
                            file.display()
                        );
                        None
                    }
                })
                .filter(|cookie| is_persistent(cookie, now))
                .collect(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                eprintln!(
                    "[cookies] Could not read {}, starting without cookies: {error}.",
                    file.display()
                );
                Vec::new()
            }
        };

        Self {
            cookies,
            file: Some(file),
        }
    }

    pub(crate) fn add_to(&self, request: &mut Request<Vec<u8>>) {
        let now = OffsetDateTime::now_utc();
        let path = request.uri().path();

        let cookies = self
            .cookies
            .iter()
            .filter(|cookie| {
                !is_expired(cookie, now) && path_matches(cookie.path().unwrap_or("/"), path)
            })
            .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
            .collect::<Vec<_>>();

        if cookies.is_empty() {
            return;
        }

        // Keep what WebKit sent, if a future version sends anything.
        let mut header = request
            .headers()
            .get(COOKIE)
            .and_then(|value| value.to_str().ok())
            .map(|value| vec![value.to_owned()])
            .unwrap_or_default();
        header.extend(cookies);

        match HeaderValue::try_from(header.join("; ")) {
            Ok(value) => {
                request.headers_mut().insert(COOKIE, value);
            }
            Err(error) => eprintln!("[cookies] Could not build the Cookie header: {error}."),
        }
    }

    pub(crate) fn store(&mut self, request_path: &str, response: &Response<Vec<u8>>) {
        let now = OffsetDateTime::now_utc();
        let mut changed = false;

        for value in response.headers().get_all(SET_COOKIE) {
            let Some(mut cookie) = value
                .to_str()
                .ok()
                .and_then(|value| Cookie::parse(value.to_owned()).ok())
            else {
                eprintln!("[cookies] Ignored an invalid Set-Cookie header: {value:?}.");
                continue;
            };

            // RFC 6265, 5.2.4: a path that does not start with "/" is the
            // default one.
            if !cookie.path().is_some_and(|path| path.starts_with('/')) {
                cookie.set_path(default_path(request_path));
            }
            // Max-Age wins over Expires (RFC 6265, 5.3): turn it into a date
            // now, which a saved cookie keeps. A huge one (PHP_INT_MAX) is as
            // good as never.
            if let Some(max_age) = cookie.max_age() {
                let expires = now
                    .checked_add(max_age)
                    .unwrap_or(cookie::time::PrimitiveDateTime::MAX.assume_utc());
                cookie.set_expires(expires);
                cookie.set_max_age(None);
            }

            self.cookies
                .retain(|stored| stored.name() != cookie.name() || stored.path() != cookie.path());
            if !is_expired(&cookie, now) {
                self.cookies.push(cookie);
            }
            changed = true;
        }

        if changed && let Some(file) = &self.file {
            let content: String = self
                .cookies
                .iter()
                .filter(|cookie| is_persistent(cookie, now))
                .map(|cookie| format!("{cookie}\n"))
                .collect();
            if let Err(error) = save(file, &content) {
                eprintln!(
                    "[cookies] Could not save the cookies to {}: {error}.",
                    file.display()
                );
            }
        }
    }
}

/// Session ids are as good as passwords: readable by the user only, and
/// written aside then renamed, so that a crash never leaves half a file.
fn save(file: &Path, content: &str) -> io::Result<()> {
    let temporary = file.with_extension("tmp");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(&temporary)?.write_all(content.as_bytes())?;

    fs::rename(&temporary, file)
}

/// Whether `cookie` outlives the application: it has an expiry date, to come.
fn is_persistent(cookie: &Cookie<'_>, now: OffsetDateTime) -> bool {
    cookie
        .expires_datetime()
        .is_some_and(|expires| expires > now)
}

fn is_expired(cookie: &Cookie<'_>, now: OffsetDateTime) -> bool {
    cookie
        .expires_datetime()
        .is_some_and(|expires| expires <= now)
}

/// RFC 6265, 5.1.4.
fn path_matches(cookie_path: &str, request_path: &str) -> bool {
    request_path == cookie_path
        || (request_path.starts_with(cookie_path)
            && (cookie_path.ends_with('/') || request_path[cookie_path.len()..].starts_with('/')))
}

/// RFC 6265, 5.1.4: the directory of the request path.
fn default_path(request_path: &str) -> String {
    match request_path.rfind('/') {
        Some(0) | None => "/".to_owned(),
        Some(last_slash) => request_path[..last_slash].to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(set_cookies: &[&str]) -> Response<Vec<u8>> {
        let mut builder = Response::builder();
        for set_cookie in set_cookies {
            builder = builder.header(SET_COOKIE, *set_cookie);
        }

        builder.body(Vec::new()).unwrap()
    }

    fn cookie_header(jar: &CookieJar, path: &str) -> Option<String> {
        let mut request = Request::get(format!("php://localhost{path}"))
            .body(Vec::new())
            .unwrap();
        jar.add_to(&mut request);

        request
            .headers()
            .get(COOKIE)
            .map(|value| value.to_str().unwrap().to_owned())
    }

    #[test]
    fn sends_back_what_php_set() {
        let mut jar = CookieJar::default();
        jar.store(
            "/login",
            &response(&[
                "PHPSESSID=abc; path=/; httponly; samesite=lax",
                "theme=dark; path=/",
            ]),
        );

        assert_eq!(
            cookie_header(&jar, "/").as_deref(),
            Some("PHPSESSID=abc; theme=dark")
        );
    }

    #[test]
    fn replaces_a_cookie_and_forgets_an_expired_one() {
        let mut jar = CookieJar::default();
        jar.store(
            "/",
            &response(&["PHPSESSID=abc; path=/", "theme=dark; path=/"]),
        );
        jar.store(
            "/",
            &response(&["PHPSESSID=def; path=/", "theme=deleted; path=/; max-age=0"]),
        );

        assert_eq!(cookie_header(&jar, "/").as_deref(), Some("PHPSESSID=def"));
    }

    #[test]
    fn a_huge_max_age_never_expires() {
        let mut jar = CookieJar::default();
        jar.store(
            "/",
            &response(&["remember=1; path=/; max-age=999999999999999"]),
        );

        assert_eq!(cookie_header(&jar, "/").as_deref(), Some("remember=1"));
    }

    #[test]
    fn a_relative_path_is_the_default_one() {
        let mut jar = CookieJar::default();
        jar.store("/admin/users", &response(&["theme=dark; path=admin"]));

        assert_eq!(
            cookie_header(&jar, "/admin/x").as_deref(),
            Some("theme=dark")
        );
    }

    #[test]
    fn keeps_the_cookies_with_an_expiry_date() {
        let file = Path::new(env!("OUT_DIR")).join(format!("cookies-{}", std::process::id()));
        let _ = fs::remove_file(&file);

        let mut jar = CookieJar::load(file.clone());
        jar.store(
            "/",
            &response(&[
                "PHPSESSID=abc; path=/",
                "remember=1; path=/; max-age=3600",
                "gone=1; path=/; expires=Thu, 01 Jan 1970 00:00:00 GMT",
            ]),
        );

        assert_eq!(
            cookie_header(&CookieJar::load(file.clone()), "/").as_deref(),
            Some("remember=1")
        );

        jar.store("/", &response(&["remember=; path=/; max-age=0"]));
        assert_eq!(cookie_header(&CookieJar::load(file.clone()), "/"), None);

        fs::remove_file(file).unwrap();
    }

    #[test]
    fn honors_paths() {
        let mut jar = CookieJar::default();
        jar.store("/admin/users", &response(&["admin=1", "api=1; path=/api"]));

        assert_eq!(
            cookie_header(&jar, "/admin/users/42").as_deref(),
            Some("admin=1")
        );
        assert_eq!(cookie_header(&jar, "/api/me").as_deref(), Some("api=1"));
        assert_eq!(cookie_header(&jar, "/apiary"), None);
        assert_eq!(cookie_header(&jar, "/"), None);
    }
}

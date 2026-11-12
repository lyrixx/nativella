//! WebKit follows no redirect answered through a custom scheme, neither for a
//! navigation nor for `fetch()`: the runtime does it.
//!
//! - A navigation gets a page that replaces itself with the target, so the
//!   address and the history end up where a browser would put them.
//! - A `fetch()` or XHR gets the target's response directly. The page then
//!   sees `redirected: false` and the original URL: a known limitation. So
//!   does a form sent to a 307 or a 308, which sends its body again.

use std::collections::HashMap;

use http::header::{CONTENT_LENGTH, CONTENT_TYPE, COOKIE, LOCATION};
use http::{Method, Request, Response, StatusCode, Uri};
use url::Url;

/// Redirects followed in a row before giving up, as browsers do: by the
/// runtime for one `fetch()`, by a window from page to page ([`Chains`]).
pub(crate) const MAX_REDIRECTS: usize = 20;

pub(crate) enum Next {
    /// Send this response to the webview.
    Respond(Response<Vec<u8>>),
    /// Send this page to the webview, which replaces itself with `target`.
    Replace {
        page: Response<Vec<u8>>,
        target: Uri,
    },
    /// Serve this request instead: the target of a redirect.
    Follow(Request<Vec<u8>>),
}

/// The redirects each window is following from page to page. The webview
/// sees as many pages, not a chain of redirects it would give up on: two
/// pages that redirect to each other would load forever.
#[derive(Default)]
pub(crate) struct Chains {
    /// By window: where its last page sent it, and after how many redirects.
    chains: HashMap<String, (Uri, usize)>,
}

impl Chains {
    /// `window` asks for `uri`. Unless its last page sent it there, its
    /// chain is over: the user went elsewhere, or the target was a file of
    /// public/, which the runtime never saw.
    pub(crate) fn navigate(&mut self, window: &str, uri: &Uri) {
        if self
            .chains
            .get(window)
            .is_some_and(|(target, _)| target != uri)
        {
            self.chains.remove(window);
        }
    }

    /// A page sends `window` to `target`. `false` after too many in a row.
    pub(crate) fn redirect(&mut self, window: &str, target: Uri) -> bool {
        let followed = self.chains.get(window).map_or(0, |(_, followed)| *followed);
        if followed == MAX_REDIRECTS {
            self.chains.remove(window);

            return false;
        }
        self.chains
            .insert(window.to_owned(), (target, followed + 1));

        true
    }

    /// `window` got a page that stays.
    pub(crate) fn arrive(&mut self, window: &str) {
        self.chains.remove(window);
    }
}

/// WebKit sends no `Sec-Fetch-Mode` to a custom scheme, but navigations carry
/// `Upgrade-Insecure-Requests` and `fetch()` never does.
pub fn is_navigation(request: &Request<Vec<u8>>) -> bool {
    request.headers().contains_key("upgrade-insecure-requests")
}

/// What to do with `response`, given the request it answers.
pub(crate) fn next(
    request: Request<Vec<u8>>,
    response: Response<Vec<u8>>,
    navigation: bool,
) -> Next {
    if !response.status().is_redirection() {
        return Next::Respond(response);
    }
    // Bytes beyond ASCII (an accented query) are percent-encoded by the join.
    let Some(location) = response
        .headers()
        .get(LOCATION)
        .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
    else {
        return Next::Respond(response);
    };
    let location = location.as_str();

    let base = php_url(request.uri());
    let target = match base.join(location) {
        Ok(target) => as_php_url(target, request.uri()),
        Err(error) => {
            log!("[redirect] Ignored the invalid Location \"{location}\": {error}.");

            return Next::Respond(response);
        }
    };
    let same_origin = target.origin() == base.origin();

    // Browsers only follow redirects to web pages: a `javascript:` or `data:`
    // Location would run in the origin of the application.
    if !same_origin && !matches!(target.scheme(), "http" | "https") {
        log!(
            "[redirect] Refused to follow a redirect to a {} URL.",
            target.scheme()
        );

        return Next::Respond(crate::empty_response(StatusCode::BAD_GATEWAY));
    }

    if navigation && !same_origin {
        log!(
            "[redirect] {} to {target}, another website: replaced by a client-side navigation.",
            response.status().as_u16()
        );

        return Next::Respond(leave_for(&target));
    }

    // A page that replaces itself would load the target with a GET: the
    // runtime sends the method and body again itself, as for a fetch().
    let sends_the_body_again = matches!(
        response.status(),
        StatusCode::TEMPORARY_REDIRECT | StatusCode::PERMANENT_REDIRECT
    ) && !matches!(*request.method(), Method::GET | Method::HEAD);

    if navigation && !sends_the_body_again {
        // The whole URL: "//docs" alone would name another host.
        let uri = webview_uri(request.uri(), &path_and_query(&target));
        let mut href = uri.to_string();
        if let Some(fragment) = target.fragment() {
            href.push('#');
            href.push_str(fragment);
        }
        log!(
            "[redirect] {} to {href}, replaced by a client-side navigation.",
            response.status().as_u16()
        );

        return Next::Replace {
            page: navigate_to(&href),
            target: uri,
        };
    }

    // Another origin is not served by PHP: the page gets the redirect itself.
    if !same_origin {
        return Next::Respond(response);
    }

    let (mut parts, mut body) = request.into_parts();
    let status = response.status();
    let becomes_get = status == StatusCode::SEE_OTHER && parts.method != Method::HEAD
        || matches!(status, StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND)
            && parts.method == Method::POST;
    if becomes_get {
        parts.method = Method::GET;
        parts.headers.remove(CONTENT_TYPE);
        parts.headers.remove(CONTENT_LENGTH);
        body.clear();
    }
    // The cookie jar adds the cookies again, maybe new ones.
    parts.headers.remove(COOKIE);
    parts.uri = webview_uri(&parts.uri, &path_and_query(&target));

    Next::Follow(Request::from_parts(parts, body))
}

/// The URL PHP believes it serves: plain HTTP on port 80 of the request host,
/// see the server variables in sapi.rs. Applications build their absolute
/// URLs from it.
fn php_url(uri: &Uri) -> Url {
    let path = uri.path_and_query().map_or("/", |path| path.as_str());

    Url::parse(&format!(
        "http://{}{path}",
        uri.host().unwrap_or("localhost")
    ))
    .expect("A host and a path from a valid URI make a valid URL.")
}

/// A Location on the webview's own scheme ("php://localhost/…", as a
/// framework told so builds it) is the same page as on PHP's: the origin of
/// a custom scheme is opaque, and would never match.
fn as_php_url(target: Url, uri: &Uri) -> Url {
    if Some(target.scheme()) != uri.scheme_str() || target.host_str() != uri.host() {
        return target;
    }

    let mut php = php_url(uri);
    php.set_path(target.path());
    php.set_query(target.query());
    php.set_fragment(target.fragment());

    php
}

fn webview_uri(uri: &Uri, path_and_query: &str) -> Uri {
    let mut parts = uri.clone().into_parts();
    parts.path_and_query = Some(
        path_and_query
            .parse()
            .expect("A path from a valid URL is valid."),
    );

    Uri::from_parts(parts).expect("Changing the path of a valid URI keeps it valid.")
}

fn path_and_query(url: &Url) -> String {
    match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_owned(),
    }
}

fn navigate_to(href: &str) -> Response<Vec<u8>> {
    page(&format!(
        "<script>location.replace({});</script>",
        js_string(href)
    ))
}

/// The window does not leave for another website: the shell opens it in the
/// browser, and the window stays on this page, which says so (an OAuth
/// login, for one).
fn leave_for(target: &Url) -> Response<Vec<u8>> {
    page(&format!(
        "<script>location.replace({});</script><p>This page continues in your browser. <a href=\"/\" onclick=\"if (history.length > 1) {{ history.back(); return false; }}\">Back</a></p>",
        js_string(target.as_str())
    ))
}

fn page(content: &str) -> Response<Vec<u8>> {
    Response::builder()
        .header(CONTENT_TYPE, "text/html; charset=UTF-8")
        .body(format!("<!DOCTYPE html><meta charset=\"UTF-8\">{content}").into_bytes())
        .expect("A content type is a valid header.")
}

fn js_string(value: &str) -> String {
    // A JSON string is a valid JavaScript string; escaping "<" keeps
    // "</script>" out of the page.
    serde_json::to_string(value)
        .expect("A string serializes to JSON.")
        .replace('<', "\\u003c")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redirect(status: u16, location: &str) -> Response<Vec<u8>> {
        Response::builder()
            .status(status)
            .header(LOCATION, location)
            .body(Vec::new())
            .unwrap()
    }

    fn post(uri: &str) -> Request<Vec<u8>> {
        Request::post(uri)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(COOKIE, "PHPSESSID=abc")
            .body(b"title=Hello".to_vec())
            .unwrap()
    }

    #[test]
    fn a_fetch_follows_a_post_redirect_with_a_get() {
        let Next::Follow(request) = next(
            post("php://localhost/native/title"),
            redirect(302, "/?done=1"),
            false,
        ) else {
            panic!("The redirect is not followed.");
        };

        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.uri(), "php://localhost/?done=1");
        assert!(request.body().is_empty());
        assert!(!request.headers().contains_key(CONTENT_TYPE));
        assert!(!request.headers().contains_key(COOKIE));
    }

    #[test]
    fn a_fetch_keeps_the_method_and_body_for_a_307() {
        let Next::Follow(request) = next(
            post("php://localhost/a"),
            redirect(307, "http://localhost/b"),
            false,
        ) else {
            panic!("The redirect is not followed.");
        };

        assert_eq!(request.method(), Method::POST);
        assert_eq!(request.uri(), "php://localhost/b");
        assert_eq!(request.body(), b"title=Hello");
    }

    #[test]
    fn a_fetch_follows_a_location_on_the_webview_scheme() {
        let Next::Follow(request) = next(
            post("php://localhost/tasks"),
            redirect(302, "php://localhost/?added=1"),
            false,
        ) else {
            panic!("The redirect is not followed.");
        };

        assert_eq!(request.uri(), "php://localhost/?added=1");
    }

    #[test]
    fn a_navigation_keeps_the_fragment() {
        let Next::Replace { page: response, .. } = next(
            post("php://localhost/notes"),
            redirect(303, "/?q=x#note-5"),
            true,
        ) else {
            panic!("A navigation is never followed by the runtime.");
        };

        assert!(
            String::from_utf8(response.into_body())
                .unwrap()
                .contains(r#"location.replace("php://localhost/?q=x#note-5")"#)
        );
    }

    #[test]
    fn a_navigation_to_a_path_of_two_slashes_stays_in_the_application() {
        let Next::Replace { page: response, .. } = next(
            post("php://localhost/notes"),
            redirect(302, "http://localhost//docs"),
            true,
        ) else {
            panic!("A navigation is never followed by the runtime.");
        };

        assert!(
            String::from_utf8(response.into_body())
                .unwrap()
                .contains(r#"location.replace("php://localhost//docs")"#)
        );
    }

    #[test]
    fn a_window_gives_up_on_pages_that_redirect_to_each_other() {
        let login: Uri = "php://localhost/login".parse().unwrap();
        let home: Uri = "php://localhost/".parse().unwrap();
        let mut chains = Chains::default();

        for _ in 0..MAX_REDIRECTS / 2 {
            chains.navigate("main", &home);
            assert!(chains.redirect("main", login.clone()));
            chains.navigate("main", &login);
            assert!(chains.redirect("main", home.clone()));
        }
        chains.navigate("main", &home);
        assert!(!chains.redirect("main", login.clone()));

        assert!(chains.redirect("main", login), "The window may try again.");
    }

    #[test]
    fn a_chain_ends_where_the_window_stays_or_goes_elsewhere() {
        let home: Uri = "php://localhost/".parse().unwrap();
        let page: Uri = "php://localhost/index.html".parse().unwrap();
        let mut chains = Chains::default();

        // A file of public/ answers the target: the runtime only sees the
        // next visit of the home page.
        for _ in 0..=MAX_REDIRECTS {
            chains.navigate("main", &home);
            assert!(chains.redirect("main", page.clone()));
        }

        for _ in 0..=MAX_REDIRECTS {
            chains.navigate("main", &home);
            assert!(chains.redirect("main", home.clone()));
            chains.navigate("main", &home);
            chains.arrive("main");
        }

        chains.navigate("settings", &home);
        assert!(chains.redirect("settings", home));
    }

    #[test]
    fn a_redirect_never_runs_script() {
        for location in [
            "javascript:alert(document.cookie)",
            "data:text/html,<script>alert(1)</script>",
        ] {
            let Next::Respond(response) =
                next(post("php://localhost/login"), redirect(302, location), true)
            else {
                panic!("A navigation is never followed by the runtime.");
            };

            assert_eq!(response.status(), StatusCode::BAD_GATEWAY, "{location}");
            assert!(response.body().is_empty());
        }
    }

    #[test]
    fn a_location_beyond_ascii_is_followed() {
        let mut response = redirect(302, "/");
        response.headers_mut().insert(
            LOCATION,
            http::HeaderValue::from_bytes("/search?q=café".as_bytes()).unwrap(),
        );

        let Next::Follow(request) = next(post("php://localhost/search"), response, false) else {
            panic!("The redirect is not followed.");
        };

        assert_eq!(request.uri(), "php://localhost/search?q=caf%C3%A9");
    }

    #[test]
    fn a_fetch_does_not_follow_another_origin() {
        let response = next(
            post("php://localhost/a"),
            redirect(302, "https://example.com/"),
            false,
        );

        assert!(
            matches!(response, Next::Respond(response) if response.status() == StatusCode::FOUND)
        );
    }

    #[test]
    fn a_form_sent_to_a_307_is_sent_again_to_the_target() {
        let Next::Follow(request) = next(
            post("php://localhost/notes"),
            redirect(307, "/notes/new"),
            true,
        ) else {
            panic!("A 307 sends the form again.");
        };

        assert_eq!(request.method(), Method::POST);
        assert_eq!(request.uri(), "php://localhost/notes/new");
        assert_eq!(request.body(), b"title=Hello");
    }

    #[test]
    fn a_navigation_to_another_website_leaves_a_way_back() {
        let Next::Respond(response) = next(
            post("php://localhost/connect/github"),
            redirect(302, "https://github.com/login/oauth/authorize?state=<x>"),
            true,
        ) else {
            panic!("A navigation is never followed by the runtime.");
        };
        let page = String::from_utf8(response.into_body()).unwrap();

        assert!(
            page.contains(
                r#"location.replace("https://github.com/login/oauth/authorize?state=%3Cx%3E")"#
            ),
            "{page}"
        );
        assert!(page.contains("history.back()"), "{page}");
    }

    #[test]
    fn a_navigation_replaces_itself_with_the_target() {
        let Next::Replace {
            page: response,
            target,
        } = next(
            post("php://localhost/native/title"),
            redirect(302, "/?from=post"),
            true,
        )
        else {
            panic!("A navigation is never followed by the runtime.");
        };

        assert_eq!(target, "php://localhost/?from=post");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            String::from_utf8(response.into_body()).unwrap(),
            r#"<!DOCTYPE html><meta charset="UTF-8"><script>location.replace("php://localhost/?from=post");</script>"#
        );
    }
}

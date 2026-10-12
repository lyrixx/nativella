//! WebKit follows no redirect answered through a custom scheme, neither for a
//! navigation nor for `fetch()`: the runtime does it.
//!
//! - A navigation gets a page that replaces itself with the target, so the
//!   address and the history end up where a browser would put them.
//! - A `fetch()` or XHR gets the target's response directly. The page then
//!   sees `redirected: false` and the original URL: a known limitation.

use http::header::{CONTENT_LENGTH, CONTENT_TYPE, COOKIE, LOCATION};
use http::{Method, Request, Response, StatusCode, Uri};
use url::Url;

/// Redirects followed for one `fetch()` before giving up, as browsers do.
pub(crate) const MAX_REDIRECTS: usize = 20;

pub(crate) enum Next {
    /// Send this response to the webview.
    Respond(Response<Vec<u8>>),
    /// Serve this request instead: the target of a redirect.
    Follow(Request<Vec<u8>>),
}

/// WebKit sends no `Sec-Fetch-Mode` to a custom scheme, but navigations carry
/// `Upgrade-Insecure-Requests` and `fetch()` never does.
pub(crate) fn is_navigation(request: &Request<Vec<u8>>) -> bool {
    request.headers().contains_key("upgrade-insecure-requests")
}

/// What to do with `response`, given a copy of the request it answers.
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
            eprintln!("[redirect] Ignored the invalid Location \"{location}\": {error}.");

            return Next::Respond(response);
        }
    };
    let same_origin = target.origin() == base.origin();

    // Browsers only follow redirects to web pages: a `javascript:` or `data:`
    // Location would run in the origin of the application.
    if !same_origin && !matches!(target.scheme(), "http" | "https") {
        eprintln!(
            "[redirect] Refused to follow a redirect to a {} URL.",
            target.scheme()
        );

        return Next::Respond(
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Vec::new())
                .expect("A response with a status and no header is valid."),
        );
    }

    if navigation {
        let href = if same_origin {
            let mut href = path_and_query(&target);
            if let Some(fragment) = target.fragment() {
                href.push('#');
                href.push_str(fragment);
            }

            href
        } else {
            target.to_string()
        };
        eprintln!(
            "[redirect] {} to {href}, replaced by a client-side navigation.",
            response.status().as_u16()
        );

        return Next::Respond(navigate_to(&href));
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

/// A copy of `request`, to build the request that follows a redirect.
pub(crate) fn copy(request: &Request<Vec<u8>>) -> Request<Vec<u8>> {
    let mut copy = Request::new(request.body().clone());
    *copy.method_mut() = request.method().clone();
    *copy.uri_mut() = request.uri().clone();
    *copy.version_mut() = request.version();
    *copy.headers_mut() = request.headers().clone();

    copy
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
    // A JSON string is a valid JavaScript string; escaping "<" keeps
    // "</script>" out of the page.
    let target = serde_json::to_string(href)
        .expect("A string serializes to JSON.")
        .replace('<', "\\u003c");
    let body = format!(
        "<!DOCTYPE html><meta charset=\"UTF-8\"><script>location.replace({target});</script>"
    );

    Response::builder()
        .header(CONTENT_TYPE, "text/html; charset=UTF-8")
        .body(body.into_bytes())
        .expect("A content type is a valid header.")
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
        let Next::Respond(response) = next(
            post("php://localhost/notes"),
            redirect(303, "/?q=x#note-5"),
            true,
        ) else {
            panic!("A navigation is never followed by the runtime.");
        };

        assert!(
            String::from_utf8(response.into_body())
                .unwrap()
                .contains(r#"location.replace("/?q=x#note-5")"#)
        );
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
    fn a_navigation_replaces_itself_with_the_target() {
        let Next::Respond(response) = next(
            post("php://localhost/native/title"),
            redirect(302, "/?from=post"),
            true,
        ) else {
            panic!("A navigation is never followed by the runtime.");
        };

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            String::from_utf8(response.into_body()).unwrap(),
            r#"<!DOCTYPE html><meta charset="UTF-8"><script>location.replace("/?from=post");</script>"#
        );
    }
}

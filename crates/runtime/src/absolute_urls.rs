//! PHP believes it is served over plain HTTP (see the server variables in
//! sapi.rs), so the absolute URLs it builds point to `http://localhost/`, where
//! nothing listens. The ones pages and styles load or link to get the
//! webview's own scheme back: `php://localhost/`.
//!
//! Only URL attributes of HTML (`href`, `src`, `action`…) and `url()` of CSS
//! are rewritten: text, form values, scripts and JSON documents are the
//! user's data, which may well mention `http://localhost`.
//!
//! Where the webview itself uses HTTP (Windows, Android: `http://php.localhost/`),
//! there is nothing to do.

use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use http::{HeaderValue, Response, Uri};

/// HTML attributes that hold a URL.
const URL_ATTRIBUTES: &[&[u8]] = &[
    b"action",
    b"background",
    b"cite",
    b"data",
    b"formaction",
    b"href",
    b"poster",
    b"src",
    b"srcset",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Html,
    Css,
}

/// Rewrites the absolute URLs of `response`, the answer to a request for `uri`.
pub(crate) fn rewrite(uri: &Uri, response: &mut Response<Vec<u8>>) {
    let (Some(scheme), Some(host)) = (uri.scheme_str(), uri.host()) else {
        return;
    };
    if scheme == "http" {
        return;
    }
    let Some(kind) = kind(response) else {
        return;
    };

    let from = format!("http://{host}");
    let to = format!("{scheme}://{host}");
    let Some(body) = replace_origin(response.body(), from.as_bytes(), to.as_bytes(), kind) else {
        return;
    };
    *response.body_mut() = body;

    if response.headers().contains_key(CONTENT_LENGTH) {
        let length = HeaderValue::from(response.body().len());
        response.headers_mut().insert(CONTENT_LENGTH, length);
    }
}

fn kind(response: &Response<Vec<u8>>) -> Option<Kind> {
    let content_type = response.headers().get(CONTENT_TYPE)?.to_str().ok()?;
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    match mime.as_str() {
        "text/html" => Some(Kind::Html),
        "text/css" => Some(Kind::Css),
        _ => None,
    }
}

/// `haystack` with every `from` origin in a URL context replaced by `to`,
/// `None` if there is none. An origin ends where the host does:
/// `http://localhost.example.com` and `http://localhost:8000` are others.
fn replace_origin(haystack: &[u8], from: &[u8], to: &[u8], kind: Kind) -> Option<Vec<u8>> {
    let is_origin_at = |index: usize| {
        haystack[index..].starts_with(from)
            && !haystack
                .get(index + from.len())
                .is_some_and(|&byte| byte.is_ascii_alphanumeric() || b".-_:@".contains(&byte))
            && is_url_context(&haystack[..index], kind)
    };

    let mut result: Option<Vec<u8>> = None;
    let mut copied = 0;
    let mut index = 0;
    while index + from.len() <= haystack.len() {
        if is_origin_at(index) {
            let result = result.get_or_insert_with(|| Vec::with_capacity(haystack.len() + 64));
            result.extend_from_slice(&haystack[copied..index]);
            result.extend_from_slice(to);
            index += from.len();
            copied = index;
        } else {
            index += 1;
        }
    }

    let mut result = result?;
    result.extend_from_slice(&haystack[copied..]);

    Some(result)
}

/// Whether what precedes a URL makes it one to load or link to: an HTML URL
/// attribute (`href="`, `src='`, `action=`) or a CSS `url(`.
fn is_url_context(before: &[u8], kind: Kind) -> bool {
    let before = before
        .strip_suffix(b"\"")
        .or_else(|| before.strip_suffix(b"'"))
        .unwrap_or(before);

    if before.len() >= 4 && before[before.len() - 4..].eq_ignore_ascii_case(b"url(") {
        return true;
    }
    if kind == Kind::Css {
        return false;
    }

    let Some(before) = before.strip_suffix(b"=") else {
        return false;
    };
    let name_start = before
        .iter()
        .rposition(|byte| !(byte.is_ascii_alphanumeric() || *byte == b'-'))
        .map_or(0, |position| position + 1);
    let name = &before[name_start..];
    // `data-href=` is no URL attribute: the name is a whole attribute.
    let whole = name_start > 0 && before[name_start - 1].is_ascii_whitespace();

    whole
        && URL_ATTRIBUTES
            .iter()
            .any(|attribute| attribute.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(content_type: &str, body: &str) -> Response<Vec<u8>> {
        Response::builder()
            .header(CONTENT_TYPE, content_type)
            .header(CONTENT_LENGTH, body.len())
            .body(body.as_bytes().to_vec())
            .unwrap()
    }

    fn rewritten(uri: &str, mut response: Response<Vec<u8>>) -> String {
        rewrite(&uri.parse().unwrap(), &mut response);

        String::from_utf8(response.into_body()).unwrap()
    }

    #[test]
    fn urls_to_load_get_the_webview_scheme() {
        let page = rewritten(
            "php://localhost/notes",
            response(
                "text/html; charset=UTF-8",
                r#"<link rel="stylesheet" href="http://localhost/_wdt/styles"><a href='http://localhost'>Home</a><form action=http://localhost/save><img SRC="http://localhost/logo.png">"#,
            ),
        );

        assert_eq!(
            page,
            r#"<link rel="stylesheet" href="php://localhost/_wdt/styles"><a href='php://localhost'>Home</a><form action=php://localhost/save><img SRC="php://localhost/logo.png">"#
        );
    }

    #[test]
    fn css_urls_too() {
        let style = rewritten(
            "php://localhost/",
            response(
                "text/css",
                "body { background: url(\"http://localhost/bg.png\") }",
            ),
        );

        assert_eq!(
            style,
            "body { background: url(\"php://localhost/bg.png\") }"
        );
    }

    #[test]
    fn the_data_of_the_user_is_left_alone() {
        let page = rewritten(
            "php://localhost/",
            response(
                "text/html",
                r#"<textarea>curl http://localhost/api</textarea><input value="http://localhost/x"><div data-href="http://localhost/y"></div><p>http://localhost/</p>"#,
            ),
        );
        assert_eq!(
            page,
            r#"<textarea>curl http://localhost/api</textarea><input value="http://localhost/x"><div data-href="http://localhost/y"></div><p>http://localhost/</p>"#
        );

        let json = rewritten(
            "php://localhost/api",
            response("application/json", r#"{"url":"http:\/\/localhost\/notes"}"#),
        );
        assert_eq!(json, r#"{"url":"http:\/\/localhost\/notes"}"#);
    }

    #[test]
    fn other_hosts_are_left_alone() {
        let page = rewritten(
            "php://localhost/",
            response(
                "text/html",
                r#"<a href="http://localhost.example.com/"><a href="http://localhost:8000/"><a href="http://example.com/">"#,
            ),
        );

        assert_eq!(
            page,
            r#"<a href="http://localhost.example.com/"><a href="http://localhost:8000/"><a href="http://example.com/">"#
        );
    }

    #[test]
    fn nothing_to_do_when_the_webview_uses_http() {
        let page = rewritten(
            "http://php.localhost/",
            response("text/html", r#"<a href="http://php.localhost/">"#),
        );

        assert_eq!(page, r#"<a href="http://php.localhost/">"#);
    }
}

//! PHP believes it is served over plain HTTP (see the server variables in
//! sapi.rs), so the absolute URLs it builds point to `http://localhost/`, where
//! nothing listens. The ones pages and styles load or link to get the
//! webview's own scheme back: `php://localhost/`.
//!
//! Only URL attributes of HTML (`href`, `src`, `action`…) and `url()` of CSS,
//! in a style sheet or in a `style` attribute or element, are rewritten:
//! text, form values, scripts and JSON documents are the user's data, which
//! may well mention `http://localhost`. The shell brings
//! back the requests scripts send there (`crates/nativella/src/absolute_urls.rs`).
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
    };

    // Where the page is at `index`, read once from its start: looking back
    // from every URL would read a long text again and again.
    let mut place = Place::Text;
    let mut result: Option<Vec<u8>> = None;
    let mut copied = 0;
    let mut index = 0;
    while index + from.len() <= haystack.len() {
        let is_url_context = |before: &[u8]| match kind {
            Kind::Css => ends_with_url_function(before),
            Kind::Html => place.is_url_context(before),
        };
        if is_origin_at(index) && is_url_context(&haystack[..index]) {
            let result = result.get_or_insert_with(|| Vec::with_capacity(haystack.len() + 64));
            result.extend_from_slice(&haystack[copied..index]);
            result.extend_from_slice(to);
            // An origin holds nothing that changes the place.
            index += from.len();
            copied = index;
        } else {
            if kind == Kind::Html {
                place = place.after(haystack, index);
            }
            index += 1;
        }
    }

    let mut result = result?;
    result.extend_from_slice(&haystack[copied..]);

    Some(result)
}

/// The elements whose content is text, whatever it looks like.
const TEXT_ELEMENTS: &[&[u8]] = &[b"script", b"style", b"textarea", b"title"];

/// Where a byte of an HTML page is.
#[derive(Clone, Copy)]
enum Place {
    Text,
    /// In a comment, until `-->`.
    Comment,
    /// In the content of one of `TEXT_ELEMENTS`, until its end tag.
    TextElement(&'static [u8]),
    /// In a tag, between its attributes.
    Tag {
        start: usize,
    },
    /// In the value of an attribute, after the quote at `quote`.
    Value {
        tag: usize,
        quote: usize,
    },
}

impl Place {
    /// The place after the byte at `index`, given the one before it.
    fn after(self, page: &[u8], index: usize) -> Self {
        let byte = page[index];
        let rest = &page[index + 1..];

        match self {
            Self::Text if byte == b'<' && rest.starts_with(b"!--") => Self::Comment,
            // "1 < 2" opens no tag.
            Self::Text
                if byte == b'<'
                    && rest
                        .first()
                        .is_some_and(|next| next.is_ascii_alphabetic() || b"/!".contains(next)) =>
            {
                Self::Tag { start: index }
            }
            Self::Comment if byte == b'>' && page[..index].ends_with(b"--") => Self::Text,
            Self::TextElement(name)
                if byte == b'<'
                    && rest.first() == Some(&b'/')
                    && rest[1..]
                        .get(..name.len())
                        .is_some_and(|end| end.eq_ignore_ascii_case(name)) =>
            {
                Self::Tag { start: index }
            }
            Self::Tag { start } if matches!(byte, b'"' | b'\'') => Self::Value {
                tag: start,
                quote: index,
            },
            Self::Tag { start } if byte == b'>' => {
                let tag = &page[start + 1..index];
                let name_end = tag
                    .iter()
                    .position(|byte| !byte.is_ascii_alphanumeric())
                    .unwrap_or(tag.len());

                TEXT_ELEMENTS
                    .iter()
                    .find(|name| name.eq_ignore_ascii_case(&tag[..name_end]))
                    .map_or(Self::Text, |name| Self::TextElement(name))
            }
            Self::Value { tag, quote } if byte == page[quote] => Self::Tag { start: tag },
            place => place,
        }
    }

    /// Whether a URL that follows `before` is one to load or link to: the
    /// value of a URL attribute (`href="`, `src='`, `action=`), a candidate
    /// of a `srcset` after the first one (`srcset="a.png 1x, `) or a CSS
    /// `url(`, in a `style` attribute or element.
    fn is_url_context(self, before: &[u8]) -> bool {
        match self {
            Self::Tag { .. } => is_attribute(before, URL_ATTRIBUTES),
            Self::Value { quote, .. } => {
                let (attribute, value) = (&before[..quote], &before[quote + 1..]);

                value.is_empty() && is_attribute(attribute, URL_ATTRIBUTES)
                    || value.trim_ascii_end().ends_with(b",")
                        && is_attribute(attribute, &[b"srcset"])
                    || ends_with_url_function(value) && is_attribute(attribute, &[b"style"])
            }
            Self::TextElement(name) => name == b"style" && ends_with_url_function(before),
            Self::Text | Self::Comment => false,
        }
    }
}

/// Whether `before` ends with `url(`, and a quote or not.
fn ends_with_url_function(before: &[u8]) -> bool {
    let unquoted = before
        .strip_suffix(b"\"")
        .or_else(|| before.strip_suffix(b"'"))
        .unwrap_or(before);

    unquoted.len() >= 4 && unquoted[unquoted.len() - 4..].eq_ignore_ascii_case(b"url(")
}

/// Whether `before` ends with `name=`, `name` one of `names`.
fn is_attribute(before: &[u8], names: &[&[u8]]) -> bool {
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

    whole && names.iter().any(|known| known.eq_ignore_ascii_case(name))
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
    fn every_candidate_of_a_srcset() {
        let page = rewritten(
            "php://localhost/",
            response(
                "text/html",
                r#"<img srcset="http://localhost/a.png 1x, http://localhost/b.png 2x"><img srcset='http://localhost/a.png 480w,http://localhost/b.png 800w'><img alt="1x, http://localhost/c.png"><img srcset="a.png"> 1x, http://localhost/d.png"#,
            ),
        );

        assert_eq!(
            page,
            r#"<img srcset="php://localhost/a.png 1x, php://localhost/b.png 2x"><img srcset='php://localhost/a.png 480w,php://localhost/b.png 800w'><img alt="1x, http://localhost/c.png"><img srcset="a.png"> 1x, http://localhost/d.png"#
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
    fn css_urls_of_a_page_too() {
        let page = rewritten(
            "php://localhost/",
            response(
                "text/html",
                r#"<style media="screen">a > b { background: url(http://localhost/a.png) }</style><p style="background: url('http://localhost/b.png')">It's</p><!-- it's --><a href="http://localhost/c">"#,
            ),
        );

        assert_eq!(
            page,
            r#"<style media="screen">a > b { background: url(php://localhost/a.png) }</style><p style="background: url('php://localhost/b.png')">It's</p><!-- it's --><a href="php://localhost/c">"#
        );
    }

    #[test]
    fn a_long_srcset_is_read_once() {
        let candidates = 200_000;
        let page = rewritten(
            "php://localhost/",
            response(
                "text/html",
                &format!(
                    r#"<img srcset="{}">"#,
                    "http://localhost/a.png 1x, ".repeat(candidates)
                ),
            ),
        );

        assert_eq!(page.matches("php://localhost/a.png").count(), candidates);
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

        let page = rewritten(
            "php://localhost/",
            response(
                "text/html",
                r#"<textarea>&lt;a href=http://localhost/x&gt; url(http://localhost/bg.png)</textarea><input value="url(http://localhost/y)"><script type="application/json">{"css":"url(http://localhost/z)"}</script><!-- <a href="http://localhost/old"> --><p>1 < 2, see href=http://localhost/</p>"#,
            ),
        );
        assert_eq!(
            page,
            r#"<textarea>&lt;a href=http://localhost/x&gt; url(http://localhost/bg.png)</textarea><input value="url(http://localhost/y)"><script type="application/json">{"css":"url(http://localhost/z)"}</script><!-- <a href="http://localhost/old"> --><p>1 < 2, see href=http://localhost/</p>"#
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

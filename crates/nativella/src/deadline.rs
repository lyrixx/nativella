//! Android's webview waits 30 seconds for a page, then loads its URL from the
//! network instead and shows an error (wry, src/android/mod.rs). PHP may take
//! longer: the user left a dialog, a file picker or a fingerprint prompt open.
//! Before the webview gives up, the page gets a stand-in that waits for PHP,
//! asking the shell, not PHP, whether PHP is free.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::http::header::{ACCEPT, CONTENT_TYPE, REFERER};
use tauri::http::uri::PathAndQuery;
use tauri::http::{Method, Request, Response, StatusCode, Uri};

use crate::native::lock;

/// Under the 30 seconds of wry.
#[cfg(target_os = "android")]
const DEADLINE: Duration = Duration::from_secs(25);

/// Answered by the shell: whether PHP has no request left to serve.
pub(crate) const IDLE_PATH: &str = "/_nativella/idle";

type Respond = Box<dyn FnOnce(Response<Vec<u8>>) + Send>;

#[cfg(target_os = "android")]
pub(crate) fn idle(idle: bool) -> Response<Vec<u8>> {
    Response::builder()
        .header(CONTENT_TYPE, "application/json")
        .body(serde_json::json!({ "idle": idle }).to_string().into_bytes())
        .expect("A static header is valid.")
}

/// Wraps `respond`: the request gets its response if it comes before the
/// deadline, a stand-in otherwise.
#[cfg(target_os = "android")]
pub(crate) fn guard(
    request: &Request<Vec<u8>>,
    page: bool,
    respond: impl FnOnce(Response<Vec<u8>>) + Send + 'static,
) -> impl FnOnce(Response<Vec<u8>>) + Send + 'static {
    guard_within(DEADLINE, request, page, respond)
}

/// Whether the webview shows the response as a page: a stand-in page would
/// break a script that waits for data, or for a form response (Turbo), which
/// asks for HTML too.
#[cfg(target_os = "android")]
pub(crate) fn is_page(request: &Request<Vec<u8>>) -> bool {
    !crate::request_bodies::is_script(request) && accepts_html(request)
}

fn accepts_html(request: &Request<Vec<u8>>) -> bool {
    request
        .headers()
        .get(ACCEPT)
        .and_then(|accept| accept.to_str().ok())
        .is_some_and(|accept| accept.contains("text/html"))
}

fn guard_within(
    deadline: Duration,
    request: &Request<Vec<u8>>,
    page: bool,
    respond: impl FnOnce(Response<Vec<u8>>) + Send + 'static,
) -> impl FnOnce(Response<Vec<u8>>) + Send + 'static {
    let respond: Arc<Mutex<Option<Respond>>> = Arc::new(Mutex::new(Some(Box::new(respond))));
    // Dropped with the closure below: the response came.
    let (answered, waiting) = mpsc::channel::<()>();
    let stand_in = stand_in(request, page);
    let uri = request.uri().clone();

    let watched = Arc::clone(&respond);
    let spawned = thread::Builder::new()
        .name("deadline".to_owned())
        .spawn(move || {
            if waiting.recv_timeout(deadline) != Err(RecvTimeoutError::Timeout) {
                return;
            }
            if let Some(respond) = lock(&watched).take() {
                eprintln!(
                    "[nativella] {uri} takes more than {} seconds: the page waits for it.",
                    deadline.as_secs()
                );
                respond(stand_in);
            }
        });
    if let Err(error) = spawned {
        eprintln!("[nativella] No deadline for {}: {error}.", request.uri());
    }

    move |response| {
        let respond = lock(&respond).take();
        drop(answered);
        match respond {
            Some(respond) => respond(response),
            None => eprintln!(
                "[nativella] A response came after its stand-in, status {}: dropped.",
                response.status()
            ),
        }
    }
}

/// A page that loads again once PHP is free: a GET runs again, a form goes
/// back to its page rather than being sent twice. It asks the shell until
/// then: reloading would queue a request after the slow one each time.
fn stand_in(request: &Request<Vec<u8>>, page: bool) -> Response<Vec<u8>> {
    if !page {
        return Response::builder()
            .status(StatusCode::GATEWAY_TIMEOUT)
            .body(Vec::new())
            .expect("A response with a status and no header is valid.");
    }

    let target = if request.method() == Method::GET {
        Some(request.uri().clone())
    } else {
        request
            .headers()
            .get(REFERER)
            .and_then(|referer| referer.to_str().ok())
            .and_then(|referer| referer.parse::<Uri>().ok())
    };
    let target = target
        .as_ref()
        .and_then(Uri::path_and_query)
        .map_or("/", PathAndQuery::as_str);

    // A JSON string in a script: no "</script>" can close it.
    let target = serde_json::to_string(target)
        .expect("A string is valid JSON.")
        .replace("</", "<\\/");
    let html = format!(
        r#"<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width">
<link rel="icon" href="data:,">
<title>Waiting…</title>
<p style="font:16px system-ui,sans-serif;margin:2em;color:#666">Waiting for the application…</p>
<script>
(function wait() {{
    fetch({idle:?})
        .then(response => response.json())
        .then(state => state.idle ? location.replace({target}) : setTimeout(wait, 500))
        .catch(() => setTimeout(wait, 1000));
}})();
</script>
"#,
        idle = IDLE_PATH,
    );

    Response::builder()
        .header(CONTENT_TYPE, "text/html; charset=utf-8")
        .body(html.into_bytes())
        .expect("A static header is valid.")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(method: Method, uri: &str) -> tauri::http::request::Builder {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(ACCEPT, "text/html,application/xhtml+xml")
    }

    fn body(response: &Response<Vec<u8>>) -> String {
        String::from_utf8(response.body().clone()).unwrap()
    }

    #[test]
    fn a_fast_response_goes_through() {
        let (sender, receiver) = mpsc::channel();
        let request = page(Method::GET, "http://php.localhost/")
            .body(Vec::new())
            .unwrap();
        let respond = guard_within(
            Duration::from_millis(200),
            &request,
            true,
            move |response| {
                sender.send(response.status()).unwrap();
            },
        );

        respond(Response::new(Vec::new()));

        assert_eq!(receiver.recv().unwrap(), StatusCode::OK);
        assert!(receiver.recv_timeout(Duration::from_millis(400)).is_err());
    }

    #[test]
    fn a_slow_form_goes_back_to_its_page() {
        let (sender, receiver) = mpsc::channel();
        let request = page(
            Method::POST,
            "http://php.localhost/snippets/2/delete?nativella-body=1",
        )
        .header(REFERER, "http://php.localhost/?q=a&b")
        .body(Vec::new())
        .unwrap();
        let respond = guard_within(Duration::from_millis(50), &request, true, move |response| {
            sender.send(body(&response)).unwrap();
        });

        let stand_in = receiver.recv().unwrap();
        respond(Response::new(Vec::new()));

        assert!(
            stand_in.contains(r#"location.replace("/?q=a&b")"#),
            "{stand_in}"
        );
        assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());
    }

    #[test]
    fn a_slow_page_loads_again() {
        let request = page(Method::GET, "http://php.localhost/notes/1")
            .body(Vec::new())
            .unwrap();

        let stand_in = body(&stand_in(&request, accepts_html(&request)));
        assert!(
            stand_in.contains(r#"location.replace("/notes/1")"#),
            "{stand_in}"
        );
        assert!(
            stand_in.contains(r#"fetch("/_nativella/idle")"#),
            "{stand_in}"
        );
    }

    #[test]
    fn a_slow_fetch_fails() {
        let request = Request::post("http://php.localhost/api")
            .body(Vec::new())
            .unwrap();

        assert_eq!(
            stand_in(&request, accepts_html(&request)).status(),
            StatusCode::GATEWAY_TIMEOUT
        );
    }

    #[test]
    fn a_slow_script_that_asks_for_html_fails() {
        // Turbo submits forms with fetch(), asking for HTML: the page is
        // its own, not a stand-in.
        let request = page(Method::POST, "http://php.localhost/notes")
            .body(Vec::new())
            .unwrap();

        assert_eq!(
            stand_in(&request, false).status(),
            StatusCode::GATEWAY_TIMEOUT
        );
    }
}

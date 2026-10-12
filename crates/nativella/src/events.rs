//! Native events: what happens on the native side (a menu click, a window
//! focus…) and reaches PHP as an internal request, and the pages as a DOM
//! event.
//!
//! Event names and payloads: keep in sync with
//! packages/nativella/src/Event/EventFactory.php.

use nativella_runtime::{PhpRuntime, RequestOrigin};
use serde_json::{Value, json};
use tauri::http::Request;
use tauri::http::header::CONTENT_TYPE;
use tauri::{AppHandle, Manager};

use crate::native::{self, window};
use crate::{APP_URL, MAIN_WINDOW};

/// Where PHP receives native events. PHP frameworks route it like any other
/// request, guarded by `$_SERVER['NATIVELLA_INTERNAL']`.
pub(crate) const EVENT_PATH: &str = "_nativella/event";

/// Notifies PHP of `name`, then the pages, as a `nativella:<name>` DOM event.
pub(crate) fn send(app: &AppHandle, name: &str, payload: Value) {
    send_then(app, name, payload, || {});
}

/// As `send`, then calls `handled` once PHP has answered: to send events no
/// faster than PHP handles them.
pub(crate) fn send_then(
    app: &AppHandle,
    name: &str,
    payload: Value,
    handled: impl FnOnce() + Send + 'static,
) {
    native::page::dispatch(app, name, &payload, None);

    let name = name.to_owned();
    app.state::<PhpRuntime>().dispatch(
        request(&name, &payload),
        RequestOrigin::Internal,
        move |response| {
            if !response.status().is_success() {
                eprintln!(
                    "[event] PHP answered {} to the \"{name}\" event.",
                    response.status()
                );
            }
            handled();
        },
    );
}

/// Sends `app.booted`, which PHP answers before anything else: it migrates
/// its database, opens windows, sets menus… If PHP opens no window, the main
/// one opens on the home page.
pub(crate) fn boot(app: &AppHandle) {
    let handle = app.clone();

    // A PHP app that knows nothing of Nativella answers 404, which is fine.
    app.state::<PhpRuntime>().dispatch(
        request("app.booted", &json!({})),
        RequestOrigin::Internal,
        move |response| {
            if !response.status().is_success() && response.status().as_u16() != 404 {
                eprintln!(
                    "[event] PHP answered {} to the \"app.booted\" event.",
                    response.status()
                );
            }

            if handle.webview_windows().is_empty() {
                let options = handle.state::<native::State>().default_window().clone();
                if let Err(error) = window::open(&handle, MAIN_WINDOW, options) {
                    eprintln!("[nativella] Could not open the main window: {error}");
                    handle.exit(1);
                }
            }
        },
    );
}

fn request(name: &str, payload: &Value) -> Request<Vec<u8>> {
    let body = json!({ "name": name, "payload": payload })
        .to_string()
        .into_bytes();

    Request::post(format!("{APP_URL}{EVENT_PATH}"))
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .expect("The event URL and header are valid.")
}

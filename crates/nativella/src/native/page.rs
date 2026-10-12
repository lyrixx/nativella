//! `page.*`: talks to the JavaScript of the pages, which listen with
//! `window.addEventListener('nativella:<name>', event => event.detail)`.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

use super::Call;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Dispatch {
    event: String,
    #[serde(default)]
    detail: Value,
    /// Every window when absent.
    window: Option<String>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "dispatch" => call.payload::<Dispatch>().and_then(|payload| {
            if let Some(window) = &payload.window
                && call.app.get_webview_window(window).is_none()
            {
                return Err(format!("There is no \"{window}\" window."));
            }
            dispatch(
                call.app,
                &payload.event,
                &payload.detail,
                payload.window.as_deref(),
            );

            Ok(json!({}))
        }),
        _ => return None,
    })
}

/// Dispatches `nativella:<event>` on the `window` object of the pages.
pub(crate) fn dispatch(app: &AppHandle, event: &str, detail: &Value, window: Option<&str>) {
    // JSON is valid JavaScript: names and payloads need no other escaping.
    let script = format!(
        "window.dispatchEvent(new CustomEvent({}, {{ detail: {detail} }}));",
        Value::String(format!("nativella:{event}")),
    );

    for (label, webview) in app.webview_windows() {
        if window.is_some_and(|window| window != label) {
            continue;
        }
        if let Err(error) = webview.eval(&script) {
            eprintln!("[page] Could not dispatch \"{event}\" in the \"{label}\" window: {error}.");
        }
    }
}

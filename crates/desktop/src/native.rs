//! The native methods PHP calls through `native_call()`.
//!
//! Keep in sync with the callers of `App\Native\Native::call()` in app/src/.

use serde_json::{Value, json};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_notification::NotificationExt;

use crate::MAIN_WINDOW;

/// Runs on the PHP thread: blocking here (a modal dialog) is fine, the main
/// thread keeps the UI alive.
pub fn call(app: &AppHandle, method: &str, payload: &Value) -> Result<Value, String> {
    match method {
        "dialog.ask" => {
            let answer = app
                .dialog()
                .message(string(payload, "message")?)
                .title(string(payload, "title")?)
                .buttons(MessageDialogButtons::YesNo)
                .blocking_show();

            Ok(json!({ "answer": answer }))
        }
        "dialog.pick_file" => {
            let path = app
                .dialog()
                .file()
                .blocking_pick_file()
                .map(|path| path.into_path().map_err(|error| error.to_string()))
                .transpose()?;

            Ok(json!({ "path": path }))
        }
        "notification.show" => {
            app.notification()
                .builder()
                .title(string(payload, "title")?)
                .body(string(payload, "body")?)
                .show()
                .map_err(|error| error.to_string())?;

            Ok(json!({}))
        }
        "window.set_title" => {
            app.get_webview_window(MAIN_WINDOW)
                .ok_or("The main window is closed.")?
                .set_title(string(payload, "title")?)
                .map_err(|error| error.to_string())?;

            Ok(json!({}))
        }
        _ => Err(format!("Unknown native method \"{method}\".")),
    }
}

fn string<'a>(payload: &'a Value, key: &str) -> Result<&'a str, String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("The payload has no \"{key}\" string."))
}

//! `share.*`: the share sheet of the phone, with a text or a file. No
//! permission: the user picks where it goes, or nothing.
//!
//! The sheet opens and PHP goes on: tauri-plugin-sharekit answers once the
//! sheet is closed, which says nothing of what the user did (Android's
//! chooser reports a cancellation either way).

use std::path::PathBuf;
use std::thread;

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::plugin::mobile::PluginInvokeError;
use tauri::{AppHandle, WebviewWindow};
use tauri_plugin_sharekit::{Error, ShareExt, ShareFileOptions, ShareTextOptions};

use super::{Call, failed};

/// What the plugin rejects with when the user closes the sheet.
const CANCELLED: &str = "Share cancelled";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Text {
    text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct File {
    path: PathBuf,
    mime_type: Option<String>,
    title: Option<String>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "text" => call.payload::<Text>().and_then(|payload| {
            let window = call.target_window(None)?;
            open(call.app, window, move |app, window| {
                app.share()
                    .share_text(window, payload.text, ShareTextOptions::default())
            })
        }),
        "file" => call.payload::<File>().and_then(|payload| {
            if !payload.path.is_file() {
                return Err(format!(
                    "There is no file {} to share.",
                    payload.path.display()
                ));
            }
            let path = super::dialog::utf8(payload.path)?;
            let window = call.target_window(None)?;
            let options = ShareFileOptions {
                mime_type: payload.mime_type,
                title: payload.title,
                ..ShareFileOptions::default()
            };
            open(call.app, window, move |app, window| {
                app.share().share_file(window, path, options)
            })
        }),
        _ => return None,
    })
}

/// Opens the sheet without waiting for it: the plugin blocks until the user
/// closes it.
fn open(
    app: &AppHandle,
    window: WebviewWindow,
    share: impl FnOnce(&AppHandle, WebviewWindow) -> tauri_plugin_sharekit::Result<()> + Send + 'static,
) -> Result<Value, String> {
    let app = app.clone();
    thread::Builder::new()
        .name("share".to_owned())
        .spawn(move || match share(&app, window) {
            Ok(()) => {}
            Err(Error::PluginInvoke(PluginInvokeError::InvokeRejected(response)))
                if response.message.as_deref() == Some(CANCELLED) => {}
            Err(error) => eprintln!("[share] The share sheet failed: {error}."),
        })
        .map_err(failed)?;

    Ok(json!({}))
}

//! `shell.*`: hands URLs and files over to the applications of the system.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use super::{Call, NativeError, failed};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Url {
    url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Path {
    path: String,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, NativeError>> {
    Some(match method {
        "openUrl" => call.payload::<Url>().and_then(|payload| {
            open_url(call.app, &payload.url)?;
            Ok(json!({}))
        }),
        "openPath" => call.payload::<Path>().and_then(|payload| {
            open_path(call.app, &payload.path)?;
            Ok(json!({}))
        }),
        "showInFolder" => call.payload::<Path>().and_then(|payload| {
            call.app
                .opener()
                .reveal_item_in_dir(payload.path)
                .map_err(failed)?;
            Ok(json!({}))
        }),
        #[cfg(desktop)]
        "trash" => call.payload::<Path>().and_then(|payload| {
            trash::delete(&payload.path).map_err(failed)?;
            Ok(json!({}))
        }),
        #[cfg(mobile)]
        "trash" => Err(super::unavailable(
            "Phones have no trash: delete the file instead.",
        )),
        _ => return None,
    })
}

/// Opens `url` with the application of the system for it: the browser, the
/// mail client.
pub(crate) fn open_url(app: &AppHandle, url: &str) -> Result<(), NativeError> {
    #[cfg(target_os = "linux")]
    if crate::sets_no_at_bridge() {
        return open_without_at_bridge(url.as_ref());
    }

    app.opener().open_url(url, None::<&str>).map_err(failed)
}

/// Opens the file or the directory `path` with the application of the
/// system for it.
fn open_path(app: &AppHandle, path: &str) -> Result<(), NativeError> {
    #[cfg(target_os = "linux")]
    if crate::sets_no_at_bridge() {
        // As the opener has it: a path that does not exist is an error.
        std::fs::metadata(path).map_err(failed)?;
        return open_without_at_bridge(path.as_ref());
    }

    app.opener().open_path(path, None::<&str>).map_err(failed)
}

/// Opens `target` as the opener does, with `xdg-open` or what else the
/// desktop has, but without the `NO_AT_BRIDGE` of Nativella: a browser
/// started with it would stay out of reach of screen readers.
#[cfg(target_os = "linux")]
fn open_without_at_bridge(target: &std::ffi::OsStr) -> Result<(), NativeError> {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    let mut failure = String::new();
    for mut command in open::commands(target) {
        command
            .env_remove("NO_AT_BRIDGE")
            .env_remove(crate::OWN_NO_AT_BRIDGE)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            // Out of reach of a Ctrl+C in the terminal of the application.
            .process_group(0);
        match command.spawn() {
            Ok(mut child) => {
                // The browser it starts may run for long: reaped once done.
                let reaper = std::thread::Builder::new()
                    .name("opener".to_owned())
                    .spawn(move || child.wait());
                if let Err(error) = reaper {
                    eprintln!("[shell] The opener stays a zombie once done: {error}.");
                }

                return Ok(());
            }
            Err(error) => failure = error.to_string(),
        }
    }

    Err(failed(format!(
        "Cannot open {}: {failure}.",
        target.to_string_lossy()
    )))
}

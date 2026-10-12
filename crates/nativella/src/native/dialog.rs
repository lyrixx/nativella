//! `dialog.*`: message boxes and file pickers, modal to the window that asked.

use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder, MessageDialogButtons, MessageDialogKind};

use super::{Call, failed};

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
enum Kind {
    #[default]
    Info,
    Warning,
    Error,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Message {
    message: String,
    title: Option<String>,
    #[serde(default)]
    kind: Kind,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Confirm {
    message: String,
    title: Option<String>,
    #[serde(default)]
    kind: Kind,
    ok_label: Option<String>,
    cancel_label: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Filter {
    name: String,
    extensions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Files {
    title: Option<String>,
    directory: Option<PathBuf>,
    file_name: Option<String>,
    #[serde(default)]
    filters: Vec<Filter>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "message" => call.payload::<Message>().map(|payload| {
            let mut dialog = call
                .app
                .dialog()
                .message(payload.message)
                .kind(kind(&payload.kind))
                .buttons(MessageDialogButtons::Ok);
            if let Some(title) = payload.title {
                dialog = dialog.title(title);
            }
            #[cfg(desktop)]
            if let Ok(window) = call.target_window(None) {
                dialog = dialog.parent(&window);
            }
            dialog.blocking_show();

            json!({})
        }),
        "confirm" => call.payload::<Confirm>().map(|payload| {
            let buttons = match (payload.ok_label, payload.cancel_label) {
                (None, None) => MessageDialogButtons::OkCancel,
                (ok, cancel) => MessageDialogButtons::OkCancelCustom(
                    ok.unwrap_or_else(|| "OK".to_owned()),
                    cancel.unwrap_or_else(|| "Cancel".to_owned()),
                ),
            };
            let mut dialog = call
                .app
                .dialog()
                .message(payload.message)
                .kind(kind(&payload.kind))
                .buttons(buttons);
            if let Some(title) = payload.title {
                dialog = dialog.title(title);
            }
            #[cfg(desktop)]
            if let Ok(window) = call.target_window(None) {
                dialog = dialog.parent(&window);
            }

            json!({ "confirmed": dialog.blocking_show() })
        }),
        "ask" => call.payload::<Message>().map(|payload| {
            let mut dialog = call
                .app
                .dialog()
                .message(payload.message)
                .kind(kind(&payload.kind))
                .buttons(MessageDialogButtons::YesNo);
            if let Some(title) = payload.title {
                dialog = dialog.title(title);
            }
            #[cfg(desktop)]
            if let Ok(window) = call.target_window(None) {
                dialog = dialog.parent(&window);
            }

            json!({ "answer": dialog.blocking_show() })
        }),
        "openFile" => file_dialog(call).and_then(|dialog| {
            let path = dialog
                .blocking_pick_file()
                .map(|path| picked(call, path))
                .transpose()?;
            Ok(json!({ "path": path }))
        }),
        "openFiles" => file_dialog(call).and_then(|dialog| {
            let paths = dialog
                .blocking_pick_files()
                .unwrap_or_default()
                .into_iter()
                .map(|path| picked(call, path))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({ "paths": paths }))
        }),
        #[cfg(desktop)]
        "openDirectory" => file_dialog(call).and_then(|dialog| {
            let path = dialog.blocking_pick_folder().map(into_path).transpose()?;
            Ok(json!({ "path": path }))
        }),
        #[cfg(mobile)]
        "openDirectory" => Err("Picking a directory is not available on mobile.".to_owned()),
        #[cfg(desktop)]
        "saveFile" => file_dialog(call).and_then(|dialog| {
            let path = dialog.blocking_save_file().map(into_path).transpose()?;
            Ok(json!({ "path": path }))
        }),
        // Phones answer a content URI, which PHP cannot write to.
        #[cfg(mobile)]
        "saveFile" => {
            Err("Saving a file through a dialog is not available on mobile yet.".to_owned())
        }
        _ => return None,
    })
}

fn file_dialog(call: &Call<'_>) -> Result<FileDialogBuilder<tauri::Wry>, String> {
    let payload: Files = call.payload()?;

    let mut dialog = call.app.dialog().file();
    if let Some(title) = payload.title {
        dialog = dialog.set_title(title);
    }
    if let Some(directory) = payload.directory {
        dialog = dialog.set_directory(directory);
    }
    if let Some(file_name) = payload.file_name {
        dialog = dialog.set_file_name(file_name);
    }
    for filter in &payload.filters {
        let extensions: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
        dialog = dialog.add_filter(&filter.name, &extensions);
    }
    #[cfg(desktop)]
    if let Ok(window) = call.target_window(None) {
        dialog = dialog.set_parent(&window);
    }

    Ok(dialog)
}

/// A file the user picked, as a path PHP can read.
#[cfg(desktop)]
fn picked(_call: &Call<'_>, path: tauri_plugin_dialog::FilePath) -> Result<String, String> {
    into_path(path)
}

/// Phones answer a `content://` URI, which only the system can read: the
/// file is copied to the cache directory of the application first.
#[cfg(mobile)]
fn picked(call: &Call<'_>, path: tauri_plugin_dialog::FilePath) -> Result<String, String> {
    use tauri::Manager;
    use tauri_plugin_fs::FsExt;

    let tauri_plugin_dialog::FilePath::Url(url) = &path else {
        return into_path(path);
    };

    let bytes = call
        .app
        .fs()
        .read(path.clone())
        .map_err(|error| format!("Cannot read the picked file {url}: {error}."))?;
    // "…/document/primary%3ADownload%2Fnotes.md": the name is at the end.
    let decoded = percent_encoding::percent_decode_str(url.path()).decode_utf8_lossy();
    let name = std::path::Path::new(decoded.as_ref())
        .file_name()
        .map_or_else(|| "file".into(), |name| name.to_owned());

    let dir = call
        .app
        .path()
        .app_cache_dir()
        .map_err(failed)?
        .join("picked")
        .join(format!(
            "{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        ));
    std::fs::create_dir_all(&dir).map_err(failed)?;
    let target = dir.join(name);
    std::fs::write(&target, bytes).map_err(failed)?;

    utf8(target)
}

fn into_path(path: tauri_plugin_dialog::FilePath) -> Result<String, String> {
    utf8(path.into_path().map_err(failed)?)
}

/// PHP gets paths as strings: a file name that is not UTF-8 (Latin-1, on an
/// old Linux file system) is an error rather than a path PHP cannot open.
pub(crate) fn utf8(path: PathBuf) -> Result<String, String> {
    path.into_os_string().into_string().map_err(|path| {
        format!(
            "The path {} is not valid UTF-8: rename the file.",
            path.to_string_lossy()
        )
    })
}

fn kind(kind: &Kind) -> MessageDialogKind {
    match kind {
        Kind::Info => MessageDialogKind::Info,
        Kind::Warning => MessageDialogKind::Warning,
        Kind::Error => MessageDialogKind::Error,
    }
}

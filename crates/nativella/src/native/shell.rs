//! `shell.*`: hands URLs and files over to the applications of the system.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri_plugin_opener::OpenerExt;

use super::{Call, failed};

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

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    let opener = call.app.opener();

    Some(match method {
        "openUrl" => call.payload::<Url>().and_then(|payload| {
            opener.open_url(payload.url, None::<&str>).map_err(failed)?;
            Ok(json!({}))
        }),
        "openPath" => call.payload::<Path>().and_then(|payload| {
            opener
                .open_path(payload.path, None::<&str>)
                .map_err(failed)?;
            Ok(json!({}))
        }),
        "showInFolder" => call.payload::<Path>().and_then(|payload| {
            opener.reveal_item_in_dir(payload.path).map_err(failed)?;
            Ok(json!({}))
        }),
        #[cfg(desktop)]
        "trash" => call.payload::<Path>().and_then(|payload| {
            trash::delete(&payload.path).map_err(failed)?;
            Ok(json!({}))
        }),
        #[cfg(mobile)]
        "trash" => Err("Phones have no trash: delete the file instead.".to_owned()),
        _ => return None,
    })
}

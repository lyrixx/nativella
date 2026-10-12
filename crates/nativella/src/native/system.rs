//! `system.*`: the operating system and its preferences.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::Theme;

use super::{Call, failed};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    window: Option<String>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "info" => Ok(json!({
            "os": tauri_plugin_os::platform(),
            "family": tauri_plugin_os::family(),
            "arch": tauri_plugin_os::arch(),
            "version": tauri_plugin_os::version().to_string(),
            "hostname": tauri_plugin_os::hostname(),
            "locale": tauri_plugin_os::locale(),
        })),
        "theme" => call.payload::<Target>().and_then(|payload| {
            let window = call.target_window(payload.window.as_deref())?;
            let theme = window.theme().map_err(failed)?;
            Ok(json!({ "theme": theme_name(theme) }))
        }),
        _ => return None,
    })
}

pub(crate) fn theme_name(theme: Theme) -> &'static str {
    match theme {
        Theme::Dark => "dark",
        _ => "light",
    }
}

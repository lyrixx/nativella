//! `system.*`: the operating system and its preferences.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{Manager, Theme};

use super::{Call, NativeError, failed};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    window: Option<String>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, NativeError>> {
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
            // Before the first window, at boot, or in the tray.
            if payload.window.is_none() && call.app.webview_windows().is_empty() {
                return Ok(json!({ "theme": theme_name(theme_of_the_system()) }));
            }
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

/// The theme the desktop asks its applications for, as the windows read it
/// (tao): the `color-scheme` of the settings portal, light when there is
/// none.
#[cfg(target_os = "linux")]
fn theme_of_the_system() -> Theme {
    use zbus::zvariant::Value;

    /// "Prefer dark appearance", of org.freedesktop.appearance.
    const PREFER_DARK: u32 = 1;

    let color_scheme = || -> zbus::Result<bool> {
        let reply = zbus::blocking::Connection::session()?.call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.portal.Settings"),
            "Read",
            &("org.freedesktop.appearance", "color-scheme"),
        )?;
        let body = reply.body();
        // A variant in a variant.
        let mut value: Value<'_> = body.deserialize()?;
        while let Value::Value(inner) = value {
            value = *inner;
        }

        Ok(value == Value::U32(PREFER_DARK))
    };

    match color_scheme() {
        Ok(true) => Theme::Dark,
        Ok(false) => Theme::Light,
        Err(error) => {
            eprintln!("[system] The desktop does not tell its theme, taken as light: {error}.");
            Theme::Light
        }
    }
}

/// Only a window tells the theme there: light until the application has one.
#[cfg(not(target_os = "linux"))]
fn theme_of_the_system() -> Theme {
    Theme::Light
}

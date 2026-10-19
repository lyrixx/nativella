//! `app.*`: the application itself: its identity, its directories, its
//! lifecycle.

use nativella_runtime::PhpRuntime;
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::Manager;

use super::{Call, failed};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum Directory {
    AppCache,
    AppConfig,
    AppData,
    AppLog,
    Desktop,
    Documents,
    Downloads,
    Home,
    Music,
    Pictures,
    Temp,
    Videos,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Path {
    directory: Directory,
}

#[cfg(desktop)]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetBadgeCount {
    window: Option<String>,
    count: Option<i64>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    let app = call.app;

    Some(match method {
        "info" => {
            let package = app.package_info();
            Ok(json!({
                "name": package.name,
                "version": package.version.to_string(),
                "identifier": app.config().identifier,
                "nativellaVersion": env!("CARGO_PKG_VERSION"),
            }))
        }
        "path" => call.payload::<Path>().and_then(|payload| {
            let paths = app.path();
            let path = match payload.directory {
                Directory::AppCache => paths.app_cache_dir(),
                Directory::AppConfig => paths.app_config_dir(),
                // NATIVELLA_DATA_DIR may move it.
                Directory::AppData => Ok(call.state().data_dir().clone()),
                Directory::AppLog => paths.app_log_dir(),
                #[cfg(desktop)]
                Directory::Desktop => paths.desktop_dir(),
                #[cfg(mobile)]
                Directory::Desktop => Err(tauri::Error::Io(std::io::Error::other(
                    "Phones have no desktop directory.",
                ))),
                Directory::Documents => paths.document_dir(),
                Directory::Downloads => paths.download_dir(),
                Directory::Home => paths.home_dir(),
                Directory::Music => paths.audio_dir(),
                Directory::Pictures => paths.picture_dir(),
                Directory::Temp => paths.temp_dir(),
                Directory::Videos => paths.video_dir(),
            }
            .map_err(failed)?;

            Ok(json!({ "path": path }))
        }),
        // Once the current request is answered, and the ones queued before:
        // PHP still has to flush the database or save the session.
        "quit" => {
            let handle = app.clone();
            app.state::<PhpRuntime>()
                .after_queued(move || handle.exit(0));
            Ok(json!({}))
        }
        "restart" => {
            let handle = app.clone();
            app.state::<PhpRuntime>()
                .after_queued(move || handle.request_restart());
            Ok(json!({}))
        }
        // Null in a development build, which has none.
        "thirdPartyNotices" => crate::notices::text()
            .map(|text| json!({ "text": text }))
            .map_err(failed),
        "hide" => hide(app).map(|()| json!({})),
        "show" => show(app).map(|()| json!({})),
        #[cfg(desktop)]
        "setBadgeCount" => call.payload::<SetBadgeCount>().and_then(|payload| {
            call.target_window(payload.window.as_deref())?
                .set_badge_count(payload.count)
                .map_err(failed)?;
            Ok(json!({}))
        }),
        #[cfg(mobile)]
        "setBadgeCount" => Err("Badges are not available on mobile yet.".to_owned()),
        "enableAutostart" => autostart::enable(app).map(|()| json!({})),
        "disableAutostart" => autostart::disable(app).map(|()| json!({})),
        "isAutostartEnabled" => {
            autostart::is_enabled(app).map(|enabled| json!({ "enabled": enabled }))
        }
        _ => return None,
    })
}

#[cfg(target_os = "macos")]
fn hide(app: &tauri::AppHandle) -> Result<(), String> {
    app.hide().map_err(failed)
}

#[cfg(not(target_os = "macos"))]
fn hide(_app: &tauri::AppHandle) -> Result<(), String> {
    Err("Hiding the application is only possible on macOS: hide its windows instead.".to_owned())
}

#[cfg(target_os = "macos")]
fn show(app: &tauri::AppHandle) -> Result<(), String> {
    app.show().map_err(failed)
}

#[cfg(not(target_os = "macos"))]
fn show(_app: &tauri::AppHandle) -> Result<(), String> {
    Err("Showing the application is only possible on macOS: show its windows instead.".to_owned())
}

/// Starting with the user session only exists on desktop.
#[cfg(desktop)]
mod autostart {
    use tauri::AppHandle;
    use tauri_plugin_autostart::ManagerExt;

    use super::failed;

    pub(super) fn enable(app: &AppHandle) -> Result<(), String> {
        app.autolaunch().enable().map_err(failed)
    }

    pub(super) fn disable(app: &AppHandle) -> Result<(), String> {
        app.autolaunch().disable().map_err(failed)
    }

    pub(super) fn is_enabled(app: &AppHandle) -> Result<bool, String> {
        app.autolaunch().is_enabled().map_err(failed)
    }
}

#[cfg(mobile)]
mod autostart {
    use tauri::AppHandle;

    const UNAVAILABLE: &str = "Autostart is not available on mobile.";

    pub(super) fn enable(_app: &AppHandle) -> Result<(), String> {
        Err(UNAVAILABLE.to_owned())
    }

    pub(super) fn disable(_app: &AppHandle) -> Result<(), String> {
        Err(UNAVAILABLE.to_owned())
    }

    pub(super) fn is_enabled(_app: &AppHandle) -> Result<bool, String> {
        Ok(false)
    }
}

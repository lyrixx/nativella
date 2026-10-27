//! `tray.*`: an icon in the system tray (the menu bar on macOS), with its
//! menu. While it exists, closing the last window keeps the application
//! running.

use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use super::menu::{self, Item};
use super::{Call, failed, lock};
use crate::events;

/// The application has one tray icon at most.
const TRAY: &str = "main";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Set {
    tooltip: Option<String>,
    /// Next to the icon, on macOS and Linux.
    title: Option<String>,
    /// A PNG file; the application icon when absent.
    icon: Option<PathBuf>,
    #[serde(default)]
    menu: Vec<Item>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "set" => call.payload::<Set>().and_then(|payload| set(call, payload)),
        "remove" => {
            // Dropped once the lock is released, as Tauri calls go.
            let tray = lock(&call.state().tray).take();
            drop(tray);
            *lock(&call.state().tray_menu) = None;
            call.app.remove_tray_by_id(TRAY);
            Ok(json!({}))
        }
        _ => return None,
    })
}

fn set(call: &Call<'_>, payload: Set) -> Result<Value, String> {
    let app = call.app;
    let icon = match &payload.icon {
        Some(path) => Image::from_path(path)
            .map_err(|error| format!("Cannot load the tray icon {}: {error}.", path.display()))?,
        None => app
            .default_window_icon()
            .cloned()
            .ok_or("The application has no icon: give the tray one.")?,
    };
    let menu = if payload.menu.is_empty() {
        None
    } else {
        Some(menu::build(call, &payload.menu)?)
    };

    // Tauri runs tray calls on the main thread and waits for them, while the
    // main thread may wait for this lock (`has_tray()` when the last window
    // closes): never hold it across them.
    let state = call.state();
    lock(&state.tray_menu).clone_from(&menu);
    let existing = lock(&state.tray).clone();
    if let Some(existing) = existing {
        existing.set_icon(Some(icon)).map_err(failed)?;
        existing.set_tooltip(payload.tooltip).map_err(failed)?;
        existing.set_title(payload.title).map_err(failed)?;
        existing.set_menu(menu).map_err(failed)?;

        return Ok(json!({}));
    }

    let mut builder = TrayIconBuilder::with_id(TRAY)
        .icon(icon)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            // Linux reports no click: only the menu works there.
            if let TrayIconEvent::Click {
                button,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let button = match button {
                    MouseButton::Left => "left",
                    MouseButton::Right => "right",
                    MouseButton::Middle => "middle",
                };
                events::send(
                    tray.app_handle(),
                    "tray.clicked",
                    json!({ "button": button }),
                );
            }
        });
    if let Some(tooltip) = payload.tooltip {
        builder = builder.tooltip(tooltip);
    }
    if let Some(title) = payload.title {
        builder = builder.title(title);
    }
    if let Some(menu) = menu {
        builder = builder.menu(&menu);
    }

    let tray = builder.build(app).map_err(failed)?;
    *lock(&state.tray) = Some(tray);

    Ok(json!({}))
}

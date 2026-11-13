//! `tray.*`: an icon in the system tray (the menu bar on macOS), with its
//! menu. While it exists, closing the last window keeps the application
//! running.
//!
//! The icon may have a window of its own, as the applications of the menu bar
//! do: a click on the icon shows it next to the icon, a second one, or a
//! click elsewhere, hides it. Linux reports no click, nor where the icon is:
//! PHP shows it from the menu, with `tray.showWindow`, in the corner of the
//! screen.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition};

use super::menu::{self, Item, Place};
use super::{Call, NativeError, failed, lock, not_found};
use crate::events;

/// A click on the icon makes the window lose the focus first, which hides
/// it: that click does not show it again. Not on macOS, where the window
/// keeps the focus: Windows may need it, untried.
const BLUR_CLICK: Duration = Duration::from_millis(300);

/// The window of the icon.
pub(crate) struct TrayWindow {
    label: String,
    /// When it hid because it lost the focus.
    blurred: Option<Instant>,
}

/// A rectangle of the screen, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Area {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

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
    /// The label of the window of the icon.
    window: Option<String>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, NativeError>> {
    Some(match method {
        "set" => call.payload::<Set>().and_then(|payload| set(call, payload)),
        "remove" => {
            // Dropped once the lock is released, as Tauri calls go.
            let tray = lock(&call.state().tray).take();
            drop(tray);
            *lock(&call.state().tray_menu) = None;
            call.state().menus.remove(Place::Tray);
            call.app.remove_tray_by_id(TRAY);
            *lock(&call.state().tray_window) = None;
            // The tray may have been all the application showed.
            crate::quit_unless_shown(call.app);
            Ok(json!({}))
        }
        "showWindow" => {
            let label = lock(&call.state().tray_window)
                .as_ref()
                .map(|tray_window| tray_window.label.clone());
            match label {
                Some(label) => show(call.app, &label, icon_rect(call.app)).map(|()| json!({})),
                None => Err(not_found(
                    "The tray icon has no window: set it with Tray::set().",
                )),
            }
        }
        _ => return None,
    })
}

fn set(call: &Call<'_>, payload: Set) -> Result<Value, NativeError> {
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
        call.state().menus.remove(Place::Tray);
        None
    } else {
        Some(menu::build(call, &payload.menu, Place::Tray)?)
    };

    // Tauri runs tray calls on the main thread and waits for them, while the
    // main thread may wait for this lock (`has_tray()` when the last window
    // closes): never hold it across them.
    let state = call.state();
    lock(&state.tray_menu).clone_from(&menu);
    *lock(&state.tray_window) = payload.window.map(|label| TrayWindow {
        label,
        blurred: None,
    });
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
                rect,
                ..
            } = event
            {
                if button == MouseButton::Left {
                    clicked(tray.app_handle(), rect);
                }
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

/// A left click on the icon at `rect`: shows its window, or hides it.
fn clicked(app: &AppHandle, rect: tauri::Rect) {
    let state = app.state::<super::State>();
    let label = {
        let mut tray_window = lock(&state.tray_window);
        let Some(tray_window) = tray_window.as_mut() else {
            return;
        };
        // The click that took the focus away from it.
        if tray_window
            .blurred
            .take()
            .is_some_and(|blurred| blurred.elapsed() < BLUR_CLICK)
        {
            return;
        }
        tray_window.label.clone()
    };

    let visible = app
        .get_webview_window(&label)
        .is_some_and(|window| window.is_visible().unwrap_or(false));
    let result = if visible {
        app.get_webview_window(&label)
            .map_or(Ok(()), |window| window.hide().map_err(failed))
    } else {
        show(app, &label, Some(rect))
    };
    if let Err(error) = result {
        eprintln!("[tray] The window of the icon failed: {error}");
    }
}

/// Where the icon is: macOS and Windows know, Linux does not.
fn icon_rect(app: &AppHandle) -> Option<tauri::Rect> {
    // Tauri asks the main thread, which may wait for this lock.
    let tray = lock(&app.state::<super::State>().tray).clone()?;
    tray.rect().unwrap_or_else(|error| {
        eprintln!("[tray] The position of the icon is unknown: {error}.");
        None
    })
}

/// Shows the window `label` next to the icon at `icon`, or in the corner of
/// the screen where the trays are when the icon is nowhere to be known, and
/// focuses it.
fn show(app: &AppHandle, label: &str, icon: Option<tauri::Rect>) -> Result<(), NativeError> {
    let window = app.get_webview_window(label).ok_or_else(|| {
        not_found(format!(
            "There is no \"{label}\" window: open it, hidden, first."
        ))
    })?;
    let scale = window.scale_factor().map_err(failed)?;
    let icon = icon.map(|rect| {
        let position = rect.position.to_physical::<f64>(scale);
        let size = rect.size.to_physical::<f64>(scale);
        Area {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        }
    });

    let monitor = match icon {
        Some(icon) => window.monitor_from_point(icon.x, icon.y).map_err(failed)?,
        None => window.primary_monitor().map_err(failed)?,
    }
    .or(window.current_monitor().map_err(failed)?);
    if let Some(monitor) = monitor {
        let work_area = monitor.work_area();
        let size = window.outer_size().map_err(failed)?;
        let (x, y) = anchor(
            icon,
            (f64::from(size.width), f64::from(size.height)),
            Area {
                x: f64::from(work_area.position.x),
                y: f64::from(work_area.position.y),
                width: f64::from(work_area.size.width),
                height: f64::from(work_area.size.height),
            },
        );
        window
            .set_position(PhysicalPosition::new(x, y))
            .map_err(failed)?;
    }
    window.show().map_err(failed)?;
    window.set_focus().map_err(failed)
}

/// Hides the window of the icon when `label` is it: it lost the focus.
pub(crate) fn blurred(app: &AppHandle, label: &str) {
    let state = app.state::<super::State>();
    // Its own dialog took the focus: hidden, the window would hide the
    // dialog, and PHP would wait for its answer forever.
    if lock(&state.modal_dialogs)
        .iter()
        .any(|parent| parent == label)
    {
        return;
    }
    {
        let mut tray_window = lock(&state.tray_window);
        match tray_window.as_mut() {
            Some(tray_window) if tray_window.label == label => {
                tray_window.blurred = Some(Instant::now());
            }
            _ => return,
        }
    }
    if let Some(window) = app.get_webview_window(label)
        && let Err(error) = window.hide()
    {
        eprintln!("[tray] Could not hide the window of the icon: {error}.");
    }
}

/// Where a window of `size` goes: centered under the icon when it is at the
/// top of the screen, as the menu bar of macOS, over it at the bottom, as the
/// taskbar of Windows, in the top right corner when the icon is unknown;
/// always within `work_area`.
fn anchor(icon: Option<Area>, (width, height): (f64, f64), work_area: Area) -> (f64, f64) {
    let (x, y) = match icon {
        Some(icon) => {
            let x = icon.x + icon.width / 2.0 - width / 2.0;
            let at_top = icon.y + icon.height / 2.0 < work_area.y + work_area.height / 2.0;
            let y = if at_top {
                icon.y + icon.height
            } else {
                icon.y - height
            };
            (x, y)
        }
        None => (work_area.x + work_area.width - width, work_area.y),
    };

    (
        x.clamp(
            work_area.x,
            (work_area.x + work_area.width - width).max(work_area.x),
        ),
        y.clamp(
            work_area.y,
            (work_area.y + work_area.height - height).max(work_area.y),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Area = Area {
        x: 0.0,
        y: 50.0,
        width: 1920.0,
        height: 1000.0,
    };

    #[test]
    fn under_an_icon_of_the_menu_bar() {
        let icon = Area {
            x: 1500.0,
            y: 10.0,
            width: 40.0,
            height: 40.0,
        };

        assert_eq!(anchor(Some(icon), (400.0, 300.0), SCREEN), (1320.0, 50.0));
    }

    #[test]
    fn over_an_icon_of_the_taskbar() {
        let icon = Area {
            x: 1800.0,
            y: 1060.0,
            width: 40.0,
            height: 40.0,
        };

        // Within the screen: the icon is near its right edge.
        assert_eq!(anchor(Some(icon), (400.0, 300.0), SCREEN), (1520.0, 750.0));
    }

    #[test]
    fn in_the_corner_without_an_icon() {
        assert_eq!(anchor(None, (400.0, 300.0), SCREEN), (1520.0, 50.0));
    }
}

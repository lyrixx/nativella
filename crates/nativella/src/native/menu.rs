//! `menu.*`: the application menu and context menus. A click reaches PHP as
//! `menu.itemClicked`, and navigates when the item has a URL.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::menu::{
    AboutMetadata, CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem,
    Submenu,
};
use tauri::{AppHandle, Manager, Wry};

use super::{Call, State, failed, lock, window};
use crate::events;

/// A menu entry, as PHP describes it.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Item {
    #[serde(rename_all = "camelCase")]
    Action {
        id: String,
        label: String,
        accelerator: Option<String>,
        #[serde(default = "enabled")]
        enabled: bool,
        /// A path of the app, opened in the focused window on click.
        url: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Checkbox {
        id: String,
        label: String,
        #[serde(default)]
        checked: bool,
        accelerator: Option<String>,
        #[serde(default = "enabled")]
        enabled: bool,
    },
    Separator,
    #[serde(rename_all = "camelCase")]
    Submenu {
        label: String,
        items: Vec<Item>,
        #[serde(default = "enabled")]
        enabled: bool,
    },
    /// An entry the system implements: copy, quit…
    Role {
        role: Role,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Role {
    About,
    CloseWindow,
    Copy,
    Cut,
    Fullscreen,
    Hide,
    HideOthers,
    Maximize,
    Minimize,
    Paste,
    Quit,
    Redo,
    SelectAll,
    Services,
    ShowAll,
    Undo,
}

fn enabled() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Set {
    items: Vec<Item>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Popup {
    items: Vec<Item>,
    window: Option<String>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "set" => call.payload::<Set>().and_then(|payload| {
            let menu = build(call, &payload.items)?;
            call.app.set_menu(menu).map_err(failed)?;
            Ok(json!({}))
        }),
        "remove" => call.app.remove_menu().map(|_| json!({})).map_err(failed),
        "popup" => call.payload::<Popup>().and_then(|payload| {
            let window = call.target_window(payload.window.as_deref())?;
            let menu = build(call, &payload.items)?;
            window.popup_menu(&menu).map_err(failed)?;
            Ok(json!({}))
        }),
        _ => return None,
    })
}

/// Builds a Tauri menu, remembering the links and checkboxes of its items.
pub(crate) fn build(call: &Call<'_>, items: &[Item]) -> Result<Menu<Wry>, String> {
    let built = build_items(call.app, &call.state(), items)?;
    let references: Vec<&dyn IsMenuItem<Wry>> = built.iter().map(AsRef::as_ref).collect();

    Menu::with_items(call.app, &references).map_err(failed)
}

fn build_items(
    app: &AppHandle,
    state: &State,
    items: &[Item],
) -> Result<Vec<Box<dyn IsMenuItem<Wry>>>, String> {
    items
        .iter()
        .map(|item| -> Result<Option<Box<dyn IsMenuItem<Wry>>>, String> {
            Ok(Some(match item {
                Item::Action {
                    id,
                    label,
                    accelerator,
                    enabled,
                    url,
                } => {
                    // Not held across the Tauri call below, which waits for
                    // the main thread, where clicks lock it too.
                    {
                        let mut links = lock(&state.menu_links);
                        match url {
                            Some(url) => links.insert(id.clone(), url.clone()),
                            None => links.remove(id),
                        };
                    }

                    Box::new(
                        MenuItem::with_id(app, id, label, *enabled, accelerator.as_deref())
                            .map_err(failed)?,
                    )
                }
                Item::Checkbox {
                    id,
                    label,
                    checked,
                    accelerator,
                    enabled,
                } => {
                    let checkbox = CheckMenuItem::with_id(
                        app,
                        id,
                        label,
                        *enabled,
                        *checked,
                        accelerator.as_deref(),
                    )
                    .map_err(failed)?;
                    lock(&state.checkboxes).insert(id.clone(), checkbox.clone());

                    Box::new(checkbox)
                }
                Item::Separator => Box::new(PredefinedMenuItem::separator(app).map_err(failed)?),
                Item::Submenu {
                    label,
                    items,
                    enabled,
                } => {
                    let children = build_items(app, state, items)?;
                    let references: Vec<&dyn IsMenuItem<Wry>> =
                        children.iter().map(AsRef::as_ref).collect();

                    Box::new(
                        Submenu::with_items(app, label, *enabled, &references).map_err(failed)?,
                    )
                }
                Item::Role { role } => match role_item(app, *role).map_err(failed)? {
                    Some(item) => item,
                    None => return Ok(None),
                },
            }))
        })
        .filter_map(Result::transpose)
        .collect()
}

/// The entry of a system role, `None` when the platform has no such thing.
#[cfg(not(target_os = "linux"))]
fn role_item(app: &AppHandle, role: Role) -> tauri::Result<Option<Box<dyn IsMenuItem<Wry>>>> {
    Ok(Some(Box::new(predefined(app, role)?)))
}

/// GTK implements few roles (muda greys the others out), and its clipboard
/// ones need X11: Nativella implements them, see `linux_roles`.
#[cfg(target_os = "linux")]
fn role_item(app: &AppHandle, role: Role) -> tauri::Result<Option<Box<dyn IsMenuItem<Wry>>>> {
    if role == Role::About {
        return Ok(Some(Box::new(predefined(app, role)?)));
    }

    linux_roles::item(app, role)
}

fn predefined(app: &AppHandle, role: Role) -> tauri::Result<PredefinedMenuItem<Wry>> {
    match role {
        // Linux shows nothing without the metadata.
        Role::About => {
            let package = app.package_info();
            let metadata = AboutMetadata {
                name: Some(package.name.clone()),
                version: Some(package.version.to_string()),
                icon: app.default_window_icon().cloned(),
                ..AboutMetadata::default()
            };
            PredefinedMenuItem::about(app, None, Some(metadata))
        }
        Role::CloseWindow => PredefinedMenuItem::close_window(app, None),
        Role::Copy => PredefinedMenuItem::copy(app, None),
        Role::Cut => PredefinedMenuItem::cut(app, None),
        Role::Fullscreen => PredefinedMenuItem::fullscreen(app, None),
        Role::Hide => PredefinedMenuItem::hide(app, None),
        Role::HideOthers => PredefinedMenuItem::hide_others(app, None),
        Role::Maximize => PredefinedMenuItem::maximize(app, None),
        Role::Minimize => PredefinedMenuItem::minimize(app, None),
        Role::Paste => PredefinedMenuItem::paste(app, None),
        Role::Quit => PredefinedMenuItem::quit(app, None),
        Role::Redo => PredefinedMenuItem::redo(app, None),
        Role::SelectAll => PredefinedMenuItem::select_all(app, None),
        Role::Services => PredefinedMenuItem::services(app, None),
        Role::ShowAll => PredefinedMenuItem::show_all(app, None),
        Role::Undo => PredefinedMenuItem::undo(app, None),
    }
}

pub(crate) fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref();
    let state = app.state::<State>();

    #[cfg(target_os = "linux")]
    if linux_roles::handle(app, id) {
        return;
    }

    if let Some(url) = lock(&state.menu_links).get(id).cloned() {
        let windows = app.webview_windows();
        let target = windows
            .values()
            .find(|window| window.is_focused().unwrap_or(false))
            .or_else(|| windows.values().next());

        match (target, window::url(&url)) {
            (Some(target), Ok(url)) => {
                if let Err(error) = target.navigate(url.clone()) {
                    eprintln!("[menu] Could not open {url}: {error}.");
                }
            }
            (None, _) => eprintln!("[menu] No window can open {url}."),
            (_, Err(error)) => eprintln!("[menu] {error}"),
        }
    }

    let checkbox = lock(&state.checkboxes).get(id).cloned();
    let checked = checkbox.and_then(|checkbox| checkbox.is_checked().ok());

    events::send(
        app,
        "menu.itemClicked",
        json!({ "id": id, "checked": checked }),
    );
}

/// The system roles GTK lacks, as regular entries with reserved ids. Edit
/// roles act on the focused page; their shortcuts stay the webview's own.
#[cfg(target_os = "linux")]
mod linux_roles {
    use serde_json::Value;
    use tauri::menu::{IsMenuItem, MenuItem};
    use tauri::{AppHandle, Manager, WebviewWindow, Wry};
    use tauri_plugin_clipboard_manager::ClipboardExt;

    use super::Role;

    const PREFIX: &str = "nativella:role:";

    pub(super) fn item(
        app: &AppHandle,
        role: Role,
    ) -> tauri::Result<Option<Box<dyn IsMenuItem<Wry>>>> {
        let (name, label, accelerator) = match role {
            Role::CloseWindow => ("closeWindow", "Close Window", Some("CmdOrCtrl+W")),
            Role::Copy => ("copy", "Copy", None),
            Role::Cut => ("cut", "Cut", None),
            Role::Fullscreen => ("fullscreen", "Toggle Full Screen", Some("F11")),
            Role::Maximize => ("maximize", "Maximize", None),
            Role::Minimize => ("minimize", "Minimize", None),
            Role::Paste => ("paste", "Paste", None),
            Role::Quit => ("quit", "Quit", Some("CmdOrCtrl+Q")),
            Role::Redo => ("redo", "Redo", None),
            Role::SelectAll => ("selectAll", "Select All", None),
            Role::Undo => ("undo", "Undo", None),
            // macOS only.
            Role::About | Role::Hide | Role::HideOthers | Role::Services | Role::ShowAll => {
                return Ok(None);
            }
        };

        Ok(Some(Box::new(MenuItem::with_id(
            app,
            format!("{PREFIX}{name}"),
            label,
            true,
            accelerator,
        )?)))
    }

    /// Runs the role `id` names, false if it names none.
    pub(super) fn handle(app: &AppHandle, id: &str) -> bool {
        let Some(name) = id.strip_prefix(PREFIX) else {
            return false;
        };
        if name == "quit" {
            crate::before_unload::quit(app);

            return true;
        }

        let windows = app.webview_windows();
        let Some(window) = windows
            .values()
            .find(|window| window.is_focused().unwrap_or(false))
            .or_else(|| windows.values().next())
        else {
            return true;
        };

        let result = match name {
            "closeWindow" => window.close(),
            "fullscreen" => window
                .is_fullscreen()
                .and_then(|fullscreen| window.set_fullscreen(!fullscreen)),
            "maximize" => window.is_maximized().and_then(|maximized| {
                if maximized {
                    window.unmaximize()
                } else {
                    window.maximize()
                }
            }),
            "minimize" => window.minimize(),
            "paste" => paste(app, window),
            "copy" | "cut" | "redo" | "selectAll" | "undo" => {
                window.eval(format!("document.execCommand('{name}')"))
            }
            _ => Ok(()),
        };

        if let Err(error) = result {
            eprintln!("[menu] The \"{name}\" entry failed: {error}.");
        }

        true
    }

    /// Pages cannot read the clipboard: paste its text as typed.
    fn paste(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
        let Ok(text) = app.clipboard().read_text() else {
            return Ok(());
        };

        window.eval(format!(
            "document.execCommand('insertText', false, {})",
            Value::String(text)
        ))
    }
}

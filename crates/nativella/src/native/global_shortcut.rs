//! `globalShortcut.*`: keyboard shortcuts caught even when the application
//! is in the background. A press reaches PHP as `globalShortcut.pressed`.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use super::{Call, State, failed, lock};
use crate::events;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Accelerator {
    accelerator: String,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    let manager = call.app.global_shortcut();

    Some(match method {
        "register" => call.payload::<Accelerator>().and_then(|payload| {
            let shortcut = parse(&payload.accelerator)?;
            if !manager.is_registered(shortcut) {
                manager.register(shortcut).map_err(failed)?;
            }
            lock(&call.state().shortcuts).insert(shortcut.id(), payload.accelerator);
            Ok(json!({}))
        }),
        "unregister" => call.payload::<Accelerator>().and_then(|payload| {
            let shortcut = parse(&payload.accelerator)?;
            if manager.is_registered(shortcut) {
                manager.unregister(shortcut).map_err(failed)?;
            }
            lock(&call.state().shortcuts).remove(&shortcut.id());
            Ok(json!({}))
        }),
        "unregisterAll" => manager.unregister_all().map_err(failed).map(|()| {
            lock(&call.state().shortcuts).clear();
            json!({})
        }),
        "isRegistered" => call.payload::<Accelerator>().and_then(|payload| {
            let shortcut = parse(&payload.accelerator)?;
            Ok(json!({ "registered": manager.is_registered(shortcut) }))
        }),
        _ => return None,
    })
}

pub(crate) fn on_shortcut(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }

    // PHP gets the accelerator back as it wrote it.
    let accelerator = lock(&app.state::<State>().shortcuts)
        .get(&shortcut.id())
        .cloned()
        .unwrap_or_else(|| shortcut.into_string());

    events::send(
        app,
        "globalShortcut.pressed",
        json!({ "accelerator": accelerator }),
    );
}

fn parse(accelerator: &str) -> Result<Shortcut, String> {
    accelerator
        .parse()
        .map_err(|error| format!("\"{accelerator}\" is not a valid accelerator: {error}."))
}

//! `nfc.*`: reads the NFC tag held against the phone, on phones whose
//! application asks for the `nfc` permission.
//!
//! tauri-plugin-nfc reads the tags with its Kotlin and Swift classes, but its
//! Rust API expects the identifier of a tag as a string, where they answer
//! bytes: [`plugin`] registers the classes under the plugin's name, as
//! barcode_scanner.rs does, and reads their answer itself.

use std::sync::mpsc;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::{AppHandle, Manager, Wry};

use super::{Call, failed};
use crate::nfc_tags::{Tag, tag_json};

// Nothing of the crate is used by name, but its build script asks for the
// Swift library that holds the symbol below: rustc leaves out the link
// directives of a crate that nothing uses.
#[cfg(target_os = "ios")]
extern crate tauri_plugin_nfc as _;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_nfc);

/// The name of Tauri's plugin, which its Kotlin and Swift classes know it by.
const PLUGIN: &str = "nfc";

/// The user holds the tag against the phone when they want: a read nobody
/// does ends.
const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// The handle on the classes of the plugin.
pub(crate) struct Reader(PluginHandle<Wry>);

/// Whether the application asks for `nfc`: only then does lib.rs register
/// the plugin.
pub(crate) fn registered(app: &AppHandle) -> bool {
    app.try_state::<Reader>().is_some()
}

pub(crate) fn plugin() -> TauriPlugin<Wry> {
    Builder::new(PLUGIN)
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let handle = api.register_android_plugin("app.tauri.nfc", "NfcPlugin")?;
            #[cfg(target_os = "ios")]
            let handle = api.register_ios_plugin(init_plugin_nfc)?;
            app.manage(Reader(handle));

            Ok(())
        })
        .build()
}

#[derive(Deserialize)]
struct Available {
    available: bool,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "isAvailable" => available(call.app).map(|available| json!({ "available": available })),
        "read" => read(call.app),
        _ => return None,
    })
}

/// Whether the phone has NFC, turned on.
fn available(app: &AppHandle) -> Result<bool, String> {
    app.state::<Reader>()
        .0
        .run_mobile_plugin::<Available>("isAvailable", ())
        .map(|answer| answer.available)
        .map_err(failed)
}

/// Waits for a tag, any tag, and reads it.
fn read(app: &AppHandle) -> Result<Value, String> {
    if !available(app)? {
        return Err(
            "NFC is not available: the phone has none, or it is turned off in its settings."
                .to_owned(),
        );
    }

    let (sender, receiver) = mpsc::channel();
    let reading = app.clone();
    // Not a thread: the plugin answers once a tag comes, if ever.
    tauri::async_runtime::spawn(async move {
        let reader = reading.state::<Reader>();
        let options = json!({ "kind": { "tag": {} }, "keepSessionAlive": false });
        let tag = reader
            .0
            .run_mobile_plugin_async::<Tag>("scan", options)
            .await
            .map_err(failed);
        // The receiver is gone when the time is up.
        let _ = sender.send(tag);
    });

    match receiver.recv_timeout(READ_TIMEOUT) {
        Ok(tag) => Ok(json!({ "tag": tag_json(&tag?) })),
        Err(_) => Err(format!(
            "No NFC tag came near the phone within {} seconds.",
            READ_TIMEOUT.as_secs()
        )),
    }
}

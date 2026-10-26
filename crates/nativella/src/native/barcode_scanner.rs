//! `barcodeScanner.*`: reads a barcode with the camera of the phone, on
//! phones whose application asks for the `barcodeScanner` permission. The
//! user is asked on the first call.
//!
//! tauri-plugin-barcode-scanner has no Rust API: its commands are the
//! JavaScript API of a page, and the handle on its Kotlin and Swift classes
//! is private. So [`plugin`] registers those classes under the plugin's name,
//! in place of its own `init()`, and holds the handle.
//!
//! The plugin draws the camera behind the webview, in its "windowed" mode:
//! the page is made transparent, and shows a frame and a button to cancel
//! (barcode_scanner.js) while PHP waits. The button comes back through
//! [`nativella_cancel_scan`]. A page that loads meanwhile, such as the
//! stand-in Android shows when PHP takes more than 25 seconds (deadline.rs),
//! gets them too.

use std::sync::Mutex;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::plugin::mobile::PluginInvokeError;
use tauri::plugin::{Builder, PermissionState, PluginHandle, TauriPlugin};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Manager, Wry};

use super::{Call, failed, lock};

// Nothing of the crate is used by name, but its build script asks for the Swift
// library that holds the symbol below: rustc leaves out the link directives of
// a crate that nothing uses.
#[cfg(target_os = "ios")]
extern crate tauri_plugin_barcode_scanner as _;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_barcode_scanner);

/// The name of Tauri's plugin, which its Kotlin and Swift classes know it by:
/// only one of the two registers (lib.rs).
const PLUGIN: &str = "barcode-scanner";

/// The user reads the question of the system: longer than a position, but not
/// forever.
const PERMISSION_TIMEOUT: Duration = Duration::from_secs(60);

/// The user cancels when they want to. This ends a scan nobody can cancel:
/// a page that did not show its button, a phone without camera (Android's
/// plugin never answers then).
const SCAN_TIMEOUT: Duration = Duration::from_secs(120);

/// Injected in the page, with the label of its button: see the script.
const OVERLAY: &str = include_str!("barcode_scanner.js");

/// What the plugin reads: its keys, from Android and iOS alike.
#[derive(Deserialize)]
struct Scanned {
    /// Missing on iOS when the code holds no text.
    #[serde(default)]
    content: String,
    /// The name of the format: `QR_CODE`, `EAN_13`…
    format: Option<String>,
}

#[derive(Deserialize)]
struct Permissions {
    camera: PermissionState,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Scan {
    /// The names of the plugin: every format when empty.
    formats: Vec<String>,
    /// The label of the button of the page.
    cancel_label: String,
}

/// How a scan ends.
enum Outcome {
    /// What the plugin answered.
    Scanned(Result<Scanned, String>),
    /// The user cancelled: the page already told the plugin.
    Cancelled,
}

/// The scan in progress.
struct Session {
    /// Where it waits for its outcome.
    outcomes: Sender<Outcome>,
    /// The window whose page shows the camera.
    window: String,
    cancel_label: String,
}

/// The handle on the plugin, and the scan in progress: PHP only runs one at
/// a time.
pub(crate) struct Scanner {
    handle: PluginHandle<Wry>,
    session: Mutex<Option<Session>>,
}

/// Whether the application asks for `barcodeScanner`: only then does lib.rs
/// register the plugin.
pub(crate) fn registered(app: &AppHandle) -> bool {
    app.try_state::<Scanner>().is_some()
}

/// The classes of Tauri's plugin, which hold the camera and the reading.
pub(crate) fn plugin() -> TauriPlugin<Wry> {
    Builder::new(PLUGIN)
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let handle =
                api.register_android_plugin("app.tauri.barcodescanner", "BarcodeScannerPlugin")?;
            #[cfg(target_os = "ios")]
            let handle = api.register_ios_plugin(init_plugin_barcode_scanner)?;
            app.manage(Scanner {
                handle,
                session: Mutex::default(),
            });

            Ok(())
        })
        .on_page_load(|webview, payload| {
            if payload.event() != PageLoadEvent::Finished {
                return;
            }
            let scanner = webview.state::<Scanner>();
            let session = lock(&scanner.session);
            if let Some(session) = session.as_ref()
                && session.window == webview.label()
                && let Err(error) = overlay(Some(&session.cancel_label))
                    .and_then(|script| webview.eval(script).map_err(failed))
            {
                eprintln!("[barcodeScanner] Could not show the camera again: {error}.");
            }
        })
        .build()
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "scan" => call
            .payload::<Scan>()
            .and_then(|payload| scan(call, payload)),
        _ => return None,
    })
}

/// The barcode, or null when the user does not allow the camera or cancels.
fn scan(call: &Call<'_>, payload: Scan) -> Result<Value, String> {
    if !camera_allowed(call.app)? {
        return Ok(json!({ "barcode": null }));
    }

    let window = call.target_window(None)?;
    let outcome = read(call.app, window.label(), payload);
    // Whatever happened: the page is the page again. Not `window`: the page
    // that shows now may be another one.
    if let Some(window) = call.app.get_webview_window(window.label())
        && let Err(error) = overlay(None).and_then(|script| window.eval(script).map_err(failed))
    {
        eprintln!("[barcodeScanner] Could not hide the camera: {error}.");
    }

    match outcome? {
        Outcome::Scanned(scanned) => {
            let scanned = scanned?;
            Ok(json!({ "barcode": { "content": scanned.content, "format": scanned.format } }))
        }
        Outcome::Cancelled => Ok(json!({ "barcode": null })),
    }
}

/// Shows the camera in `window`, starts it, and waits for a code, the user
/// cancelling, or the end of the time.
fn read(app: &AppHandle, window: &str, payload: Scan) -> Result<Outcome, String> {
    let (sender, receiver) = mpsc::channel();
    let scanner = app.state::<Scanner>();
    *lock(&scanner.session) = Some(Session {
        outcomes: sender.clone(),
        window: window.to_owned(),
        cancel_label: payload.cancel_label.clone(),
    });
    let ended = |app: &AppHandle| lock(&app.state::<Scanner>().session).take();

    let window = app
        .get_webview_window(window)
        .ok_or_else(|| format!("There is no \"{window}\" window."))?;
    if let Err(error) =
        overlay(Some(&payload.cancel_label)).and_then(|script| window.eval(script).map_err(failed))
    {
        ended(app);
        return Err(error);
    }

    let scanning = app.clone();
    // Not a thread: Android's plugin may never answer a scan it cancelled.
    tauri::async_runtime::spawn(async move {
        let scanner = scanning.state::<Scanner>();
        let options = json!({
            "formats": payload.formats,
            "windowed": true,
            "cameraDirection": "back",
        });
        let outcome = match scanner
            .handle
            .run_mobile_plugin_async::<Scanned>("scan", options)
            .await
        {
            Ok(scanned) => Outcome::Scanned(Ok(scanned)),
            // The scan that was cancelled: iOS and recent Android reject it.
            Err(PluginInvokeError::InvokeRejected(response))
                if response.message.as_deref() == Some("cancelled") =>
            {
                Outcome::Cancelled
            }
            Err(error) => Outcome::Scanned(Err(failed(error))),
        };
        // The receiver is gone when the scan ended another way.
        let _ = sender.send(outcome);
    });

    let outcome = receiver.recv_timeout(SCAN_TIMEOUT);
    ended(app);

    match outcome {
        Ok(Outcome::Cancelled) => Ok(Outcome::Cancelled),
        // The camera may still run, when the plugin failed in the middle.
        Ok(Outcome::Scanned(Err(error))) => {
            stop(app);
            Err(error)
        }
        Ok(outcome) => Ok(outcome),
        Err(_) => {
            stop(app);
            Err(format!(
                "The phone read no barcode within {} seconds.",
                SCAN_TIMEOUT.as_secs()
            ))
        }
    }
}

/// Tells the plugin to stop the camera, without waiting for it.
fn stop(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let scanner = app.state::<Scanner>();
        if let Err(error) = scanner
            .handle
            .run_mobile_plugin_async::<Value>("cancel", ())
            .await
        {
            eprintln!("[barcodeScanner] Could not stop the camera: {error}.");
        }
    });
}

/// Called by the button of the page.
#[tauri::command]
pub(crate) async fn nativella_cancel_scan(app: AppHandle) -> Result<(), String> {
    let Some(scanner) = app.try_state::<Scanner>() else {
        return Ok(());
    };
    // Not the one of an earlier scan: PHP takes it away when it is done.
    let Some(session) = lock(&scanner.session).take() else {
        return Ok(());
    };
    // First, so that PHP goes on whatever the plugin does.
    let _ = session.outcomes.send(Outcome::Cancelled);

    scanner
        .handle
        .run_mobile_plugin_async::<Value>("cancel", ())
        .await
        .map(|_| ())
        .map_err(failed)
}

/// Whether the user lets the application use the camera: asked the first
/// time.
fn camera_allowed(app: &AppHandle) -> Result<bool, String> {
    let (sender, receiver) = mpsc::channel();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let scanner = app.state::<Scanner>();
        // The receiver is gone when the user took too long.
        let _ = sender.send(permission(&scanner.handle).await);
    });

    receiver.recv_timeout(PERMISSION_TIMEOUT).map_err(|_| {
        format!(
            "The user did not answer whether the application may use the camera within {} seconds.",
            PERMISSION_TIMEOUT.as_secs()
        )
    })?
}

async fn permission(handle: &PluginHandle<Wry>) -> Result<bool, String> {
    let mut status: Permissions = handle
        .run_mobile_plugin_async("checkPermissions", ())
        .await
        .map_err(failed)?;
    if matches!(
        status.camera,
        PermissionState::Prompt | PermissionState::PromptWithRationale
    ) {
        status = handle
            .run_mobile_plugin_async("requestPermissions", ())
            .await
            .map_err(failed)?;
    }

    Ok(status.camera == PermissionState::Granted)
}

/// The script that shows the frame and the button of the page with
/// `cancel_label`, or takes them away.
fn overlay(cancel_label: Option<&str>) -> Result<String, String> {
    // JSON is valid JavaScript: the label needs no other escaping.
    let label = serde_json::to_string(&cancel_label).map_err(failed)?;

    Ok(format!("({OVERLAY})({label});"))
}

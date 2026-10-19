//! `geolocation.*`: where the phone is, for applications that ask for the
//! `geolocation` permission. The user is asked on the first call.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::plugin::PermissionState;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_geolocation::{Geolocation, GeolocationExt, PermissionType, PositionOptions};

use super::{Call, failed};

/// Whether the application asks for `geolocation`: only then does lib.rs
/// register the plugin.
pub(crate) fn registered(app: &AppHandle) -> bool {
    app.try_state::<Geolocation<Wry>>().is_some()
}

/// Under the 30 seconds Android's webview waits for a page (wry): a phone that
/// cannot find itself fails the request, not the page. Android's own search
/// ends after 30 seconds.
const POSITION_TIMEOUT: Duration = Duration::from_secs(20);
/// The user reads the question of the system: longer, but not forever.
const PERMISSION_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Accuracy {
    Balanced,
    High,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CurrentPosition {
    accuracy: Accuracy,
    /// Seconds: how old a known position may be.
    maximum_age: u32,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "currentPosition" => call
            .payload::<CurrentPosition>()
            .and_then(|payload| current_position(call, payload)),
        _ => return None,
    })
}

/// The position, or null when the user does not allow it.
fn current_position(call: &Call<'_>, payload: CurrentPosition) -> Result<Value, String> {
    let app = call.app.clone();
    let status = in_thread(
        PERMISSION_TIMEOUT,
        "The user did not answer whether the application may know the location",
        move || {
            let geolocation = app.geolocation();
            let status = geolocation.check_permissions().map_err(failed)?;
            if !matches!(
                status.location,
                PermissionState::Prompt | PermissionState::PromptWithRationale
            ) {
                return Ok(status);
            }

            geolocation
                .request_permissions(Some(vec![PermissionType::Location]))
                .map_err(failed)
        },
    )?;
    // Android 12 lets the user grant an approximate location only.
    if status.location != PermissionState::Granted
        && status.coarse_location != PermissionState::Granted
    {
        return Ok(json!({ "position": null }));
    }

    let options = PositionOptions {
        enable_high_accuracy: payload.accuracy == Accuracy::High,
        // Ignored by Android: see POSITION_TIMEOUT.
        timeout: 10_000,
        maximum_age: payload.maximum_age.saturating_mul(1000),
    };
    let app = call.app.clone();
    let position = in_thread(
        POSITION_TIMEOUT,
        "The phone did not find where it is",
        move || {
            app.geolocation()
                .get_current_position(Some(options))
                .map_err(failed)
        },
    )?;

    Ok(json!({
        "position": {
            "latitude": position.coords.latitude,
            "longitude": position.coords.longitude,
            "accuracy": position.coords.accuracy,
            "altitude": position.coords.altitude,
            "altitudeAccuracy": position.coords.altitude_accuracy,
            "speed": position.coords.speed,
            "heading": position.coords.heading,
            // Milliseconds since the epoch.
            "timestamp": position.timestamp,
        }
    }))
}

/// Runs `work` aside, for at most `timeout`: a plugin that never answers (iOS
/// without the reason for a permission in Info.plist) must not hold PHP.
fn in_thread<T: Send + 'static>(
    timeout: Duration,
    what: &str,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::Builder::new()
        .name("geolocation".to_owned())
        .spawn(move || {
            // The receiver is gone if it took too long.
            let _ = sender.send(work());
        })
        .map_err(|error| format!("The geolocation thread cannot start: {error}."))?;

    receiver
        .recv_timeout(timeout)
        .map_err(|_| format!("{what} within {} seconds.", timeout.as_secs()))?
}

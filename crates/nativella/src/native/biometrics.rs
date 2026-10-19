//! `biometrics.*`: fingerprint or face authentication, on phones whose
//! application asks for the `biometrics` permission.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::plugin::mobile::PluginInvokeError;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_biometric::{AuthOptions, Biometric, BiometricExt, Error};

use super::{Call, failed};

/// Whether the application asks for `biometrics`: only then does lib.rs
/// register the plugin.
pub(crate) fn registered(app: &AppHandle) -> bool {
    app.try_state::<Biometric<Wry>>().is_some()
}

/// The answers that mean "not authenticated", rather than a failure: the
/// user cancelled, did not match, or tried too often (Android and iOS codes).
const REFUSALS: &[&str] = &[
    "appCancel",
    "authenticationFailed",
    "biometryLockout",
    "systemCancel",
    "userCancel",
    "userFallback",
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Authenticate {
    reason: String,
    title: Option<String>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    let biometric = call.app.biometric();

    Some(match method {
        "isAvailable" => biometric
            .status()
            .map(|status| json!({ "available": status.is_available }))
            .map_err(failed),
        "authenticate" => call.payload::<Authenticate>().and_then(|payload| {
            let options = AuthOptions {
                // The PIN of the phone when biometrics fail, as everywhere
                // else on the phone.
                allow_device_credential: true,
                title: payload.title,
                ..AuthOptions::default()
            };

            match biometric.authenticate(payload.reason, options) {
                Ok(()) => Ok(json!({ "authenticated": true })),
                Err(Error::PluginInvoke(PluginInvokeError::InvokeRejected(response)))
                    if response
                        .code
                        .as_deref()
                        .is_some_and(|code| REFUSALS.contains(&code)) =>
                {
                    Ok(json!({ "authenticated": false }))
                }
                Err(error) => Err(failed(error)),
            }
        }),
        _ => return None,
    })
}

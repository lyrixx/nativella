//! `haptics.*`: vibrations and haptic feedback, on phones whose application
//! asks for the `haptics` permission.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_haptics::{Haptics, HapticsExt, ImpactFeedbackStyle, NotificationFeedbackType};

use super::{Call, failed};

/// Whether the application asks for `haptics`: only then does lib.rs
/// register the plugin.
pub(crate) fn registered(app: &AppHandle) -> bool {
    app.try_state::<Haptics<Wry>>().is_some()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Vibrate {
    milliseconds: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Impact {
    impact: ImpactFeedbackStyle,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Feedback {
    feedback: NotificationFeedbackType,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    let haptics = call.app.haptics();

    let result = match method {
        "vibrate" => call
            .payload::<Vibrate>()
            .and_then(|payload| haptics.vibrate(payload.milliseconds).map_err(failed)),
        "impact" => call
            .payload::<Impact>()
            .and_then(|payload| haptics.impact_feedback(payload.impact).map_err(failed)),
        "feedback" => call.payload::<Feedback>().and_then(|payload| {
            haptics
                .notification_feedback(payload.feedback)
                .map_err(failed)
        }),
        "selection" => haptics.selection_feedback().map_err(failed),
        _ => return None,
    };

    Some(result.map(|()| json!({})))
}

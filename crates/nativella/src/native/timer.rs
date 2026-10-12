//! `timer.*`: PHP only runs to answer requests; timers wake it up with
//! `timer.fired` events, for periodic work (a sync, a cleanup, a reminder).
//!
//! They live as long as the application: register them at boot.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

use super::{Call, State, lock};
use crate::events;

/// The running timers, by name. Setting a flag stops its timer.
#[derive(Default)]
pub(crate) struct Timers(std::sync::Mutex<HashMap<String, Arc<AtomicBool>>>);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Repeat {
    Once,
    Forever,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Schedule {
    name: String,
    seconds: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Name {
    name: String,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    let timers = &call.state().timers;

    Some(match method {
        "every" => call
            .payload::<Schedule>()
            .and_then(|payload| start(call.app, timers, payload, Repeat::Forever)),
        "after" => call
            .payload::<Schedule>()
            .and_then(|payload| start(call.app, timers, payload, Repeat::Once)),
        "cancel" => call.payload::<Name>().map(|payload| {
            if let Some(stopped) = lock(&timers.0).remove(&payload.name) {
                stopped.store(true, Ordering::Relaxed);
            }
            json!({})
        }),
        _ => return None,
    })
}

fn start(
    app: &AppHandle,
    timers: &Timers,
    schedule: Schedule,
    repeat: Repeat,
) -> Result<Value, String> {
    // Below a second, PHP would do nothing else.
    if !(1.0..=31_536_000.0).contains(&schedule.seconds) {
        return Err(format!(
            "A timer fires every second at most, and once a year at least: {} seconds is out of range.",
            schedule.seconds
        ));
    }

    let stopped = Arc::new(AtomicBool::new(false));
    // A timer of the same name is replaced.
    if let Some(previous) = lock(&timers.0).insert(schedule.name.clone(), Arc::clone(&stopped)) {
        previous.store(true, Ordering::Relaxed);
    }

    let app = app.clone();
    let interval = Duration::from_secs_f64(schedule.seconds);
    thread::Builder::new()
        .name(format!("timer {}", schedule.name))
        .spawn(move || {
            loop {
                thread::sleep(interval);
                if stopped.load(Ordering::Relaxed) {
                    return;
                }
                events::send(&app, "timer.fired", json!({ "name": schedule.name }));
                if repeat == Repeat::Once {
                    // Unless another timer took the name since.
                    let state = app.state::<State>();
                    let mut timers = lock(&state.timers.0);
                    if timers
                        .get(&schedule.name)
                        .is_some_and(|flag| Arc::ptr_eq(flag, &stopped))
                    {
                        timers.remove(&schedule.name);
                    }

                    return;
                }
            }
        })
        .map_err(|error| format!("The timer thread cannot start: {error}."))?;

    Ok(json!({}))
}

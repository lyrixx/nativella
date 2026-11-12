//! `timer.*`: PHP only runs to answer requests; timers wake it up with
//! `timer.fired` events, for periodic work (a sync, a cleanup, a reminder).
//!
//! They live as long as the application: register them at boot.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

use super::{Call, State, lock};
use crate::events;

/// The running timers, by name.
#[derive(Default)]
pub(crate) struct Timers {
    running: std::sync::Mutex<HashMap<String, Running>>,
    last_id: AtomicU64,
}

struct Running {
    /// Tells a timer from the one that took its name.
    id: u64,
    /// Dropping it stops the timer, and ends its thread at once.
    _stop: mpsc::Sender<()>,
}

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
            lock(&timers.running).remove(&payload.name);
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

    let id = timers.last_id.fetch_add(1, Ordering::Relaxed) + 1;
    let (stop, stopped) = mpsc::channel();
    // A timer of the same name is replaced.
    lock(&timers.running).insert(schedule.name.clone(), Running { id, _stop: stop });

    let app = app.clone();
    let interval = Duration::from_secs_f64(schedule.seconds);
    // Set until PHP has handled the last tick.
    let firing = Arc::new(AtomicBool::new(false));
    thread::Builder::new()
        .name(format!("timer {}", schedule.name))
        .spawn(move || {
            loop {
                if !elapses(&stopped, interval) {
                    return;
                }
                // PHP is busy, with a dialog for one: ticks would pile up.
                if firing.swap(true, Ordering::SeqCst) {
                    continue;
                }
                let handled = Arc::clone(&firing);
                events::send_then(
                    &app,
                    "timer.fired",
                    json!({ "name": schedule.name }),
                    move || handled.store(false, Ordering::SeqCst),
                );
                if repeat == Repeat::Once {
                    // Unless another timer took the name since.
                    let state = app.state::<State>();
                    let mut timers = lock(&state.timers.running);
                    if timers
                        .get(&schedule.name)
                        .is_some_and(|running| running.id == id)
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

/// Waits for `interval`: false when the timer was stopped meanwhile, without
/// waiting for the end of an interval that may be a year.
fn elapses(stopped: &mpsc::Receiver<()>, interval: Duration) -> bool {
    stopped.recv_timeout(interval) == Err(RecvTimeoutError::Timeout)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn a_stopped_timer_does_not_wait_for_its_interval() {
        let (stop, stopped) = mpsc::channel();
        assert!(elapses(&stopped, Duration::from_millis(20)));

        let started = Instant::now();
        drop(stop);
        assert!(!elapses(&stopped, Duration::from_secs(3600)));
        assert!(started.elapsed() < Duration::from_secs(60));
    }
}

//! `power.*`: the battery and the power source, and keeping the computer
//! awake. A thread watches them for `power.sourceChanged`, when the computer
//! goes on battery or back on mains, and `power.resumed`, when it wakes from
//! sleep.

use std::thread;
use std::time::Duration;
#[cfg(not(target_os = "linux"))]
use std::time::{Instant, SystemTime};

use serde::Deserialize;
use serde_json::{Value, json};
use starship_battery::units::energy::watt_hour;
use starship_battery::units::time::second;
use starship_battery::{Manager, State};
use tauri::AppHandle;

use super::{Call, failed, lock};
use crate::events;

/// How often the thread looks: unplugging the computer is no emergency.
const POLL: Duration = Duration::from_secs(5);
/// A sleep shorter than that is no news.
const SLEPT: Duration = Duration::from_secs(15);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct KeepAwake {
    /// What the system shows the user, next to the name of the application.
    reason: String,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "status" => status(),
        "keepAwake" => call
            .payload::<KeepAwake>()
            .and_then(|payload| keep_awake(call, payload)),
        "allowSleep" => {
            // Dropped once the lock is released.
            let awake = lock(&call.state().awake).take();
            drop(awake);
            Ok(json!({}))
        }
        _ => return None,
    })
}

/// Keeps the screen on and the computer awake, replacing what did, if
/// anything. The previous one goes first: on Windows, dropping it restores
/// the state it found.
fn keep_awake(call: &Call<'_>, payload: KeepAwake) -> Result<Value, String> {
    let previous = lock(&call.state().awake).take();
    drop(previous);

    let awake = keepawake::Builder::default()
        .display(true)
        .idle(true)
        .reason(payload.reason)
        .app_name(call.app.package_info().name.clone())
        .app_reverse_domain(call.app.config().identifier.clone())
        .create()
        .map_err(|error| format!("The computer cannot be kept awake: {error}."))?;
    *lock(&call.state().awake) = Some(awake);

    Ok(json!({}))
}

/// Starts watching, for the whole life of the application.
pub(crate) fn watch(app: &AppHandle) {
    let app = app.clone();
    let spawned = thread::Builder::new()
        .name("power".to_owned())
        .spawn(move || {
            let mut was_on_battery = on_battery();
            let mut sleep = Sleep::new();

            loop {
                thread::sleep(POLL);

                if sleep.slept() > SLEPT {
                    events::send(&app, "power.resumed", json!({}));
                }

                let now = on_battery();
                if now.is_some() && now != was_on_battery {
                    events::send(&app, "power.sourceChanged", json!({ "onBattery": now }));
                    was_on_battery = now;
                }
            }
        });

    if let Err(error) = spawned {
        eprintln!("[power] Cannot watch the power source: {error}.");
    }
}

/// How long the computer slept since the last look.
///
/// Linux keeps a clock that counts sleep (CLOCK_BOOTTIME) next to one that
/// does not (CLOCK_MONOTONIC): their gap grows only while it sleeps. Other
/// systems compare the wall clock to the monotonic one, which stops while
/// macOS sleeps, not while Windows does; NTP or a hand setting the clock
/// moves the wall clock too.
struct Sleep {
    #[cfg(target_os = "linux")]
    gap: Duration,
    #[cfg(not(target_os = "linux"))]
    wall: SystemTime,
    #[cfg(not(target_os = "linux"))]
    monotonic: Instant,
}

impl Sleep {
    #[cfg(target_os = "linux")]
    fn new() -> Self {
        Self { gap: Self::gap() }
    }

    #[cfg(target_os = "linux")]
    fn slept(&mut self) -> Duration {
        let gap = Self::gap();
        let slept = gap.saturating_sub(self.gap);
        self.gap = gap;

        slept
    }

    #[cfg(target_os = "linux")]
    fn gap() -> Duration {
        let clock = |id| {
            let mut time = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            // SAFETY: a valid clock id, and a timespec to fill.
            unsafe { libc::clock_gettime(id, &mut time) };
            Duration::new(
                u64::try_from(time.tv_sec).unwrap_or_default(),
                u32::try_from(time.tv_nsec).unwrap_or_default(),
            )
        };

        clock(libc::CLOCK_BOOTTIME).saturating_sub(clock(libc::CLOCK_MONOTONIC))
    }

    #[cfg(not(target_os = "linux"))]
    fn new() -> Self {
        Self {
            wall: SystemTime::now(),
            monotonic: Instant::now(),
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn slept(&mut self) -> Duration {
        // A wall clock set back is no sleep: no duration.
        let slept = SystemTime::now()
            .duration_since(self.wall)
            .unwrap_or_default()
            .saturating_sub(self.monotonic.elapsed());
        self.wall = SystemTime::now();
        self.monotonic = Instant::now();

        slept
    }
}

/// Unknown when the system does not say.
fn on_battery() -> Option<bool> {
    status()
        .ok()
        .and_then(|status| status["onBattery"].as_bool())
}

/// All the batteries as one: computers with two of them drain them in turn.
fn status() -> Result<Value, String> {
    let manager = Manager::new().map_err(failed)?;

    let mut energy = 0.0;
    let mut energy_full = 0.0;
    let mut states = Vec::new();
    let mut time_to_empty = None;
    let mut time_to_full = None;
    for battery in manager.batteries().map_err(failed)? {
        let battery = battery.map_err(failed)?;
        energy += battery.energy().get::<watt_hour>();
        energy_full += battery.energy_full().get::<watt_hour>();
        states.push(battery.state());
        time_to_empty = time_to_empty.or(battery.time_to_empty());
        time_to_full = time_to_full.or(battery.time_to_full());
    }

    // A desktop computer.
    if states.is_empty() || energy_full <= 0.0 {
        return Ok(json!({ "onBattery": false, "battery": null }));
    }

    let state = if states.contains(&State::Discharging) {
        "discharging"
    } else if states.contains(&State::Charging) {
        "charging"
    } else if states.iter().all(|state| *state == State::Full) {
        "full"
    } else if states.contains(&State::Empty) {
        "empty"
    } else if states.contains(&State::Paused) {
        // A charge limit was reached, on mains.
        "notCharging"
    } else {
        "unknown"
    };
    let seconds = |time: Option<starship_battery::units::Time>| {
        // Whole seconds, positive: the cast cannot lose more.
        time.map(|time| time.get::<second>().round().max(0.0) as u64)
    };

    Ok(json!({
        "onBattery": state == "discharging",
        "battery": {
            "level": (energy / energy_full).clamp(0.0, 1.0),
            "state": state,
            "timeToEmpty": seconds(time_to_empty),
            "timeToFull": seconds(time_to_full),
        },
    }))
}

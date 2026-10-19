//! The native methods PHP calls through `nativella_call()`: `<domain>.<method>`,
//! with a JSON payload whose keys are camelCase.
//!
//! Method names and payloads: keep in sync with the API classes of
//! packages/nativella/src/ (`Nativella\Window` for `window.*`, and so on).

pub(crate) mod app;
#[cfg(all(mobile, feature = "biometrics"))]
pub(crate) mod biometrics;
#[cfg(desktop)]
pub(crate) mod child_process;
pub(crate) mod clipboard;
pub(crate) mod dialog;
#[cfg(all(mobile, feature = "geolocation"))]
pub(crate) mod geolocation;
#[cfg(desktop)]
pub(crate) mod global_shortcut;
#[cfg(all(mobile, feature = "haptics"))]
pub(crate) mod haptics;
#[cfg(desktop)]
pub(crate) mod menu;
pub(crate) mod notification;
pub(crate) mod page;
#[cfg(target_os = "linux")]
pub(crate) mod pdf;
#[cfg(desktop)]
pub(crate) mod power;
pub(crate) mod screen;
#[cfg(desktop)]
pub(crate) mod secrets;
pub(crate) mod settings;
pub(crate) mod shell;
pub(crate) mod system;
pub(crate) mod timer;
#[cfg(desktop)]
pub(crate) mod tray;
pub(crate) mod window;

#[cfg(desktop)]
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use nativella_runtime::NativeCall;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
#[cfg(desktop)]
use tauri::Wry;
#[cfg(desktop)]
use tauri::menu::CheckMenuItem;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::MAIN_WINDOW;

/// What the native methods remember between calls.
pub(crate) struct State {
    #[cfg(desktop)]
    pub(crate) child_processes: child_process::ChildProcesses,
    data_dir: PathBuf,
    default_window: Map<String, Value>,
    pub(crate) settings: settings::Store,
    pub(crate) timers: timer::Timers,
    /// Menu item id → the URL its click navigates to.
    #[cfg(desktop)]
    pub(crate) menu_links: Mutex<HashMap<String, String>>,
    /// Menu item id → its checkbox, to tell PHP whether it is checked.
    #[cfg(desktop)]
    pub(crate) checkboxes: Mutex<HashMap<String, CheckMenuItem<Wry>>>,
    /// Shortcut id → the accelerator PHP registered it with.
    #[cfg(desktop)]
    pub(crate) shortcuts: Mutex<HashMap<u32, String>>,
    #[cfg(desktop)]
    pub(crate) tray: Mutex<Option<tauri::tray::TrayIcon<Wry>>>,
}

impl State {
    #[cfg_attr(mobile, expect(unused_variables, reason = "Phones start no process."))]
    pub(crate) fn new(
        project_dir: PathBuf,
        data_dir: PathBuf,
        default_window: Map<String, Value>,
    ) -> Self {
        Self {
            #[cfg(desktop)]
            child_processes: child_process::ChildProcesses::new(project_dir),
            settings: settings::Store::new(data_dir.join("settings.json")),
            timers: timer::Timers::default(),
            data_dir,
            default_window,
            #[cfg(desktop)]
            menu_links: Mutex::default(),
            #[cfg(desktop)]
            checkboxes: Mutex::default(),
            #[cfg(desktop)]
            shortcuts: Mutex::default(),
            #[cfg(desktop)]
            tray: Mutex::default(),
        }
    }

    pub(crate) fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub(crate) fn default_window(&self) -> &Map<String, Value> {
        &self.default_window
    }

    pub(crate) fn has_tray(&self) -> bool {
        #[cfg(desktop)]
        return lock(&self.tray).is_some();

        #[cfg(mobile)]
        return false;
    }
}

/// One native call, as the domain modules see it.
pub(crate) struct Call<'a> {
    pub(crate) app: &'a AppHandle,
    pub(crate) method: &'a str,
    pub(crate) payload: Value,
    /// The window of the request that made the call.
    pub(crate) window: Option<&'a str>,
}

impl Call<'_> {
    /// The payload as `T`, or an error naming the method.
    pub(crate) fn payload<T: DeserializeOwned>(&self) -> Result<T, String> {
        serde_json::from_value(self.payload.clone())
            .map_err(|error| format!("The payload of \"{}\" is invalid: {error}.", self.method))
    }

    /// The window the payload names, or the one that made the call, or the
    /// focused one, or the main one.
    pub(crate) fn target_window(&self, label: Option<&str>) -> Result<WebviewWindow, String> {
        let windows = self.app.webview_windows();

        let window = match label.or(self.window) {
            Some(label) => windows.get(label).cloned(),
            None => windows
                .values()
                .find(|window| window.is_focused().unwrap_or(false))
                .or_else(|| windows.get(MAIN_WINDOW))
                .or_else(|| windows.values().next())
                .cloned(),
        };

        window.ok_or_else(|| match label.or(self.window) {
            Some(label) => format!("There is no \"{label}\" window."),
            None => "There is no window.".to_owned(),
        })
    }

    pub(crate) fn state(&self) -> tauri::State<'_, State> {
        self.app.state::<State>()
    }
}

/// Runs on the PHP thread: blocking here (a modal dialog) is fine, the main
/// thread keeps the UI alive.
pub(crate) fn call(app: &AppHandle, call: NativeCall<'_>) -> Result<Value, String> {
    let Some((domain, method)) = call.method.split_once('.') else {
        return Err(unknown(call.method));
    };
    let full_method = call.method;
    let call = Call {
        app,
        method: full_method,
        payload: call.payload,
        window: call.window,
    };

    let result = match domain {
        "app" => app::call(&call, method),
        #[cfg(desktop)]
        "childProcess" => child_process::call(&call, method),
        #[cfg(all(mobile, feature = "biometrics"))]
        "biometrics" if biometrics::registered(app) => biometrics::call(&call, method),
        "clipboard" => clipboard::call(&call, method),
        "dialog" => dialog::call(&call, method),
        #[cfg(all(mobile, feature = "geolocation"))]
        "geolocation" if geolocation::registered(app) => geolocation::call(&call, method),
        #[cfg(desktop)]
        "globalShortcut" => global_shortcut::call(&call, method),
        #[cfg(all(mobile, feature = "haptics"))]
        "haptics" if haptics::registered(app) => haptics::call(&call, method),
        #[cfg(desktop)]
        "menu" => menu::call(&call, method),
        "notification" => notification::call(&call, method),
        "page" => page::call(&call, method),
        #[cfg(desktop)]
        "power" => power::call(&call, method),
        "screen" => screen::call(&call, method),
        #[cfg(desktop)]
        "secrets" => secrets::call(&call, method),
        "settings" => settings::call(&call, method),
        "shell" => shell::call(&call, method),
        "system" => system::call(&call, method),
        "timer" => timer::call(&call, method),
        #[cfg(desktop)]
        "tray" => tray::call(&call, method),
        "window" => window::call(&call, method),
        #[cfg(mobile)]
        "childProcess" | "globalShortcut" | "menu" | "power" | "secrets" | "tray" => {
            Some(Err(format!(
                "\"{full_method}\" is not available on mobile: check Nativella\\Runtime::isMobile() first."
            )))
        }
        // The APIs of phones this build lacks. A guard, not a pattern: the
        // arms above take the ones it has.
        _ if PHONE_DOMAINS.contains(&domain) => Some(Err(if cfg!(mobile) {
            format!(
                "\"{full_method}\" needs \"{domain}\" in the permissions of the application: see extra.nativella in its composer.json."
            )
        } else {
            format!(
                "\"{full_method}\" is only available on phones: check Nativella\\Runtime::isMobile() first."
            )
        })),
        _ => None,
    };

    result.unwrap_or_else(|| Err(unknown(full_method)))
}

/// Keep in sync with the features of Cargo.toml and the permissions of
/// project.rs.
const PHONE_DOMAINS: &[&str] = &["biometrics", "geolocation", "haptics"];

pub(crate) fn unknown(method: &str) -> String {
    format!("Unknown native method \"{method}\".")
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // A panic while holding one of these locks leaves consistent data behind:
    // each one only guards a plain map or option.
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Turns a Tauri or plugin error into the message PHP throws.
pub(crate) fn failed(error: impl std::fmt::Display) -> String {
    error.to_string()
}

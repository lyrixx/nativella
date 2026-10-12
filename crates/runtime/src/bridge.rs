//! The PHP side of the bridge: a `nativella_call()` function, registered in PHP
//! as an extension, that forwards to the host application.

use std::cell::RefCell;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ext_php_rs::exception::{PhpException, PhpResult};
use ext_php_rs::ffi::zend_long;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::{ClassEntry, ExecutorGlobals, ModuleEntry};
use serde_json::Value;

unsafe extern "C" {
    /// Zend/zend_execute.h: starts the countdown of max_execution_time again,
    /// as set_time_limit() does.
    fn zend_set_timeout(seconds: zend_long, reset_signals: bool);
}

/// A `nativella_call()` made by PHP.
#[derive(Debug)]
pub struct NativeCall<'a> {
    pub method: &'a str,
    pub payload: Value,
    /// The window whose request is running, `None` during an internal request.
    pub window: Option<&'a str>,
}

/// Handles a `nativella_call()`: a JSON result or an error message, thrown in
/// PHP as a `RuntimeException`.
///
/// It runs on the PHP thread, so it may block (a modal dialog, for instance)
/// but must never wait for a PHP request to complete.
pub type NativeHandler = dyn Fn(NativeCall<'_>) -> Result<Value, String> + Send + Sync;

static HANDLER: OnceLock<Box<NativeHandler>> = OnceLock::new();

thread_local! {
    /// The window of the request PHP is serving, on the PHP thread.
    static WINDOW: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub(crate) fn install(handler: Box<NativeHandler>) {
    assert!(
        HANDLER.set(handler).is_ok(),
        "The native handler is already installed: PHP can only be started once per process."
    );
}

/// Runs `serve` with `window` as the window of the native calls it makes.
pub(crate) fn with_window<T>(window: Option<&str>, serve: impl FnOnce() -> T) -> T {
    WINDOW.with_borrow_mut(|current| *current = window.map(str::to_owned));
    let result = serve();
    WINDOW.with_borrow_mut(|current| *current = None);

    result
}

/// Calls a native method of the host application.
///
/// The payload and the result are JSON documents, so the PHP side stays a thin
/// userland wrapper around `json_encode()` and `json_decode()`.
#[php_function]
pub fn nativella_call(method: &str, payload: &str) -> PhpResult<String> {
    let payload: Value = serde_json::from_str(payload).map_err(|error| {
        runtime_exception(format!(
            "The payload of \"{method}\" is not valid JSON: {error}."
        ))
    })?;

    // The runtime installs the handler before it starts PHP.
    let handler = HANDLER
        .get()
        .expect("The native handler is installed before PHP starts.");

    let window = WINDOW.with_borrow(Clone::clone);
    let call = NativeCall {
        method,
        payload,
        window: window.as_deref(),
    };

    let started = Instant::now();
    let result = handler(call);
    if started.elapsed() >= WAITED {
        restart_time_limit();
    }

    Ok(result.map_err(runtime_exception)?.to_string())
}

/// A native call that took that long waited for the user (a dialog, a
/// fingerprint, a permission). Quicker ones do not restart the time limit:
/// a loop of them must still time out.
const WAITED: Duration = Duration::from_secs(1);

/// Waiting for the user is not running PHP: as with sleep() on Linux, it does
/// not count against max_execution_time. Where PHP measures the time of the
/// wall clock (Android, ZTS builds), a dialog left open would otherwise kill
/// the request.
fn restart_time_limit() {
    let seconds = ExecutorGlobals::get().timeout_seconds;
    if seconds > 0 {
        // SAFETY: on the PHP thread, during a request, like set_time_limit().
        unsafe { zend_set_timeout(seconds, false) };
    }
}

fn runtime_exception(message: String) -> PhpException {
    // SPL, and so RuntimeException, is always compiled into PHP.
    let class = ClassEntry::try_find("RuntimeException").expect("RuntimeException always exists.");

    PhpException::new(message, 0, class)
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(nativella_call))
}

/// The extension to load at module startup, next to the ones from php.ini.
pub(crate) fn module_entry() -> *mut ModuleEntry {
    get_module()
}

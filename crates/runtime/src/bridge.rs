//! The PHP side of the bridge: a `nativella_call()` function, registered in PHP
//! as an extension, that forwards to the host application.

use std::cell::RefCell;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ext_php_rs::ffi::zend_long;
use ext_php_rs::prelude::*;
use ext_php_rs::zend::{ExecutorGlobals, ModuleEntry};
use serde_json::{Value, json};

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

/// Why a native call failed: what PHP can act on, next to the message, which
/// is for people.
///
/// Keep in sync with packages/nativella/src/Exception/ErrorKind.php.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The system, or a library, failed: anything the other kinds do not say.
    Failed,
    /// The payload is not what the method reads: not JSON, a missing or
    /// unknown key, a value out of range.
    InvalidPayload,
    /// What the call names does not exist: a window, a menu item, a process,
    /// a file.
    NotFound,
    /// The user, or a device, did not answer in time.
    Timeout,
    /// The method does not exist here: on this platform, in this build, or
    /// without the permission or the configuration it needs.
    Unavailable,
}

impl ErrorKind {
    /// Its name on the wire.
    pub fn name(self) -> &'static str {
        match self {
            Self::Failed => "failed",
            Self::InvalidPayload => "invalidPayload",
            Self::NotFound => "notFound",
            Self::Timeout => "timeout",
            Self::Unavailable => "unavailable",
        }
    }
}

/// A native call that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeError {
    pub kind: ErrorKind,
    /// A sentence, for the developer.
    pub message: String,
}

impl NativeError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NativeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

/// A failure nothing more is known of.
impl From<String> for NativeError {
    fn from(message: String) -> Self {
        Self::new(ErrorKind::Failed, message)
    }
}

impl From<&str> for NativeError {
    fn from(message: &str) -> Self {
        Self::new(ErrorKind::Failed, message)
    }
}

/// Handles a `nativella_call()`: a JSON result, or why it failed.
///
/// It runs on the PHP thread, so it may block (a modal dialog, for instance)
/// but must never wait for a PHP request to complete.
pub type NativeHandler = dyn Fn(NativeCall<'_>) -> Result<Value, NativeError> + Send + Sync;

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
/// The payload and the answer are JSON documents, so the PHP side stays a thin
/// userland wrapper around `json_encode()` and `json_decode()`. The answer is
/// `{"result": {…}}`, or `{"error": {"kind": "…", "message": "…"}}` with an
/// [`ErrorKind`]: it never throws, a kind is not something an exception of
/// PHP carries.
#[php_function]
pub fn nativella_call(method: &str, payload: &str) -> String {
    answer(call(method, payload)).to_string()
}

fn call(method: &str, payload: &str) -> Result<Value, NativeError> {
    let payload: Value = serde_json::from_str(payload).map_err(|error| {
        NativeError::new(
            ErrorKind::InvalidPayload,
            format!("The payload of \"{method}\" is not valid JSON: {error}."),
        )
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

    result
}

/// What PHP gets: keep in sync with
/// packages/nativella/src/Client/ExtensionClient.php.
fn answer(result: Result<Value, NativeError>) -> Value {
    match result {
        Ok(result) => json!({ "result": result }),
        Err(error) => json!({ "error": { "kind": error.kind.name(), "message": error.message } }),
    }
}

/// A native call that took that long waited for the user (a dialog, a
/// fingerprint, a permission). Quicker ones do not restart the time limit:
/// a loop of them must still time out.
const WAITED: Duration = Duration::from_secs(1);

/// Waiting for the user is not running PHP: as with sleep() on Linux, it does
/// not count against max_execution_time. Where PHP measures the time of the
/// wall clock (on Apple's processors, macOS and iOS, and in thread-safe
/// builds), a dialog left open would otherwise kill the request.
fn restart_time_limit() {
    let seconds = ExecutorGlobals::get().timeout_seconds;
    if seconds > 0 {
        // SAFETY: on the PHP thread, during a request, like set_time_limit().
        unsafe { zend_set_timeout(seconds, false) };
    }
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(nativella_call))
}

/// The extension to load at module startup, next to the ones from php.ini.
pub(crate) fn module_entry() -> *mut ModuleEntry {
    get_module()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_and_an_error_are_told_apart() {
        assert_eq!(
            answer(Ok(json!({ "path": null }))).to_string(),
            r#"{"result":{"path":null}}"#
        );
        assert_eq!(
            answer(Err(NativeError::new(
                ErrorKind::NotFound,
                "There is no \"settings\" window."
            )))
            .to_string(),
            r#"{"error":{"kind":"notFound","message":"There is no \"settings\" window."}}"#
        );
    }

    #[test]
    fn every_kind_has_its_name_on_the_wire() {
        let names = [
            ErrorKind::Failed,
            ErrorKind::InvalidPayload,
            ErrorKind::NotFound,
            ErrorKind::Timeout,
            ErrorKind::Unavailable,
        ]
        .map(ErrorKind::name);

        assert_eq!(
            names,
            [
                "failed",
                "invalidPayload",
                "notFound",
                "timeout",
                "unavailable"
            ]
        );
    }

    #[test]
    fn a_plain_message_is_a_plain_failure() {
        assert_eq!(
            NativeError::from("The disk is full."),
            NativeError::new(ErrorKind::Failed, "The disk is full.")
        );
    }
}

//! The PHP side of the bridge: a `native_call()` function, registered in PHP as
//! an extension, that forwards to the host application.

use std::sync::OnceLock;

use ext_php_rs::exception::{PhpException, PhpResult};
use ext_php_rs::prelude::*;
use ext_php_rs::zend::{ClassEntry, ModuleEntry};
use serde_json::Value;

/// Handles a `native_call()` made by PHP: a method name and a JSON payload in,
/// a JSON result or an error message out.
///
/// It runs on the PHP thread, so it may block (a modal dialog, for instance)
/// but must never wait for a PHP request to complete.
pub type NativeHandler = dyn Fn(&str, Value) -> Result<Value, String> + Send + Sync;

static HANDLER: OnceLock<Box<NativeHandler>> = OnceLock::new();

pub(crate) fn install(handler: Box<NativeHandler>) {
    assert!(
        HANDLER.set(handler).is_ok(),
        "The native handler is already installed: PHP can only be started once per process."
    );
}

/// Calls a native method of the host application.
///
/// The payload and the result are JSON documents, so the PHP side stays a thin
/// userland wrapper around `json_encode()` and `json_decode()`.
#[php_function]
pub fn native_call(method: &str, payload: &str) -> PhpResult<String> {
    let payload: Value = serde_json::from_str(payload).map_err(|error| {
        runtime_exception(format!(
            "The payload of \"{method}\" is not valid JSON: {error}."
        ))
    })?;

    // The runtime installs the handler before it starts PHP.
    let handler = HANDLER
        .get()
        .expect("The native handler is installed before PHP starts.");

    let result = handler(method, payload).map_err(runtime_exception)?;

    Ok(result.to_string())
}

fn runtime_exception(message: String) -> PhpException {
    // SPL, and so RuntimeException, is always compiled into PHP.
    let class = ClassEntry::try_find("RuntimeException").expect("RuntimeException always exists.");

    PhpException::new(message, 0, class)
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(native_call))
}

/// The extension to load at module startup, next to the ones from php.ini.
pub(crate) fn module_entry() -> *mut ModuleEntry {
    get_module()
}

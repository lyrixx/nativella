//! Runs a PHP application in-process, through a custom SAPI.
//!
//! The webview's requests never touch the network: the host hands them to
//! [`PhpRuntime::dispatch`], which serves public files directly and runs the
//! front controller for everything else.
//!
//! PHP is built without thread safety (NTS) here, so the interpreter lives on a
//! single dedicated thread for the whole life of the process and requests are
//! queued to it.

/// `eprintln!`, without its panic when the standard error output is gone (a
/// closed pipe): PHP calls the SAPI from C, and webviews call the scheme
/// handler from C, where a panic cannot unwind and aborts the process.
macro_rules! log {
    ($($argument:tt)*) => {{
        use std::io::Write as _;
        // There is nowhere left to report the failure.
        let _ = writeln!(std::io::stderr(), $($argument)*);
    }};
}

mod absolute_urls;
mod bridge;
mod cookies;
#[cfg(all(nativella_static_php, target_os = "linux", target_env = "gnu"))]
mod glibc_compat;
mod redirects;
mod runtime;
mod sapi;
mod static_files;

pub use bridge::{ErrorKind, NativeCall, NativeError, NativeHandler};
pub use redirects::is_navigation;
pub use runtime::{PhpApp, PhpRuntime, RequestOrigin};

/// A response with `status` and nothing else.
fn empty_response(status: http::StatusCode) -> http::Response<Vec<u8>> {
    let mut response = http::Response::new(Vec::new());
    *response.status_mut() = status;

    response
}

//! Runs a PHP application in-process, through a custom SAPI.
//!
//! The webview's requests never touch the network: the host hands them to
//! [`PhpRuntime::dispatch`], which serves public files directly and runs the
//! front controller for everything else.
//!
//! PHP is built without thread safety (NTS) here, so the interpreter lives on a
//! single dedicated thread for the whole life of the process and requests are
//! queued to it.

mod absolute_urls;
mod bridge;
mod cookies;
#[cfg(all(nativella_static_php, target_os = "linux", target_env = "gnu"))]
mod glibc_compat;
mod redirects;
mod runtime;
mod sapi;
mod static_files;

pub use bridge::{NativeCall, NativeHandler};
pub use redirects::is_navigation;
pub use runtime::{PhpApp, PhpRuntime, RequestOrigin};

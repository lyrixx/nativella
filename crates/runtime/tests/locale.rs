//! PHP runs in the C locale, whatever locale the host took: GTK takes the one
//! of the user. `setlocale()` keeps working in PHP, for one request.
//!
//! One test: PHP starts once per process.

#![cfg(target_os = "linux")]

use std::fs;
use std::path::Path;

use http::Request;
use nativella_runtime::{PhpApp, PhpRuntime, RequestOrigin};
use serde_json::json;

const SCRIPT: &str = r"<?php
echo setlocale(LC_NUMERIC, 0), ' ', setlocale(LC_TIME, 0), ' ', setlocale(LC_TIME, 'C.UTF-8');
";

#[test]
fn php_runs_in_the_c_locale() {
    // As GTK does, with a locale every glibc has.
    // SAFETY: before PHP starts, and no other thread reads the locale.
    let taken = unsafe { libc::setlocale(libc::LC_ALL, c"C.UTF-8".as_ptr()) };
    assert!(!taken.is_null(), "The C library has the C.UTF-8 locale.");

    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join("locale");
    fs::create_dir_all(project.join("public")).unwrap();
    fs::write(project.join("public/index.php"), SCRIPT).unwrap();

    let runtime = PhpRuntime::start(
        PhpApp::from_project_dir(&project).unwrap(),
        Box::new(|_| Ok(json!({}))),
    );
    let get = || {
        let response = runtime.dispatch_blocking(
            Request::get("php://localhost/").body(Vec::new()).unwrap(),
            RequestOrigin::Internal,
        );

        String::from_utf8_lossy(response.body()).into_owned()
    };

    assert_eq!(get(), "C C C.UTF-8");
    assert_eq!(
        get(),
        "C C C.UTF-8",
        "PHP sets the locale back after a request."
    );
}

//! PHP reads the php.ini of Nativella. The static PHP reads no other: neither
//! the php.ini PHPRC points to nor the directory of PHP_INI_SCAN_DIR, which
//! belong to another PHP. The PHP of the system reads them, before the one of
//! Nativella.
//!
//! One test: PHP starts once per process.

use std::env;
use std::fs;
use std::path::Path;

use http::Request;
use nativella_runtime::{PhpApp, PhpRuntime, RequestOrigin};
use serde_json::json;

const SCRIPT: &str = r"<?php
echo ini_get('hard_timeout'), ' ', ini_get('memory_limit'), ' ', ini_get('precision');
";

#[test]
fn reads_the_php_ini_of_nativella() {
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join("php-ini");
    let machine = project.join("machine");
    fs::create_dir_all(project.join("public")).unwrap();
    fs::create_dir_all(machine.join("conf.d")).unwrap();
    fs::write(project.join("public/index.php"), SCRIPT).unwrap();
    fs::write(
        machine.join("php.ini"),
        "memory_limit = 7M\nhard_timeout = 7\n",
    )
    .unwrap();
    fs::write(machine.join("conf.d/precision.ini"), "precision = 7\n").unwrap();

    // SAFETY: before PHP starts, and no other thread reads the environment.
    unsafe {
        env::set_var("PHPRC", &machine);
        env::set_var("PHP_INI_SCAN_DIR", machine.join("conf.d"));
    }

    let runtime = PhpRuntime::start(
        PhpApp::from_project_dir(&project).unwrap(),
        Box::new(|_| Ok(json!({}))),
    );
    let response = runtime.dispatch_blocking(
        Request::get("php://localhost/").body(Vec::new()).unwrap(),
        RequestOrigin::Internal,
    );

    let expected = if cfg!(nativella_static_php) {
        "0 128M 14"
    } else {
        "0 7M 7"
    };
    assert_eq!(String::from_utf8_lossy(response.body()), expected);
}

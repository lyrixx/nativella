//! The time a native call spends waiting for the user does not count against
//! max_execution_time: a user may leave a dialog open as long as they like.
//! Quick calls do not stop the clock, though.
//!
//! One test: PHP starts once per process.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use http::Request;
use nativella_runtime::{PhpApp, PhpRuntime, RequestOrigin};
use serde_json::json;

const SCRIPT: &str = r"<?php
set_time_limit(1);
if ('wait' === $_GET['case']) {
    nativella_call('dialog.confirm', '{}');
} else {
    while (true) {
        nativella_call('settings.get', '{}');
    }
}
echo 'Answered.';
";

#[test]
fn only_waiting_does_not_count() {
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join("time-limit");
    fs::create_dir_all(project.join("public")).unwrap();
    fs::write(project.join("public/index.php"), SCRIPT).unwrap();

    let runtime = PhpRuntime::start(
        PhpApp::from_project_dir(&project)
            .unwrap()
            // PHP answers a fatal error with a 500 only when it does not
            // display it: php.ini files of development machines do.
            .with_ini("display_errors", "0"),
        Box::new(|call| {
            if call.method == "dialog.confirm" {
                // Busy, not asleep: PHP without threads counts the CPU time
                // of the process. Longer than the limit and the hard timeout
                // (2 s by default) after it, which would kill the process.
                let started = Instant::now();
                while started.elapsed() < Duration::from_millis(3500) {
                    std::hint::spin_loop();
                }
            }

            Ok(json!({ "confirmed": true }))
        }),
    );
    let get = |uri: &str| {
        runtime.dispatch_blocking(
            Request::get(uri).body(Vec::new()).unwrap(),
            RequestOrigin::Internal,
        )
    };

    let waited = get("php://localhost/?case=wait");
    assert_eq!(String::from_utf8_lossy(waited.body()), "Answered.");
    assert_eq!(waited.status(), 200);

    let looped = get("php://localhost/?case=loop");
    assert_eq!(
        looped.status(),
        500,
        "A loop of quick native calls times out."
    );
}

//! The requests queued when the PHP thread dies get a 503, as do the ones
//! sent after: none leaves its window waiting.
//!
//! One test: PHP starts once per process.

use std::fs;
use std::path::Path;
use std::sync::mpsc;

use http::Request;
use nativella_runtime::{PhpApp, PhpRuntime, RequestOrigin};
use serde_json::json;

#[test]
fn no_request_waits_for_a_dead_php_thread() {
    let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join("thread-death");
    fs::create_dir_all(project.join("public")).unwrap();
    fs::write(project.join("public/index.php"), "<?php echo 'Hello';").unwrap();

    let runtime = PhpRuntime::start(
        PhpApp::from_project_dir(&project).unwrap(),
        Box::new(|_| Ok(json!({}))),
    );
    let request = || Request::get("php://localhost/").body(Vec::new()).unwrap();

    // PHP takes longer to start than these take to queue.
    runtime.dispatch(request(), RequestOrigin::Internal, |_| {
        panic!("The responder fails, and the PHP thread with it.")
    });
    let (sender, queued) = mpsc::channel();
    for _ in 0..2 {
        let sender = sender.clone();
        runtime.dispatch(request(), RequestOrigin::Internal, move |response| {
            sender.send(response.status()).unwrap();
        });
    }

    assert_eq!(queued.recv().unwrap(), 503);
    assert_eq!(queued.recv().unwrap(), 503);
    assert_eq!(
        runtime
            .dispatch_blocking(request(), RequestOrigin::Internal)
            .status(),
        503
    );
    assert!(runtime.is_idle());
}

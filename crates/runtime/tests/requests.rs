//! What PHP makes of the requests of a page, and what the page gets back.
//!
//! PHP starts once per process: the tests share one runtime, which serves
//! their requests one at a time.

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use http::header::CONTENT_TYPE;
use http::{Request, Response};
use nativella_runtime::{PhpApp, PhpRuntime, RequestOrigin};
use serde_json::json;

const SCRIPT: &str = r"<?php
switch ($_GET['case'] ?? '') {
    case 'fatal':
        ini_set('memory_limit', '8M');
        str_repeat('x', 16 * 1024 * 1024);
        break;
    case 'upload':
        printf('%d %d', $_FILES['photo']['error'], $_FILES['photo']['size']);
        break;
    default:
        echo 'Hello';
}
";

fn runtime() -> &'static PhpRuntime {
    static RUNTIME: OnceLock<PhpRuntime> = OnceLock::new();

    RUNTIME.get_or_init(|| {
        let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join("requests");
        let uploads = project.join("uploads");
        fs::create_dir_all(project.join("public")).unwrap();
        fs::create_dir_all(&uploads).unwrap();
        fs::write(project.join("public/index.php"), SCRIPT).unwrap();

        PhpRuntime::start(
            PhpApp::from_project_dir(&project)
                .unwrap()
                .with_ini("upload_tmp_dir", uploads.to_string_lossy()),
            Box::new(|_| Ok(json!({}))),
        )
    })
}

fn send(request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    runtime().dispatch_blocking(
        request,
        RequestOrigin::Webview {
            window: "main".to_owned(),
        },
    )
}

fn get(path: &str) -> Response<Vec<u8>> {
    send(
        Request::get(format!("php://localhost{path}"))
            .body(Vec::new())
            .unwrap(),
    )
}

fn text(response: &Response<Vec<u8>>) -> String {
    String::from_utf8_lossy(response.body()).into_owned()
}

#[test]
fn a_fatal_error_answers_a_500_without_showing_itself() {
    let failed = get("/?case=fatal");

    assert_eq!(failed.status(), 500);
    assert!(!text(&failed).contains("memory"), "{}", text(&failed));

    let next = get("/");
    assert_eq!(next.status(), 200);
    assert_eq!(text(&next), "Hello");
}

#[test]
fn a_photo_of_a_phone_can_be_uploaded() {
    let photo = vec![b'x'; 4 * 1024 * 1024];
    let mut body = b"--boundary\r\nContent-Disposition: form-data; name=\"photo\"; filename=\"photo.jpg\"\r\nContent-Type: image/jpeg\r\n\r\n".to_vec();
    body.extend_from_slice(&photo);
    body.extend_from_slice(b"\r\n--boundary--\r\n");

    let response = send(
        Request::post("php://localhost/?case=upload")
            .header(CONTENT_TYPE, "multipart/form-data; boundary=boundary")
            .body(body)
            .unwrap(),
    );

    assert_eq!(text(&response), format!("0 {}", photo.len()));
}

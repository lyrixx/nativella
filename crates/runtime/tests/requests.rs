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
    case 'avatar':
        header('Location: /avatar.png');
        break;
    case 'moved':
        header('Location: /?case=input', true, 307);
        break;
    case 'input':
        echo $_SERVER['REQUEST_METHOD'], ' ', file_get_contents('php://input');
        break;
    case 'status':
        http_response_code((int) $_GET['code']);
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
        fs::write(project.join("public/avatar.png"), "PNG").unwrap();

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

#[test]
fn a_status_beyond_http_answers_a_500() {
    assert_eq!(get("/?case=status&code=418").status(), 418);
    assert_eq!(get("/?case=status&code=599").status(), 599);
    assert_eq!(get("/?case=status&code=600").status(), 500);
    assert_eq!(get("/?case=status&code=999").status(), 500);
}

#[test]
fn a_307_sends_the_body_again() {
    let response = send(
        Request::post("php://localhost/?case=moved")
            .header(CONTENT_TYPE, "application/json")
            .body(br#"{"title":"Groceries"}"#.to_vec())
            .unwrap(),
    );

    assert_eq!(text(&response), r#"POST {"title":"Groceries"}"#);
}

#[test]
fn a_fetch_redirected_to_a_public_file_gets_the_file() {
    let response = get("/?case=avatar");

    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()[CONTENT_TYPE], "image/png");
    assert_eq!(text(&response), "PNG");
}

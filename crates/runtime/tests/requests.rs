//! What PHP makes of the requests of a page, and what the page gets back.
//!
//! PHP starts once per process: the tests share one runtime, which serves
//! their requests one at a time.

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use http::header::{CONTENT_TYPE, SET_COOKIE};
use http::{Request, Response};
use nativella_runtime::{ErrorKind, NativeError, PhpApp, PhpRuntime, RequestOrigin};
use serde_json::json;

const SCRIPT: &str = r#"<?php
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
    case 'deleted':
        header('Location: /?case=input');
        break;
    case 'exception':
        throw new RuntimeException('Boom.');
    case 'exit':
        echo 'Bye';
        exit(3);
    case 'cookies':
        setcookie('first', '1');
        setcookie('second', '2');
        break;
    case 'read-cookies':
        echo $_COOKIE['first'] ?? '', ' ', $_COOKIE['second'] ?? '';
        break;
    case 'headers':
        header('Bad Name: 1');
        header("X-Latin-1: caf\xe9");
        header('X-Good: 1');
        echo 'Headers';
        break;
    case 'origin':
        echo json_encode([
            $_SERVER['NATIVELLA_INTERNAL'] ?? null,
            $_SERVER['NATIVELLA_WINDOW'] ?? null,
            $_SERVER['HTTP_NATIVELLA_INTERNAL'] ?? null,
        ]);
        break;
    case 'late-header':
        set_error_handler(static function (int $level, string $message): bool {
            echo $message;

            return true;
        });
        while (ob_get_level() > 0) {
            ob_end_flush();
        }
        echo 'Sent. ';
        header('X-Late: 1');
        break;
    case 'early-output':
        echo 'Created';
        header('X-After: 1');
        http_response_code(201);
        break;
    case 'feed':
        require __DIR__.'/../feed.php';
        break;
    case 'loop':
        header('Location: /?case=loop');
        break;
    case 'ini':
        echo ini_get('user_agent');
        break;
    case 'login':
        setcookie('session', 'abc');
        header('Location: /?case=session');
        break;
    case 'session':
        echo $_COOKIE['session'] ?? 'none';
        break;
    case 'native':
        try {
            echo nativella_call($_GET['method'], $_GET['payload'] ?? '{}');
        } catch (Throwable $error) {
            echo $error::class, ': ', $error->getMessage();
        }
        break;
    case 'form':
        $note = $_FILES['note'] ?? null;
        echo json_encode([
            'post' => $_POST,
            'file' => $note ? [$note['name'], file_get_contents($note['tmp_name'])] : null,
        ]);
        break;
    default:
        echo 'Hello';
}
"#;

/// What a php.ini reads otherwise between double quotes.
const INI_VALUE: &str = r#"\\server\share "quoted" ${HOME} \"#;

fn runtime() -> &'static PhpRuntime {
    static RUNTIME: OnceLock<PhpRuntime> = OnceLock::new();

    RUNTIME.get_or_init(|| {
        let project = Path::new(env!("CARGO_TARGET_TMPDIR")).join("requests");
        let uploads = project.join("uploads");
        fs::create_dir_all(project.join("public")).unwrap();
        fs::create_dir_all(&uploads).unwrap();
        fs::write(project.join("public/index.php"), SCRIPT).unwrap();
        fs::write(project.join("public/avatar.png"), "PNG").unwrap();
        fs::write(project.join("feed.php"), "<?xml version=\"1.0\"?><feed/>").unwrap();

        // SAFETY: before PHP starts, and no other thread reads the environment.
        unsafe {
            std::env::set_var("NATIVELLA_INTERNAL", "1");
            std::env::set_var("NATIVELLA_WINDOW", "forged");
        }

        PhpRuntime::start(
            PhpApp::from_project_dir(&project)
                .unwrap()
                .with_ini("upload_tmp_dir", uploads.to_string_lossy())
                .with_ini("user_agent", INI_VALUE),
            Box::new(|call| match call.method {
                "fail" => Err("It failed.".into()),
                "missing" => Err(NativeError::new(
                    ErrorKind::NotFound,
                    "There is no such thing.",
                )),
                "panic" => panic!("It panicked."),
                _ => Ok(json!({ "window": call.window })),
            }),
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
fn an_uncaught_exception_answers_a_500_without_showing_itself() {
    let failed = get("/?case=exception");

    assert_eq!(failed.status(), 500);
    assert!(!text(&failed).contains("Boom"), "{}", text(&failed));
    assert_eq!(text(&get("/")), "Hello");
}

#[test]
fn exit_ends_the_script_not_the_response() {
    let response = get("/?case=exit");

    assert_eq!(response.status(), 200);
    assert_eq!(text(&response), "Bye");
    assert_eq!(text(&get("/")), "Hello");
}

#[test]
fn every_cookie_reaches_the_next_request() {
    let set = get("/?case=cookies");

    assert_eq!(set.headers().get_all(SET_COOKIE).iter().count(), 2);
    assert_eq!(text(&get("/?case=read-cookies")), "1 2");
}

#[test]
fn an_invalid_header_is_dropped_alone() {
    let response = get("/?case=headers");

    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["x-good"], "1");
    assert!(!response.headers().contains_key("x-latin-1"));
    assert_eq!(text(&response), "Headers");
}

#[test]
fn a_header_after_the_output_is_reported_with_its_file() {
    let response = get("/?case=late-header");

    assert!(!response.headers().contains_key("x-late"));
    assert!(
        text(&response).contains("index.php:"),
        "{}",
        text(&response)
    );
}

#[test]
fn a_header_set_after_a_short_output_is_sent() {
    let response = get("/?case=early-output");

    assert_eq!(response.status(), 201);
    assert_eq!(response.headers()["x-after"], "1");
    assert_eq!(text(&response), "Created");
}

#[test]
fn an_xml_declaration_is_not_php() {
    let response = get("/?case=feed");

    assert_eq!(text(&response), r#"<?xml version="1.0"?><feed/>"#);
}

#[test]
fn a_fetch_gives_up_on_a_page_that_redirects_to_itself() {
    assert_eq!(get("/?case=loop").status(), 508);
}

#[test]
fn a_window_gives_up_on_a_page_that_redirects_to_itself() {
    let navigate = || {
        runtime().dispatch_blocking(
            Request::get("php://localhost/?case=loop")
                .header("Upgrade-Insecure-Requests", "1")
                .body(Vec::new())
                .unwrap(),
            RequestOrigin::Webview {
                window: "loop".to_owned(),
            },
        )
    };

    for _ in 0..20 {
        let page = navigate();
        assert_eq!(page.status(), 200);
        assert!(text(&page).contains("location.replace"), "{}", text(&page));
    }
    assert_eq!(navigate().status(), 508);
    assert_eq!(navigate().status(), 200, "The window may try again.");
}

#[test]
fn a_setting_reaches_php_as_it_is() {
    assert_eq!(text(&get("/?case=ini")), INI_VALUE);
}

#[test]
fn the_cookie_of_a_redirect_reaches_its_target() {
    assert_eq!(text(&get("/?case=login")), "abc");
}

#[test]
fn a_native_call_answers_or_says_why_it_failed() {
    assert_eq!(
        text(&get("/?case=native&method=window.get")),
        r#"{"result":{"window":"main"}}"#
    );
    assert_eq!(
        text(&get("/?case=native&method=fail")),
        r#"{"error":{"kind":"failed","message":"It failed."}}"#
    );
    assert_eq!(
        text(&get("/?case=native&method=missing")),
        r#"{"error":{"kind":"notFound","message":"There is no such thing."}}"#
    );
    assert!(
        text(&get("/?case=native&method=window.get&payload=%7B")).starts_with(
            r#"{"error":{"kind":"invalidPayload","message":"The payload of \"window.get\" is not valid JSON: "#
        )
    );

    let panicked = text(&get("/?case=native&method=panic"));
    assert!(
        panicked.starts_with("Error: ") && panicked.contains("It panicked."),
        "{panicked}"
    );
    assert_eq!(text(&get("/")), "Hello");
}

#[test]
fn a_page_cannot_pass_for_the_runtime() {
    let forged = send(
        Request::get("php://localhost/?case=origin")
            .header("Nativella-Internal", "1")
            .header("Nativella-Window", "settings")
            .body(Vec::new())
            .unwrap(),
    );
    assert_eq!(text(&forged), r#"[null,"main","1"]"#);

    let internal = runtime().dispatch_blocking(
        Request::get("php://localhost/?case=origin")
            .body(Vec::new())
            .unwrap(),
        RequestOrigin::Internal,
    );
    assert_eq!(text(&internal), r#"["1",null,null]"#);
}

#[test]
fn php_reads_forms_and_files() {
    let form = send(
        Request::post("php://localhost/?case=form")
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(b"title=Groceries&tags%5B%5D=milk&tags%5B%5D=eggs".to_vec())
            .unwrap(),
    );
    assert_eq!(
        text(&form),
        r#"{"post":{"title":"Groceries","tags":["milk","eggs"]},"file":null}"#
    );

    let multipart = send(
        Request::post("php://localhost/?case=form")
            .header(CONTENT_TYPE, "multipart/form-data; boundary=boundary")
            .body(b"--boundary\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nGroceries\r\n--boundary\r\nContent-Disposition: form-data; name=\"note\"; filename=\"note.txt\"\r\nContent-Type: text/plain\r\n\r\nMilk, eggs\r\n--boundary--\r\n".to_vec())
            .unwrap(),
    );
    assert_eq!(
        text(&multipart),
        r#"{"post":{"title":"Groceries"},"file":["note.txt","Milk, eggs"]}"#
    );
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
fn a_location_after_a_delete_is_followed_with_a_get() {
    let response = send(
        Request::delete("php://localhost/?case=deleted")
            .body(b"id=42".to_vec())
            .unwrap(),
    );

    assert_eq!(text(&response), "GET ");
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

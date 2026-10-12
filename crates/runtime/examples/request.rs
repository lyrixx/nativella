//! Serves requests through the runtime without any GUI, to test the SAPI.
//!
//! ```text
//! cargo run -p nativella-runtime --example request -- <project-dir> <METHOD> <URI> \
//!     [--repeat N] [--data BODY] [--cookie COOKIE] [--internal] [--server NAME=VALUE]... \
//!     [--ini NAME=VALUE]...
//! ```
//!
//! `--internal` sends the request as the host would, for a native event:
//!
//! ```text
//! cargo run -p nativella-runtime --example request -- <project-dir> POST /_nativella/event \
//!     --internal --data '{"name":"app.booted","payload":{}}'
//! ```
//!
//! `--server` adds a server variable, as the desktop shell adds
//! `NATIVELLA_DATA_DIR`.
//!
//! `--data` is sent as JSON when it starts with `{`, as a form otherwise.
//! `nativella_call()` answers as the desktop shell would when nothing happens
//! (no file picked, an empty clipboard…), and logs every call.

use std::env;
use std::path::PathBuf;

use http::Request;
use http::header::{CONTENT_TYPE, COOKIE};
use nativella_runtime::{PhpApp, PhpRuntime, RequestOrigin};
use serde_json::json;

const USAGE: &str = "Usage: request <project-dir> <METHOD> <URI> [--repeat N] [--data BODY] [--cookie COOKIE] [--internal] [--server NAME=VALUE]... [--ini NAME=VALUE]...";

fn main() {
    let mut args = env::args().skip(1);
    let project_dir = PathBuf::from(args.next().expect(USAGE));
    let method = args.next().expect(USAGE);
    let uri = args.next().expect(USAGE);

    let mut repeat = 1;
    let mut data = None;
    let mut cookie = None;
    let mut origin = RequestOrigin::Webview {
        window: "main".to_owned(),
    };
    let mut server_variables = Vec::new();
    let mut ini = Vec::new();
    while let Some(option) = args.next() {
        match option.as_str() {
            "--repeat" => {
                repeat = args
                    .next()
                    .expect(USAGE)
                    .parse()
                    .expect("--repeat takes a number.")
            }
            "--data" => data = Some(args.next().expect(USAGE)),
            "--cookie" => cookie = Some(args.next().expect(USAGE)),
            "--internal" => origin = RequestOrigin::Internal,
            "--ini" => {
                let setting = args.next().expect(USAGE);
                let (name, value) = setting.split_once('=').expect("--ini takes NAME=VALUE.");
                ini.push((name.to_owned(), value.to_owned()));
            }
            "--server" => {
                let variable = args.next().expect(USAGE);
                let (name, value) = variable
                    .split_once('=')
                    .expect("--server takes NAME=VALUE.");
                server_variables.push((name.to_owned(), value.to_owned()));
            }
            _ => panic!("Unknown option {option}. {USAGE}"),
        }
    }

    let mut app =
        PhpApp::from_project_dir(&project_dir).expect("The project has a public/index.php.");
    for (name, value) in server_variables {
        // The desktop shell creates its data directory.
        if name == "NATIVELLA_DATA_DIR" {
            std::fs::create_dir_all(&value).expect("The data directory can be created.");
        }
        app = app.with_server_variable(name, value);
    }
    for (name, value) in ini {
        app = app.with_ini(name, value);
    }
    let runtime = PhpRuntime::start(
        app,
        Box::new(|call| {
            eprintln!("[native] {} {}", call.method, call.payload);

            Ok(answer(call.method, &call.payload))
        }),
    );

    for iteration in 1..=repeat {
        let mut request = Request::builder()
            .method(method.as_str())
            .uri(format!("php://localhost{uri}"));
        if let Some(cookie) = &cookie {
            request = request.header(COOKIE, cookie);
        }
        if let Some(data) = &data {
            let content_type = if data.starts_with('{') {
                "application/json"
            } else {
                "application/x-www-form-urlencoded"
            };
            request = request.header(CONTENT_TYPE, content_type);
        }
        let request = request
            .body(data.clone().unwrap_or_default().into_bytes())
            .expect("The method and URI are valid.");

        let response = runtime.dispatch_blocking(request, origin.clone());

        if iteration < repeat {
            continue;
        }

        println!("{:?} {}", response.version(), response.status());
        for (name, value) in response.headers() {
            println!("{name}: {}", value.to_str().unwrap_or("<binary>"));
        }
        println!();
        println!("{}", String::from_utf8_lossy(response.body()));
    }
}

/// What the shell answers when nothing happens. Keep in sync with
/// `FakeClient::DEFAULTS` in packages/nativella/src/Client/FakeClient.php.
fn answer(method: &str, payload: &serde_json::Value) -> serde_json::Value {
    let window = |label: &serde_json::Value| {
        json!({
            "label": label.as_str().unwrap_or("main"), "title": "Nativella", "url": "php://localhost/",
            "width": 1024.0, "height": 768.0, "x": 0.0, "y": 0.0, "focused": true,
            "maximized": false, "minimized": false, "fullscreen": false, "visible": true,
        })
    };

    match method {
        "app.info" => {
            json!({ "name": "Nativella", "version": "0.1.0", "identifier": "dev.nativella.app", "nativellaVersion": env!("CARGO_PKG_VERSION") })
        }
        "app.isAutostartEnabled" => json!({ "enabled": false }),
        "biometrics.authenticate" => json!({ "authenticated": false }),
        "biometrics.isAvailable" => json!({ "available": false }),
        "childProcess.all" => json!({ "processes": [] }),
        "childProcess.start" => json!({ "pid": 1 }),
        "app.path" => {
            json!({ "path": format!("/home/nativella/{}", payload["directory"].as_str().unwrap_or_default()) })
        }
        "clipboard.readImage" => json!({ "png": null }),
        "clipboard.readText" => json!({ "text": null }),
        "dialog.ask" => json!({ "answer": false }),
        "dialog.confirm" => json!({ "confirmed": false }),
        "dialog.openDirectory" | "dialog.openFile" | "dialog.saveFile" => json!({ "path": null }),
        "dialog.openFiles" => json!({ "paths": [] }),
        "geolocation.currentPosition" => json!({ "position": null }),
        "globalShortcut.isRegistered" => json!({ "registered": false }),
        "power.status" => json!({ "onBattery": false, "battery": null }),
        "screen.all" => json!({ "screens": [] }),
        "screen.cursorPosition" => json!({ "x": 0, "y": 0 }),
        "screen.primary" => json!({ "screen": null }),
        "secrets.get" => json!({ "value": null }),
        "settings.all" => json!({ "settings": {} }),
        "settings.has" => json!({ "has": false }),
        "system.info" => {
            json!({ "os": std::env::consts::OS, "family": std::env::consts::FAMILY, "arch": std::env::consts::ARCH, "version": "", "hostname": null, "locale": null })
        }
        "system.theme" => json!({ "theme": "light" }),
        "window.all" => json!({ "windows": [] }),
        "window.get" => json!({ "window": window(&payload["window"]) }),
        "window.open" => json!({ "window": window(&payload["label"]) }),
        _ => json!({}),
    }
}

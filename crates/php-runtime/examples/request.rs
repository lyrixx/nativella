//! Serves requests through the runtime without any GUI, to test the SAPI.
//!
//! ```text
//! cargo run -p php-runtime --example request -- <project-dir> <METHOD> <URI> \
//!     [--repeat N] [--data BODY] [--cookie COOKIE] [--internal]
//! ```
//!
//! `--data` is sent as JSON when it starts with `{`, as a form otherwise.
//! `native_call()` answers with an echo of its arguments.

use std::env;
use std::path::PathBuf;

use http::Request;
use http::header::{CONTENT_TYPE, COOKIE};
use php_runtime::{PhpApp, PhpRuntime, RequestOrigin};
use serde_json::json;

const USAGE: &str = "Usage: request <project-dir> <METHOD> <URI> [--repeat N] [--data BODY] [--cookie COOKIE] [--internal]";

fn main() {
    let mut args = env::args().skip(1);
    let project_dir = PathBuf::from(args.next().expect(USAGE));
    let method = args.next().expect(USAGE);
    let uri = args.next().expect(USAGE);

    let mut repeat = 1;
    let mut data = None;
    let mut cookie = None;
    let mut origin = RequestOrigin::Webview;
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
            _ => panic!("Unknown option {option}. {USAGE}"),
        }
    }

    let app = PhpApp::from_project_dir(&project_dir).expect("The project has a public/index.php.");
    let runtime = PhpRuntime::start(
        app,
        Box::new(|method, payload| Ok(json!({ "method": method, "payload": payload }))),
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

        let response = runtime.dispatch_blocking(request, origin);

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

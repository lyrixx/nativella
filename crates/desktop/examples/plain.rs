//! The same window without PHP, to tell PHP issues from WebKitGTK ones.

use tauri::http::Response;
use tauri::{WebviewUrl, WebviewWindowBuilder};

fn main() {
    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("php", |_context, _request, responder| {
            responder.respond(Response::new(b"<h1>Plain Tauri, no PHP</h1>".to_vec()));
        })
        .setup(|app| {
            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::CustomProtocol("php://localhost/".parse()?),
            )
            .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("The Tauri application runs.");
}

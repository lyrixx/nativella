//! The desktop shell: a Tauri window whose requests are served in-process by
//! PHP, through the `php://` scheme instead of a local HTTP server.
//!
//! [`run`] lets the binary and the examples share the same application.

mod native;

use std::env;
use std::path::PathBuf;

use php_runtime::{PhpApp, PhpRuntime, RequestOrigin};
use serde_json::{Value, json};
use tauri::http::Request;
use tauri::http::header::CONTENT_TYPE;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Builder, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry};

const MAIN_WINDOW: &str = "main";

/// Where the webview finds the app. Tauri exposes the `php` scheme registered
/// below as `php://localhost/`, except on Windows and Android where custom
/// schemes are served from `http://php.localhost/`.
#[cfg(not(any(windows, target_os = "android")))]
const APP_URL: &str = "php://localhost/";
#[cfg(any(windows, target_os = "android"))]
const APP_URL: &str = "http://php.localhost/";

/// Runs the application. `customize` can add to the Tauri builder, as the
/// smoke test does.
pub fn run(customize: impl FnOnce(Builder<Wry>) -> Builder<Wry>) {
    // WebKitGTK 2.52 (Ubuntu 26.04) segfaults in the AT-SPI bridge
    // (spi_register_object_to_path) as soon as a page loads, with or without
    // PHP: see examples/plain.rs. Disabling the bridge avoids it, at the cost
    // of screen reader support.
    #[cfg(target_os = "linux")]
    if env::var_os("NO_AT_BRIDGE").is_none() {
        // SAFETY: no other thread exists yet.
        unsafe { env::set_var("NO_AT_BRIDGE", "1") };
    }

    let project_dir = env::var_os("NATIVE_PHP_APP").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../app"),
        PathBuf::from,
    );
    let php_app = PhpApp::from_project_dir(&project_dir).unwrap_or_else(|error| {
        panic!(
            "Cannot serve the PHP app in {}: {error}",
            project_dir.display()
        )
    });

    let builder = Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .register_asynchronous_uri_scheme_protocol("php", |context, request, responder| {
            let runtime = context.app_handle().state::<PhpRuntime>();
            runtime.dispatch(request, RequestOrigin::Webview, move |response| {
                responder.respond(response)
            });
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            let runtime = PhpRuntime::start(
                php_app,
                Box::new(move |method, payload| native::call(&handle, method, &payload)),
            );
            app.manage(runtime);

            let demo = Submenu::with_items(
                app,
                "Demo",
                true,
                &[
                    &MenuItem::with_id(app, "ping", "Ping PHP", true, None::<&str>)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?;

            WebviewWindowBuilder::new(
                app,
                MAIN_WINDOW,
                WebviewUrl::CustomProtocol(APP_URL.parse()?),
            )
            .title("native-php")
            .inner_size(900.0, 760.0)
            .menu(Menu::with_items(app, &[&demo])?)
            .build()?;

            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id() == "ping" {
                send_event(app, "menu.ping", json!({}));
            }
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(focused) = event {
                let name = if *focused {
                    "window.focused"
                } else {
                    "window.blurred"
                };
                send_event(
                    window.app_handle(),
                    name,
                    json!({ "window": window.label() }),
                );
            }
        });

    customize(builder)
        .run(tauri::generate_context!())
        .expect("The Tauri application runs.");
}

/// Notifies PHP of a native event, as an internal request that the app routes
/// like any other (see App\Controller\NativeEventController).
fn send_event(app: &AppHandle, name: &str, payload: Value) {
    let body = json!({ "name": name, "payload": payload })
        .to_string()
        .into_bytes();
    let request = Request::post(format!("{APP_URL}_native/event"))
        .header(CONTENT_TYPE, "application/json")
        .body(body)
        .expect("The event URL and header are valid.");

    let name = name.to_owned();
    app.state::<PhpRuntime>()
        .dispatch(request, RequestOrigin::Internal, move |response| {
            if !response.status().is_success() {
                eprintln!(
                    "[event] PHP answered {} to the \"{name}\" event.",
                    response.status()
                );
            }
        });
}

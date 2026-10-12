//! `window.*`: open, inspect and drive windows.

use serde::Deserialize;
use serde_json::{Map, Value, json};
use tauri::webview::Url;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use tauri_plugin_opener::OpenerExt;

use super::{Call, State, failed};
use crate::APP_URL;
use crate::MAIN_WINDOW;

/// The options of `window.open`, and of the default window in composer.json.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(
    mobile,
    expect(
        dead_code,
        reason = "Mobile windows fill the screen: they ignore most options."
    )
)]
pub(crate) struct WindowOptions {
    url: Option<String>,
    title: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
    min_width: Option<f64>,
    min_height: Option<f64>,
    max_width: Option<f64>,
    max_height: Option<f64>,
    x: Option<f64>,
    y: Option<f64>,
    center: Option<bool>,
    resizable: Option<bool>,
    maximizable: Option<bool>,
    minimizable: Option<bool>,
    closable: Option<bool>,
    maximized: Option<bool>,
    fullscreen: Option<bool>,
    /// `false` for a frameless window.
    decorations: Option<bool>,
    always_on_top: Option<bool>,
    focused: Option<bool>,
    visible: Option<bool>,
    skip_taskbar: Option<bool>,
}

/// The options are checked by `open()`: serde cannot deny unknown fields
/// next to a flattened map.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Open {
    label: String,
    #[serde(flatten)]
    options: Map<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    window: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetTitle {
    window: Option<String>,
    title: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetSize {
    window: Option<String>,
    width: f64,
    height: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetPosition {
    window: Option<String>,
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetFlag {
    window: Option<String>,
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(mobile, expect(dead_code, reason = "Phones show no progress bar."))]
struct SetProgress {
    window: Option<String>,
    /// None removes the bar.
    percent: Option<u8>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(
    not(target_os = "linux"),
    expect(dead_code, reason = "Only Linux prints to PDF yet.")
)]
struct PrintToPdf {
    window: Option<String>,
    path: std::path::PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Navigate {
    window: Option<String>,
    url: String,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "open" => call.payload::<Open>().and_then(|payload| {
            let window = open(call.app, &payload.label, payload.options)?;
            Ok(json!({ "window": info(&window) }))
        }),
        "close" => with_target(call, |window| window.close()),
        "all" => {
            let mut windows: Vec<Value> = call.app.webview_windows().values().map(info).collect();
            windows.sort_by(|a, b| a["label"].as_str().cmp(&b["label"].as_str()));
            Ok(json!({ "windows": windows }))
        }
        "get" => call
            .payload::<Target>()
            .and_then(|target| call.target_window(target.window.as_deref()))
            .map(|window| json!({ "window": info(&window) })),
        "setTitle" => call.payload::<SetTitle>().and_then(|payload| {
            let window = call.target_window(payload.window.as_deref())?;
            window.set_title(&payload.title).map_err(failed)?;
            Ok(json!({}))
        }),
        "setSize" => call.payload::<SetSize>().and_then(|payload| {
            let window = call.target_window(payload.window.as_deref())?;
            window
                .set_size(LogicalSize::new(payload.width, payload.height))
                .map_err(failed)?;
            Ok(json!({}))
        }),
        "setPosition" => call.payload::<SetPosition>().and_then(|payload| {
            let window = call.target_window(payload.window.as_deref())?;
            window
                .set_position(LogicalPosition::new(payload.x, payload.y))
                .map_err(failed)?;
            Ok(json!({}))
        }),
        #[cfg(desktop)]
        "setAlwaysOnTop" => with_flag(call, |window, enabled| window.set_always_on_top(enabled)),
        #[cfg(desktop)]
        "setFullscreen" => with_flag(call, |window, enabled| window.set_fullscreen(enabled)),
        // In the taskbar or the dock, where the system shows one.
        #[cfg(desktop)]
        "setProgress" => call.payload::<SetProgress>().and_then(|payload| {
            let window = call.target_window(payload.window.as_deref())?;
            let state = match payload.percent {
                Some(percent) => tauri::window::ProgressBarState {
                    status: Some(tauri::window::ProgressBarStatus::Normal),
                    progress: Some(u64::from(percent.min(100))),
                },
                None => tauri::window::ProgressBarState {
                    status: Some(tauri::window::ProgressBarStatus::None),
                    progress: None,
                },
            };
            window.set_progress_bar(state).map_err(failed)?;
            Ok(json!({}))
        }),
        "setResizable" => with_flag(call, |window, enabled| window.set_resizable(enabled)),
        #[cfg(desktop)]
        "center" => with_target(call, |window| window.center()),
        #[cfg(desktop)]
        "minimize" => with_target(call, |window| window.minimize()),
        #[cfg(desktop)]
        "maximize" => with_target(call, |window| window.maximize()),
        #[cfg(desktop)]
        "unmaximize" => with_target(call, |window| window.unmaximize()),
        "show" => with_target(call, |window| window.show()),
        "hide" => with_target(call, |window| window.hide()),
        "focus" => with_target(call, |window| window.set_focus()),
        "reload" => with_target(call, |window| window.reload()),
        // The system print dialog, for the page of the window.
        #[cfg(desktop)]
        "print" => with_target(call, |window| window.print()),
        #[cfg(target_os = "linux")]
        "printToPdf" => call.payload::<PrintToPdf>().and_then(|payload| {
            let window = call.target_window(payload.window.as_deref())?;
            super::pdf::print(&window, &payload.path)?;
            Ok(json!({}))
        }),
        #[cfg(not(target_os = "linux"))]
        "printToPdf" => Err("Printing to PDF only works on Linux, for now.".to_owned()),
        "navigate" => call.payload::<Navigate>().and_then(|payload| {
            let window = call.target_window(payload.window.as_deref())?;
            window.navigate(url(&payload.url)?).map_err(failed)?;
            Ok(json!({}))
        }),
        // Release builds have no developer tools: shipping them would rely on
        // private APIs on macOS.
        #[cfg(debug_assertions)]
        "openDevtools" => with_target(call, |window| {
            window.open_devtools();
            Ok(())
        }),
        #[cfg(not(debug_assertions))]
        "openDevtools" => Err("Developer tools only exist in debug builds.".to_owned()),
        #[cfg(mobile)]
        "center" | "maximize" | "minimize" | "print" | "setAlwaysOnTop" | "setFullscreen"
        | "setProgress" | "unmaximize" => {
            Err(format!("\"{}\" is not available on mobile.", call.method))
        }
        _ => return None,
    })
}

/// Opens a window with options in their JSON form, as `window.open` and the
/// default window of composer.json give them.
///
/// The main window starts from the options of composer.json, which `options`
/// complete: PHP can open it again without knowing them.
pub(crate) fn open(
    app: &AppHandle,
    label: &str,
    options: Map<String, Value>,
) -> Result<WebviewWindow, String> {
    let options = if label == MAIN_WINDOW {
        let mut merged = app.state::<State>().default_window().clone();
        merged.extend(options);
        merged
    } else {
        options
    };

    let options: WindowOptions = serde_json::from_value(Value::Object(options))
        .map_err(|error| format!("The window options are invalid: {error}."))?;

    if let Some(window) = app.get_webview_window(label) {
        window.set_focus().map_err(failed)?;

        return Ok(window);
    }

    let url = url(options.url.as_deref().unwrap_or("/"))?;
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::CustomProtocol(url))
        .title(
            options
                .title
                .clone()
                .unwrap_or_else(|| app.package_info().name.clone()),
        )
        .inner_size(
            options.width.unwrap_or(1024.0),
            options.height.unwrap_or(768.0),
        );

    if let (Some(width), Some(height)) = (options.min_width, options.min_height) {
        builder = builder.min_inner_size(width, height);
    }
    if let (Some(width), Some(height)) = (options.max_width, options.max_height) {
        builder = builder.max_inner_size(width, height);
    }
    if let (Some(x), Some(y)) = (options.x, options.y) {
        builder = builder.position(x, y);
    }
    if let Some(resizable) = options.resizable {
        builder = builder.resizable(resizable);
    }
    if let Some(visible) = options.visible {
        builder = builder.visible(visible);
    }

    builder = with_desktop_options(builder, &options);
    #[cfg(target_os = "android")]
    {
        builder = builder.initialization_script(crate::request_bodies::SCRIPT);
    }

    let handle = app.clone();
    let label = label.to_owned();
    builder = builder.on_navigation(move |url| allow_navigation(&handle, &label, url));

    builder.build().map_err(failed)
}

/// What only desktop windows have: mobile ones fill the screen.
#[cfg(desktop)]
fn with_desktop_options<'a>(
    mut builder: WebviewWindowBuilder<'a, tauri::Wry, AppHandle>,
    options: &WindowOptions,
) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    if options.center.unwrap_or(false) {
        builder = builder.center();
    }
    if let Some(maximizable) = options.maximizable {
        builder = builder.maximizable(maximizable);
    }
    if let Some(minimizable) = options.minimizable {
        builder = builder.minimizable(minimizable);
    }
    if let Some(closable) = options.closable {
        builder = builder.closable(closable);
    }
    if let Some(maximized) = options.maximized {
        builder = builder.maximized(maximized);
    }
    if let Some(fullscreen) = options.fullscreen {
        builder = builder.fullscreen(fullscreen);
    }
    if let Some(decorations) = options.decorations {
        builder = builder.decorations(decorations);
    }
    if let Some(always_on_top) = options.always_on_top {
        builder = builder.always_on_top(always_on_top);
    }
    if let Some(focused) = options.focused {
        builder = builder.focused(focused);
    }
    if let Some(skip_taskbar) = options.skip_taskbar {
        builder = builder.skip_taskbar(skip_taskbar);
    }

    builder
}

#[cfg(mobile)]
fn with_desktop_options<'a>(
    builder: WebviewWindowBuilder<'a, tauri::Wry, AppHandle>,
    _options: &WindowOptions,
) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    builder
}

/// Pages of the app stay in the window. Absolute URLs PHP built for
/// `http://localhost/` (it believes it is served there) are brought back to
/// the app, and other websites open in the browser.
fn allow_navigation(app: &AppHandle, label: &str, url: &Url) -> bool {
    let base = Url::parse(APP_URL).expect("The app URL is valid.");
    if url.scheme() == base.scheme() && url.host() == base.host() {
        return true;
    }

    match url.scheme() {
        "http" | "https" if url.host_str() == Some("localhost") => {
            let mut target = base;
            target.set_path(url.path());
            target.set_query(url.query());
            target.set_fragment(url.fragment());

            // Not from within the navigation callback of the webview.
            if let Some(window) = app.get_webview_window(label) {
                std::thread::spawn(move || {
                    if let Err(error) = window.navigate(target) {
                        eprintln!("[window] Could not open the app page: {error}.");
                    }
                });
            }

            false
        }
        "http" | "https" | "mailto" | "tel" => {
            if let Err(error) = app.opener().open_url(url.as_str(), None::<&str>) {
                eprintln!("[window] Could not open {url} in the browser: {error}.");
            }

            false
        }
        // about:blank, data: and blob: URLs.
        _ => true,
    }
}

/// A path of the app ("/settings") or an absolute URL.
pub(crate) fn url(target: &str) -> Result<Url, String> {
    let base = Url::parse(APP_URL).expect("The app URL is valid.");

    base.join(target)
        .map_err(|error| format!("\"{target}\" is not a valid URL: {error}."))
}

pub(crate) fn info(window: &WebviewWindow) -> Value {
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window
        .inner_size()
        .map(|size| size.to_logical::<f64>(scale))
        .ok();
    let position = window
        .outer_position()
        .map(|position| position.to_logical::<f64>(scale))
        .ok();

    json!({
        "label": window.label(),
        "title": window.title().ok(),
        "url": window.url().ok().map(|url| url.to_string()),
        "width": size.map(|size| size.width),
        "height": size.map(|size| size.height),
        "x": position.map(|position| position.x),
        "y": position.map(|position| position.y),
        "focused": window.is_focused().unwrap_or(false),
        "maximized": window.is_maximized().unwrap_or(false),
        "minimized": window.is_minimized().unwrap_or(false),
        "fullscreen": window.is_fullscreen().unwrap_or(false),
        "visible": window.is_visible().unwrap_or(false),
    })
}

fn with_target(
    call: &Call<'_>,
    action: impl FnOnce(&WebviewWindow) -> tauri::Result<()>,
) -> Result<Value, String> {
    let target: Target = call.payload()?;
    let window = call.target_window(target.window.as_deref())?;
    action(&window).map_err(failed)?;

    Ok(json!({}))
}

fn with_flag(
    call: &Call<'_>,
    action: impl FnOnce(&WebviewWindow, bool) -> tauri::Result<()>,
) -> Result<Value, String> {
    let flag: SetFlag = call.payload()?;
    let window = call.target_window(flag.window.as_deref())?;
    action(&window, flag.enabled).map_err(failed)?;

    Ok(json!({}))
}

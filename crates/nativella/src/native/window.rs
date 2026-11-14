//! `window.*`: open, inspect and drive windows.

use serde::Deserialize;
use serde_json::{Map, Value, json};
use tauri::webview::{NewWindowResponse, Url};
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use super::{Call, NativeError, State, failed, invalid_payload};
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
    window: String,
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
    not(any(target_os = "linux", target_os = "macos")),
    expect(dead_code, reason = "Only Linux and macOS print to PDF yet.")
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

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, NativeError>> {
    Some(match method {
        "open" => call.payload::<Open>().and_then(|payload| {
            let window = open(call.app, &payload.window, payload.options)?;
            // It may be the window of the tray icon, which gets no menu bar.
            #[cfg(all(desktop, not(target_os = "macos")))]
            super::tray::remove_menu_bar(call.app);
            Ok(json!({ "window": info(&window) }))
        }),
        "close" => with_target(call, |window| window.close()),
        "all" => {
            let mut windows: Vec<Value> = call.app.webview_windows().values().map(info).collect();
            windows.sort_by(|a, b| a["window"].as_str().cmp(&b["window"].as_str()));
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
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        "printToPdf" => call.payload::<PrintToPdf>().and_then(|payload| {
            let window = call.target_window(payload.window.as_deref())?;
            super::pdf::print(&window, &payload.path)?;
            Ok(json!({}))
        }),
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        "printToPdf" => Err(super::unavailable(
            "Printing to PDF only works on Linux and macOS, for now.",
        )),
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
        "openDevtools" => Err(super::unavailable(
            "Developer tools only exist in debug builds.",
        )),
        #[cfg(mobile)]
        "center" | "maximize" | "minimize" | "print" | "setAlwaysOnTop" | "setFullscreen"
        | "setProgress" | "unmaximize" => Err(super::unavailable(format!(
            "\"{}\" is not available on mobile.",
            call.method
        ))),
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
) -> Result<WebviewWindow, NativeError> {
    let options = if label == MAIN_WINDOW {
        let mut merged = app.state::<State>().default_window().clone();
        merged.extend(options);
        merged
    } else {
        options
    };

    let options: WindowOptions = serde_json::from_value(Value::Object(options))
        .map_err(|error| invalid_payload(format!("The window options are invalid: {error}.")))?;

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
    #[cfg(not(any(windows, target_os = "android")))]
    {
        builder = builder.initialization_script(crate::absolute_urls::SCRIPT);
    }
    #[cfg(target_os = "android")]
    {
        builder = builder.initialization_script(crate::request_bodies::SCRIPT);
    }

    let handle = app.clone();
    let navigating = label.to_owned();
    builder = builder.on_navigation(move |url| allow_navigation(&handle, &navigating, url));
    // `target="_blank"` and `window.open()`: Linux does nothing without it.
    let handle = app.clone();
    let opening = label.to_owned();
    builder = builder.on_new_window(move |url, _| {
        open_elsewhere(&handle, &opening, &url);
        NewWindowResponse::Deny
    });

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
        .initialization_script(crate::downloads::SCRIPT)
        .on_download(crate::downloads::on_download)
}

#[cfg(mobile)]
fn with_desktop_options<'a>(
    builder: WebviewWindowBuilder<'a, tauri::Wry, AppHandle>,
    _options: &WindowOptions,
) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    builder
}

/// Where a navigation goes.
#[derive(Debug, PartialEq)]
enum Destination {
    /// A page of the app.
    App,
    /// The page of the app an absolute URL of PHP means: PHP believes it is
    /// served from `http://localhost/`.
    Rebased(Url),
    /// What a page makes itself: `about:blank`, `data:` and `blob:` URLs.
    Inline,
    /// Another website, a mail or a phone number: the browser, or the
    /// application of the system for it.
    System,
    /// `file:`, and schemes nothing here knows.
    Refused,
}

impl Destination {
    fn of(url: &Url) -> Self {
        let base = Url::parse(APP_URL).expect("The app URL is valid.");
        if url.scheme() == base.scheme() && url.host() == base.host() && url.port() == base.port() {
            return Self::App;
        }

        match url.scheme() {
            // As absolute_urls.js and the runtime have it: `http://localhost:8025`
            // is another server, Mailpit or a child process.
            "http" if url.host_str() == Some("localhost") && url.port().is_none() => {
                let mut target = base;
                target.set_path(url.path());
                target.set_query(url.query());
                target.set_fragment(url.fragment());

                Self::Rebased(target)
            }
            "http" | "https" | "mailto" | "tel" => Self::System,
            "about" | "blob" | "data" => Self::Inline,
            _ => Self::Refused,
        }
    }
}

/// Pages of the app stay in the window, absolute URLs of PHP are brought
/// back to the app, and other websites open in the browser.
fn allow_navigation(app: &AppHandle, label: &str, url: &Url) -> bool {
    match Destination::of(url) {
        Destination::App | Destination::Inline => true,
        Destination::Rebased(target) => {
            navigate_later(app, label, target);
            false
        }
        Destination::System => {
            open_in_system(app, url);
            false
        }
        Destination::Refused => {
            eprintln!("[window] {url} is not opened: only pages of the app and websites are.");
            false
        }
    }
}

/// What a link to a new window, or `window.open()`, opens: the window
/// itself for a page of the app, the browser for a website. Nativella opens
/// no window of its own: PHP does, with `Window::open()`.
fn open_elsewhere(app: &AppHandle, label: &str, url: &Url) {
    match Destination::of(url) {
        Destination::App => navigate_later(app, label, url.clone()),
        Destination::Rebased(target) => navigate_later(app, label, target),
        Destination::System => open_in_system(app, url),
        Destination::Inline => {
            eprintln!(
                "[window] A {}: URL is not opened: a page opens no window.",
                url.scheme()
            );
        }
        Destination::Refused => eprintln!("[window] {url} is not opened: a page opens no window."),
    }
}

/// Not from within a callback of the webview, which navigating would enter
/// again.
fn navigate_later(app: &AppHandle, label: &str, target: Url) {
    if let Some(window) = app.get_webview_window(label) {
        std::thread::spawn(move || {
            if let Err(error) = window.navigate(target) {
                eprintln!("[window] Could not open the app page: {error}.");
            }
        });
    }
}

fn open_in_system(app: &AppHandle, url: &Url) {
    if let Err(error) = super::shell::open_url(app, url.as_str()) {
        eprintln!("[window] Could not open {url} in the browser: {error}.");
    }
}

/// A path of the app ("/settings") or an absolute URL.
pub(crate) fn url(target: &str) -> Result<Url, NativeError> {
    let base = Url::parse(APP_URL).expect("The app URL is valid.");

    base.join(target)
        .map_err(|error| invalid_payload(format!("\"{target}\" is not a valid URL: {error}.")))
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
    let fullscreen = window.is_fullscreen().unwrap_or(false);

    json!({
        "window": window.label(),
        "title": window.title().ok(),
        "url": window.url().ok().map(|url| url.to_string()),
        "width": size.map(|size| size.width),
        "height": size.map(|size| size.height),
        "x": position.map(|position| position.x),
        "y": position.map(|position| position.y),
        "focused": window.is_focused().unwrap_or(false),
        // macOS says a window in fullscreen is maximized too: it fills the
        // screen.
        "maximized": !fullscreen && window.is_maximized().unwrap_or(false),
        "minimized": window.is_minimized().unwrap_or(false),
        "fullscreen": fullscreen,
        "visible": window.is_visible().unwrap_or(false),
    })
}

fn with_target(
    call: &Call<'_>,
    action: impl FnOnce(&WebviewWindow) -> tauri::Result<()>,
) -> Result<Value, NativeError> {
    let target: Target = call.payload()?;
    let window = call.target_window(target.window.as_deref())?;
    action(&window).map_err(failed)?;

    Ok(json!({}))
}

fn with_flag(
    call: &Call<'_>,
    action: impl FnOnce(&WebviewWindow, bool) -> tauri::Result<()>,
) -> Result<Value, NativeError> {
    let flag: SetFlag = call.payload()?;
    let window = call.target_window(flag.window.as_deref())?;
    action(&window, flag.enabled).map_err(failed)?;

    Ok(json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn destination(url: &str) -> Destination {
        Destination::of(&Url::parse(url).unwrap())
    }

    #[test]
    fn pages_of_the_app_stay_in_the_window() {
        assert_eq!(destination(APP_URL), Destination::App);
        assert_eq!(
            destination(url("/notes/42?tab=edit").unwrap().as_str()),
            Destination::App
        );
    }

    #[test]
    fn absolute_urls_of_php_are_brought_back_to_the_app() {
        assert_eq!(
            destination("http://localhost/notes/42?tab=edit#title"),
            Destination::Rebased(url("/notes/42?tab=edit#title").unwrap())
        );
        assert_eq!(
            destination("http://localhost"),
            Destination::Rebased(url("/").unwrap())
        );
    }

    #[test]
    fn other_servers_of_this_machine_open_in_the_browser() {
        for other in [
            "http://localhost:8025/",
            "https://localhost/",
            "http://127.0.0.1/",
            "http://localhost.example/",
        ] {
            assert_eq!(destination(other), Destination::System, "{other}");
        }
    }

    #[test]
    fn websites_mails_and_phone_numbers_open_in_the_system() {
        for external in [
            "https://example.com/",
            "http://example.com/",
            "mailto:me@example.com",
            "tel:+33123456789",
        ] {
            assert_eq!(destination(external), Destination::System, "{external}");
        }
    }

    #[test]
    fn what_a_page_makes_itself_stays_in_the_window() {
        for inline in [
            "about:blank",
            "data:text/html,<p>Hello</p>",
            "blob:php://localhost/2f6c9a3e",
        ] {
            assert_eq!(destination(inline), Destination::Inline, "{inline}");
        }
    }

    #[test]
    fn files_and_unknown_schemes_are_refused() {
        for refused in [
            "file:///home/me/.ssh/config",
            "ftp://example.com/",
            "javascript:alert(1)",
            "notes://open/42",
        ] {
            assert_eq!(destination(refused), Destination::Refused, "{refused}");
        }
    }
}

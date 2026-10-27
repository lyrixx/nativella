//! The shell: Tauri windows whose requests are served in-process by PHP,
//! through the `php://` scheme instead of a local HTTP server.
//!
//! [`run`] lets the desktop binary and the examples share the same
//! application; mobile platforms start from the entry point below, with the
//! project their package carries (see embedded.rs).

#[cfg(not(any(windows, target_os = "android")))]
mod absolute_urls;
#[cfg(desktop)]
pub mod apk;
#[cfg(desktop)]
mod before_unload;
#[cfg(any(test, target_os = "android"))]
mod deadline;
pub mod embedded;
mod events;
#[cfg(desktop)]
pub mod ios;
#[cfg(target_os = "android")]
mod logcat;
mod native;
pub mod notices;
pub mod project;
#[cfg(desktop)]
mod reload;
#[cfg(target_os = "android")]
mod request_bodies;
mod secret;
mod webview_permissions;
#[cfg(desktop)]
mod zip;

use std::env;
use std::fs;
use std::path::PathBuf;

use nativella_runtime::{PhpApp, PhpRuntime, RequestOrigin};
use serde_json::json;
use tauri::{App, Builder, Manager, RunEvent, WindowEvent, Wry};

use crate::native::State;
pub use crate::project::{Manifest, Project};

/// Where the webview finds the app. Tauri exposes the `php` scheme registered
/// below as `php://localhost/`, except on Windows and Android where custom
/// schemes are served from `http://php.localhost/`.
#[cfg(not(any(windows, target_os = "android")))]
pub(crate) const APP_URL: &str = "php://localhost/";
#[cfg(any(windows, target_os = "android"))]
pub(crate) const APP_URL: &str = "http://php.localhost/";

/// The label of the window opened when PHP opens none at boot.
pub(crate) const MAIN_WINDOW: &str = "main";

/// Where the PHP project comes from.
enum Source {
    /// A directory: `nativella run`, or a standalone executable once extracted.
    Project(Project),
    /// The bundle of a phone application, extracted at startup.
    #[cfg(mobile)]
    Packaged(embedded::Packaged),
}

impl Source {
    fn manifest(&self) -> &Manifest {
        match self {
            Self::Project(project) => &project.manifest,
            #[cfg(mobile)]
            Self::Packaged(bundle) => bundle.manifest(),
        }
    }

    #[cfg_attr(
        desktop,
        expect(unused_variables, reason = "Only mobile builds extract a bundle.")
    )]
    fn into_project(self, app: &App) -> Result<Project, Box<dyn std::error::Error>> {
        Ok(match self {
            Self::Project(project) => project,
            #[cfg(mobile)]
            Self::Packaged(bundle) => bundle.extract(&app.path().app_cache_dir()?)?,
        })
    }
}

/// Runs `project` as a desktop application. `customize` can add to the Tauri
/// builder, as the smoke test does.
pub fn run(
    project: Project,
    customize: impl FnOnce(Builder<Wry>) -> Builder<Wry>,
) -> Result<(), String> {
    start(Source::Project(project), customize)
}

/// Android and iOS start here, with the bundle their package carries.
#[cfg(mobile)]
#[tauri::mobile_entry_point]
fn mobile_main() {
    #[cfg(target_os = "android")]
    logcat::redirect_stderr();

    let result = match embedded::packaged() {
        Ok(Some(bundle)) => start(Source::Packaged(bundle), |builder| builder),
        #[cfg(target_os = "android")]
        Ok(None) => {
            Err("This APK carries no PHP project: make it with nativella android.".to_owned())
        }
        #[cfg(target_os = "ios")]
        Ok(None) => Err("This app carries no PHP project: make it with nativella ios.".to_owned()),
        Err(error) => Err(format!("The bundled PHP project is unreadable: {error}")),
    };

    if let Err(error) = result {
        eprintln!("[nativella] {error}");
    }
}

fn start(
    source: Source,
    customize: impl FnOnce(Builder<Wry>) -> Builder<Wry>,
) -> Result<(), String> {
    // WebKitGTK 2.52 (Ubuntu 26.04) segfaults in the AT-SPI bridge
    // (spi_register_object_to_path) as soon as a page loads, with or without
    // PHP: see examples/plain.rs. Disabling the bridge avoids it, at the cost
    // of screen reader support.
    #[cfg(target_os = "linux")]
    if env::var_os("NO_AT_BRIDGE").is_none() {
        // SAFETY: no other thread exists yet.
        unsafe { env::set_var("NO_AT_BRIDGE", "1") };
    }

    let mut context = tauri::generate_context!();
    let manifest = source.manifest().clone();
    let config = context.config_mut();
    config.identifier.clone_from(&manifest.identifier);
    config.product_name = Some(manifest.name.clone());
    config.version = Some(manifest.version.clone());
    let package_info = context.package_info_mut();
    package_info.name.clone_from(&manifest.name);
    if let Ok(version) = manifest.version.parse() {
        package_info.version = version;
    }
    if !manifest.deep_links.is_empty() {
        context.config_mut().plugins.0.insert(
            "deep-link".to_owned(),
            json!({ "desktop": { "schemes": manifest.deep_links } }),
        );
    }
    if let Source::Project(project) = &source
        && let Some(icon) = project.icon()
    {
        let icon = tauri::image::Image::from_path(&icon)
            .map_err(|error| format!("Cannot load the icon {}: {error}", icon.display()))?;
        context.set_default_window_icon(Some(icon));
    }

    // What the pages may ask of the webview, getUserMedia() for a start.
    let asked = manifest.permissions.clone();
    let builder = Builder::default()
        .on_permission_request(move |_, kind| webview_permissions::respond(&asked, kind));

    #[cfg(desktop)]
    let builder = builder
        // First, so that a second instance exits before doing anything else.
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            // A second instance may come before the first one has started PHP.
            if app.try_state::<PhpRuntime>().is_none() {
                return;
            }
            // Its windows may all be closed, when it lives in the tray.
            match app.webview_windows().values().next() {
                Some(window) => {
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
                None => {
                    if let Err(error) = native::window::open(app, MAIN_WINDOW, Default::default()) {
                        eprintln!("[nativella] Could not open the main window: {error}");
                    }
                }
            }
            events::send(app, "app.reopened", json!({ "args": args, "cwd": cwd }));
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(native::global_shortcut::on_shortcut)
                .build(),
        )
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .on_menu_event(native::menu::on_menu_event);

    // Reads the files picked in dialogs (content URIs), see native/dialog.rs.
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_fs::init());
    // The APIs of phones the application asks for, and only them: the
    // templates of `nativella android` and `nativella ios` are built with them
    // all.
    #[cfg(all(mobile, feature = "barcode-scanner"))]
    let builder = if manifest
        .permissions
        .contains(&project::Permission::BarcodeScanner)
    {
        builder.plugin(native::barcode_scanner::plugin())
    } else {
        builder
    };
    #[cfg(all(mobile, feature = "biometrics"))]
    let builder = if manifest
        .permissions
        .contains(&project::Permission::Biometrics)
    {
        builder.plugin(tauri_plugin_biometric::init())
    } else {
        builder
    };
    #[cfg(all(mobile, feature = "geolocation"))]
    let builder = if manifest
        .permissions
        .contains(&project::Permission::Geolocation)
    {
        builder.plugin(tauri_plugin_geolocation::init())
    } else {
        builder
    };
    #[cfg(all(mobile, feature = "haptics"))]
    let builder = if manifest.permissions.contains(&project::Permission::Haptics) {
        builder.plugin(tauri_plugin_haptics::init())
    } else {
        builder
    };

    #[cfg(target_os = "android")]
    let builder = builder.manage(request_bodies::Bodies::default());
    // What the scripts Nativella injects in the pages call.
    #[cfg(any(target_os = "android", all(mobile, feature = "barcode-scanner")))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        #[cfg(all(mobile, feature = "barcode-scanner"))]
        native::barcode_scanner::nativella_cancel_scan,
        #[cfg(target_os = "android")]
        request_bodies::nativella_store_body,
    ]);

    let builder = builder
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .register_asynchronous_uri_scheme_protocol("php", |context, request, responder| {
            #[cfg(target_os = "android")]
            #[cfg(target_os = "android")]
            let (request, page) = {
                let mut request = request;
                as_page_origin(&mut request);
                // Before restore(), which takes away what tells them apart.
                let page = deadline::is_page(&request);
                request_bodies::restore(context.app_handle(), &mut request);
                (request, page)
            };
            let origin = RequestOrigin::Webview {
                window: context.webview_label().to_owned(),
            };
            #[cfg(target_os = "android")]
            if request.uri().path() == deadline::IDLE_PATH {
                let idle = context.app_handle().state::<PhpRuntime>().is_idle();
                responder.respond(deadline::idle(idle));
                return;
            }
            let respond = move |response| responder.respond(response);
            #[cfg(target_os = "android")]
            let respond = deadline::guard(&request, page, respond);
            context
                .app_handle()
                .state::<PhpRuntime>()
                .dispatch(request, origin, respond);
        })
        .setup(move |app| {
            let extracted = match &source {
                Source::Project(_) => embedded::is_standalone(),
                #[cfg(mobile)]
                Source::Packaged(_) => true,
            };
            let project = source.into_project(app)?;
            // Not before: a second instance has exited by now, so the
            // previous build runs no more.
            if extracted {
                embedded::remove_other_builds(&project.dir);
            }
            let php_app = PhpApp::from_project_dir(&project.dir).map_err(|error| {
                format!(
                    "Cannot serve the PHP app in {}: {error}",
                    project.dir.display()
                )
            })?;

            // Absolute: PHP changes its working directory to public/.
            let data_dir = match env::var_os("NATIVELLA_DATA_DIR") {
                Some(data_dir) => std::path::absolute(PathBuf::from(data_dir))?,
                None => app.path().app_data_dir()?,
            };
            fs::create_dir_all(&data_dir)?;
            eprintln!("[nativella] Data directory: {}", data_dir.display());

            // PHP's defaults point to system directories: an application
            // has its own, and phones have no /tmp. Opcache locks its shared
            // memory with a file there, without thread safety.
            let cache_dir = app.path().app_cache_dir()?;
            let session_dir = cache_dir.join("sessions");
            fs::create_dir_all(&session_dir)?;
            let mut php_app = php_app.with_ini("session.save_path", session_dir.to_string_lossy());
            if cfg!(mobile) {
                php_app = php_app
                    .with_ini("sys_temp_dir", cache_dir.to_string_lossy())
                    .with_ini("upload_tmp_dir", cache_dir.to_string_lossy())
                    .with_ini("opcache.lockfile_path", cache_dir.to_string_lossy());
            }
            // `nativella run`: a reloaded page runs the code that changed, as
            // opcache checks the files on every request.
            if !extracted {
                php_app = php_app.with_ini("opcache.revalidate_freq", "0");
            }
            // Dates in the user's time zone, not UTC.
            match iana_time_zone::get_timezone() {
                Ok(time_zone) => php_app = php_app.with_ini("date.timezone", time_zone),
                Err(error) => eprintln!("[nativella] Unknown time zone, PHP uses UTC: {error}."),
            }

            let php_app = php_app
                .with_cookie_file(data_dir.join(".nativella-cookies"))
                .with_server_variable("NATIVELLA_DATA_DIR", data_dir.to_string_lossy())
                .with_server_variable("NATIVELLA_SECRET", secret::load(&data_dir)?)
                .with_server_variable("NATIVELLA_PLATFORM", env::consts::OS)
                .with_server_variable("NATIVELLA_VERSION", env!("CARGO_PKG_VERSION"));

            #[cfg(desktop)]
            if !extracted && reload::enabled() {
                reload::watch(app.handle(), &project.dir, &data_dir);
            }
            app.manage(State::new(
                project.dir.clone(),
                data_dir,
                project.manifest.window.clone(),
            ));
            // Before PHP: it keeps the handlers it finds and calls them, a
            // default action otherwise, which would kill the application.
            #[cfg(all(desktop, unix))]
            quit_on_signals(app.handle());

            let handle = app.handle().clone();
            let runtime =
                PhpRuntime::start(php_app, Box::new(move |call| native::call(&handle, call)));
            app.manage(runtime);

            events::boot(app.handle());
            #[cfg(desktop)]
            native::power::watch(app.handle());
            #[cfg(desktop)]
            if !manifest.deep_links.is_empty() {
                deep_links(app)?;
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label();
            match event {
                WindowEvent::Focused(true) => {
                    events::send(
                        window.app_handle(),
                        "window.focused",
                        json!({ "window": label }),
                    );
                }
                WindowEvent::Focused(false) => {
                    events::send(
                        window.app_handle(),
                        "window.blurred",
                        json!({ "window": label }),
                    );
                }
                // Not before its page lets it go.
                #[cfg(desktop)]
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    if let Some(window) = window.get_webview_window(label) {
                        before_unload::close(window);
                    }
                }
                WindowEvent::Destroyed => {
                    events::send(
                        window.app_handle(),
                        "window.closed",
                        json!({ "window": label }),
                    );
                }
                WindowEvent::ThemeChanged(theme) => {
                    events::send(
                        window.app_handle(),
                        "window.themeChanged",
                        json!({ "window": label, "theme": native::system::theme_name(*theme) }),
                    );
                }
                WindowEvent::DragDrop(tauri::DragDropEvent::Enter { paths, .. }) => {
                    events::send(
                        window.app_handle(),
                        "window.dragEntered",
                        json!({ "window": label, "paths": utf8_paths(paths) }),
                    );
                }
                WindowEvent::DragDrop(tauri::DragDropEvent::Leave) => {
                    events::send(
                        window.app_handle(),
                        "window.dragLeft",
                        json!({ "window": label }),
                    );
                }
                WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                    events::send(
                        window.app_handle(),
                        "window.filesDropped",
                        json!({ "window": label, "paths": utf8_paths(paths) }),
                    );
                }
                _ => {}
            }
        });

    let app = customize(builder)
        .build(context)
        .map_err(|error| format!("The application could not start: {error}"))?;

    app.run(|app, event| match event {
        // Closing the last window quits the application, unless it lives in
        // the system tray; once PHP has handled the events queued until then
        // (`window.closed` of that window, for one).
        RunEvent::ExitRequested {
            code: None, api, ..
        } => {
            api.prevent_exit();
            if !app.state::<State>().has_tray() {
                let handle = app.clone();
                app.state::<PhpRuntime>()
                    .after_queued(move || handle.exit(0));
            }
        }
        // The processes PHP started do not outlive the application.
        #[cfg(desktop)]
        RunEvent::Exit => app.state::<State>().child_processes.stop_all(),
        _ => {}
    });

    Ok(())
}

/// Tauri hands Android requests as `php://localhost/…` while the page lives
/// at `http://php.localhost/…`: PHP must see the page's origin, to build its
/// URLs and to check the `Origin` of forms (CSRF protections).
#[cfg(target_os = "android")]
fn as_page_origin(request: &mut tauri::http::Request<Vec<u8>>) {
    let path = request
        .uri()
        .path_and_query()
        .map_or("/", |path| path.as_str());

    match format!("{}{}", APP_URL.trim_end_matches('/'), path).parse() {
        Ok(uri) => *request.uri_mut() = uri,
        Err(error) => eprintln!("[android] Cannot rewrite {}: {error}.", request.uri()),
    }
}

/// Sends the URLs of the deep link schemes of the manifest to PHP, as
/// `app.urlsOpened`: the one that launched the application, then the next
/// ones (a second launch forwards its URL to the running instance).
#[cfg(desktop)]
fn deep_links(app: &App) -> tauri::Result<()> {
    use tauri_plugin_deep_link::DeepLinkExt;

    // Linux and Windows learn the schemes at runtime. Only for an installed
    // application: `nativella run` is not what the system should launch.
    #[cfg(any(target_os = "linux", windows))]
    if embedded::is_standalone()
        && let Err(error) = app.deep_link().register_all()
    {
        eprintln!("[nativella] Could not register the deep link schemes: {error}.");
    }

    let send = |app: &tauri::AppHandle, urls: Vec<tauri::Url>| {
        let urls: Vec<String> = urls.iter().map(ToString::to_string).collect();
        events::send(app, "app.urlsOpened", json!({ "urls": urls }));
    };

    if let Some(urls) = app
        .deep_link()
        .get_current()
        .map_err(|error| tauri::Error::Anyhow(error.into()))?
    {
        send(app.handle(), urls);
    }
    let handle = app.handle().clone();
    app.deep_link()
        .on_open_url(move |event| send(&handle, event.urls()));

    Ok(())
}

/// The dropped paths PHP can open: the others, not UTF-8, are logged.
fn utf8_paths(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|path| match native::dialog::utf8(path.clone()) {
            Ok(path) => Some(path),
            Err(error) => {
                eprintln!("[nativella] Dropped file skipped: {error}");
                None
            }
        })
        .collect()
}

/// Ctrl+C, a closed terminal or `kill` quit the application as its menu
/// would: the processes it started end with it. A second signal does not
/// wait.
///
/// Registered here, not in the thread: PHP must find the handlers when it
/// starts.
#[cfg(all(desktop, unix))]
fn quit_on_signals(app: &tauri::AppHandle) {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};

    let mut signals = match signal_hook::iterator::Signals::new([SIGHUP, SIGINT, SIGTERM]) {
        Ok(signals) => signals,
        Err(error) => {
            eprintln!("[nativella] Signals keep their default action: {error}.");
            return;
        }
    };
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("signals".to_owned())
        .spawn(move || {
            let mut quitting = false;
            for signal in signals.forever() {
                if quitting {
                    std::process::exit(128 + signal);
                }
                eprintln!("[nativella] Signal {signal}: quitting.");
                quitting = true;
                app.exit(0);
            }
        });
    if let Err(error) = spawned {
        // The handlers stay: these signals now do nothing.
        eprintln!("[nativella] Signals cannot quit the application: {error}.");
    }
}

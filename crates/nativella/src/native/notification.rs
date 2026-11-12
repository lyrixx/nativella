//! `notification.*`: desktop notifications.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri_plugin_notification::NotificationExt;

use super::{Call, NativeError, failed};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Send {
    title: String,
    body: Option<String>,
    /// Given back by `notification.clicked`, when the user clicks it.
    #[cfg_attr(
        not(any(target_os = "linux", target_os = "macos")),
        allow(dead_code, reason = "Only Linux and macOS report clicks.")
    )]
    reference: Option<String>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, NativeError>> {
    Some(match method {
        "send" => call.payload::<Send>().and_then(|payload| {
            #[cfg(target_os = "macos")]
            macos::name_the_sender(call.app);
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            if let Some(reference) = payload.reference {
                #[cfg(target_os = "linux")]
                clickable::show(call.app, &payload.title, payload.body.as_deref(), reference)?;
                #[cfg(target_os = "macos")]
                macos::show(call.app, &payload.title, payload.body.as_deref(), reference)?;

                return Ok(json!({}));
            }

            let mut notification = call.app.notification().builder().title(payload.title);
            if let Some(body) = payload.body {
                notification = notification.body(body);
            }
            notification.show().map_err(failed)?;

            Ok(json!({}))
        }),
        _ => return None,
    })
}

/// The plugin cannot tell a click. notify-rust shows the notification with
/// the "default" action of the XDG specification, which notification servers
/// invoke when the user clicks the notification itself; one listener on the
/// session bus, for the whole application, hears it. macOS reports clicks its
/// own way (see below); Windows, not there yet.
#[cfg(target_os = "linux")]
mod clickable {
    use std::collections::VecDeque;
    use std::sync::{Mutex, OnceLock};
    use std::thread;

    use serde_json::json;
    use tauri::AppHandle;
    use zbus::MatchRule;
    use zbus::blocking::{Connection, MessageIterator};
    use zbus::message::Type;

    use super::{NativeError, failed};
    use crate::events;
    use crate::native::lock;

    /// Notification id → reference, until the user clicks or dismisses it.
    /// GNOME keeps the notifications nobody clicked without saying when it
    /// drops them: the oldest go past that many.
    const KEPT: usize = 256;

    static PENDING: Mutex<VecDeque<(u32, String)>> = Mutex::new(VecDeque::new());
    static LISTENER: OnceLock<()> = OnceLock::new();

    pub(super) fn show(
        app: &AppHandle,
        title: &str,
        body: Option<&str>,
        reference: String,
    ) -> Result<(), NativeError> {
        LISTENER.get_or_init(|| {
            let app = app.clone();
            let spawned = thread::Builder::new()
                .name("notification clicks".to_owned())
                .spawn(move || {
                    if let Err(error) = listen(&app) {
                        eprintln!("[notification] Clicks are not reported: {error}.");
                    }
                });
            if let Err(error) = spawned {
                eprintln!("[notification] Clicks are not reported: {error}.");
            }
        });

        let mut notification = notify_rust::Notification::new();
        notification
            .summary(title)
            .auto_icon()
            // Servers show no button for this one.
            .action("default", "Open");
        if let Some(body) = body {
            notification.body(body);
        }
        let id = notification.show().map_err(failed)?.id();

        let mut pending = lock(&PENDING);
        if pending.len() == KEPT {
            pending.pop_front();
        }
        pending.push_back((id, reference));

        Ok(())
    }

    fn listen(app: &AppHandle) -> zbus::Result<()> {
        let connection = Connection::session()?;
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.Notifications")?
            .build();

        for message in MessageIterator::for_match_rule(rule, &connection, None)? {
            let message = message?;
            let header = message.header();
            let clicked = match header.member().map(|member| member.as_str()) {
                Some("ActionInvoked") => match message.body().deserialize::<(u32, String)>() {
                    Ok((id, action)) => Some((id, action == "default")),
                    Err(_) => None,
                },
                Some("NotificationClosed") => message
                    .body()
                    .deserialize::<(u32, u32)>()
                    .ok()
                    .map(|(id, _reason)| (id, false)),
                _ => None,
            };
            let Some((id, clicked)) = clicked else {
                continue;
            };

            // Other applications get their signals too: only ours are known.
            let reference = {
                let mut pending = lock(&PENDING);
                pending
                    .iter()
                    .position(|(pending, _)| *pending == id)
                    .and_then(|index| pending.remove(index))
                    .map(|(_, reference)| reference)
            };
            if let Some(reference) = reference
                && clicked
            {
                events::send(
                    app,
                    "notification.clicked",
                    json!({ "reference": reference }),
                );
            }
        }

        Ok(())
    }
}

/// On macOS, the plugin sends through mac-notification-sys, as the Terminal
/// while Tauri builds without its `custom-protocol` feature, which Nativella
/// does (`tauri::is_dev()`): its notifications would be the Terminal's.
/// mac-notification-sys keeps the first application it is given, so an app
/// gives its own first. And a notification with a reference waits for its
/// click, in a thread of its own, as mac-notification-sys waits: until the
/// user clicks it, closes it, or clears it from Notification Center.
#[cfg(target_os = "macos")]
mod macos {
    use std::sync::Once;
    use std::{env, thread};

    use mac_notification_sys::{Notification, NotificationResponse};
    use serde_json::json;
    use tauri::AppHandle;

    use super::{NativeError, failed};
    use crate::embedded;
    use crate::events;

    /// Sends the notifications of an app as its own. `nativella run` runs no
    /// app, which macOS knows of: they stay the Terminal's.
    pub(super) fn name_the_sender(app: &AppHandle) {
        static NAMED: Once = Once::new();
        NAMED.call_once(|| {
            let in_an_app = env::current_exe()
                .is_ok_and(|executable| embedded::app_bundle_of(&executable).is_some());
            if in_an_app
                && let Err(error) = mac_notification_sys::set_application(&app.config().identifier)
            {
                eprintln!("[notification] The notifications are not the application's: {error}.");
            }
        });
    }

    pub(super) fn show(
        app: &AppHandle,
        title: &str,
        body: Option<&str>,
        reference: String,
    ) -> Result<(), NativeError> {
        let app = app.clone();
        let title = title.to_owned();
        let body = body.map(str::to_owned);
        thread::Builder::new()
            .name("notification".to_owned())
            .spawn(move || {
                let mut notification = Notification::new();
                notification.title(&title).wait_for_click(true);
                if let Some(body) = &body {
                    notification.message(body);
                }
                match notification.send() {
                    Ok(NotificationResponse::Click) => events::send(
                        &app,
                        "notification.clicked",
                        json!({ "reference": reference }),
                    ),
                    Ok(_) => {}
                    Err(error) => eprintln!("[notification] \"{title}\" is not shown: {error}."),
                }
            })
            .map_err(failed)?;

        Ok(())
    }
}

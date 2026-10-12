//! `notification.*`: desktop notifications.

use serde::Deserialize;
use serde_json::{Value, json};
use tauri_plugin_notification::NotificationExt;

use super::{Call, failed};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Send {
    title: String,
    body: Option<String>,
    /// Given back by `notification.clicked`, when the user clicks it.
    #[cfg_attr(
        not(target_os = "linux"),
        allow(dead_code, reason = "Only Linux reports clicks.")
    )]
    reference: Option<String>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "send" => call.payload::<Send>().and_then(|payload| {
            #[cfg(target_os = "linux")]
            if let Some(reference) = payload.reference {
                clickable::show(call.app, &payload.title, payload.body.as_deref(), reference)?;

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
/// session bus, for the whole application, hears it. macOS and Windows report
/// clicks their own way: not there yet.
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

    use super::failed;
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
    ) -> Result<(), String> {
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

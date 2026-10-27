//! Closing a window asks its page first, as a browser closing a tab does: a
//! `beforeunload` listener that cancels the event keeps the window open,
//! unless the user confirms. WebKit runs `beforeunload` for a navigation,
//! not for a window that closes.
//!
//! On Linux, where Nativella makes the Quit item of the menu, quitting asks
//! every window the same way.

use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError, mpsc};
use std::thread;
use std::time::Duration;

use tauri::WebviewWindow;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

/// Evaluated in the page: true when it asks to stay.
const SCRIPT: &str = include_str!("before_unload.js");

/// How long a page has to answer: one that hangs, or has no script, does not
/// keep its window open.
const TIMEOUT: Duration = Duration::from_secs(1);

/// The windows whose page is being asked: a second click on the close
/// button does not ask twice.
static CLOSING: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Closes `window`, once its page lets it go or the user confirms. For a
/// window whose close request was prevented: `close()` would ask again.
pub(crate) fn close(window: WebviewWindow) {
    let label = window.label().to_owned();
    if !closing().insert(label.clone()) {
        return;
    }
    let answer = ask(&window);

    spawn(move || {
        let stays = answer
            .recv_timeout(TIMEOUT)
            .unwrap_or_else(|error| no_answer(&window, error));
        let leaves = !stays || confirm(&window, "Close this window?", "Close");
        closing().remove(&label);
        if !leaves {
            return;
        }
        if let Err(error) = window.destroy() {
            eprintln!(
                "[window] The \"{}\" window could not close: {error}.",
                window.label()
            );
        }
    });
}

/// Quits, once the pages of every window let them go or the user confirms.
#[cfg(target_os = "linux")]
pub(crate) fn quit(app: &tauri::AppHandle) {
    use std::time::Instant;
    use tauri::Manager;

    let windows: Vec<WebviewWindow> = app.webview_windows().into_values().collect();
    let answers: Vec<_> = windows.iter().map(ask).collect();
    let app = app.clone();

    spawn(move || {
        let deadline = Instant::now() + TIMEOUT;
        let staying = windows.iter().zip(answers).find(|(window, answer)| {
            answer
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap_or_else(|error| no_answer(window, error))
        });
        if let Some((window, _)) = staying
            && !confirm(window, "Quit?", "Quit")
        {
            return;
        }
        app.exit(0);
    });
}

/// Evaluates SCRIPT in `window`: its answer comes on the receiver, or never.
fn ask(window: &WebviewWindow) -> mpsc::Receiver<bool> {
    let (sender, answer) = mpsc::channel();
    // The result of the script, as JSON.
    let evaluated = window.eval_with_callback(SCRIPT, move |result| {
        let _ = sender.send(result == "true");
    });
    if let Err(error) = evaluated {
        eprintln!(
            "[window] The page of \"{}\" was not asked before closing: {error}.",
            window.label()
        );
    }

    answer
}

/// A page that does not answer lets its window go.
fn no_answer(window: &WebviewWindow, error: mpsc::RecvTimeoutError) -> bool {
    // Disconnected: the script was not evaluated, which ask() logged.
    if error == mpsc::RecvTimeoutError::Timeout {
        eprintln!(
            "[window] The page of \"{}\" did not answer before closing.",
            window.label()
        );
    }

    false
}

/// Asks the user, as a browser does, over `window`. Blocks: not on the main
/// thread.
fn confirm(window: &WebviewWindow, question: &str, action: &str) -> bool {
    window
        .dialog()
        .message(format!("{question} Changes you made may not be saved."))
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            action.to_owned(),
            "Cancel".to_owned(),
        ))
        .parent(window)
        .blocking_show()
}

fn closing() -> std::sync::MutexGuard<'static, BTreeSet<String>> {
    CLOSING.lock().unwrap_or_else(PoisonError::into_inner)
}

fn spawn(work: impl FnOnce() + Send + 'static) {
    if let Err(error) = thread::Builder::new()
        .name("before-unload".to_owned())
        .spawn(work)
    {
        eprintln!("[window] Could not ask the page before closing: {error}.");
    }
}

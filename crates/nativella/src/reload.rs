//! `nativella run` reloads the windows when a file of the project changes,
//! as the development servers of the front end do. Not standalone
//! executables, which run a copy of the project.
//!
//! The directories the application and its tools write to are left out:
//! a page that writes there would reload itself forever. `NATIVELLA_RELOAD=0`
//! turns reloading off.

use std::env;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify::{Event, EventKind, RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

/// What the frameworks, Composer, npm and git write to, relative to the
/// project: Symfony's cache and logs, Laravel's storage and cache.
const LEFT_OUT: &[&str] = &[
    ".git",
    "bootstrap/cache",
    "node_modules",
    "storage",
    "var",
    "vendor",
];

/// What the application writes as it runs, wherever it puts them: its logs,
/// and the SQLite databases with their journals. A page would reload itself
/// forever.
const WRITTEN: &[&str] = &[
    ".db", ".log", ".sqlite", ".sqlite3", "-journal", "-shm", "-wal",
];

/// An editor saves a file in several steps, a build tool writes several
/// files: the windows reload once they are done.
const QUIET: Duration = Duration::from_millis(200);

/// Whether `nativella run` reloads: unless `NATIVELLA_RELOAD` is `0`.
pub(crate) fn enabled() -> bool {
    env::var_os("NATIVELLA_RELOAD").is_none_or(|value| value != "0")
}

/// Watches `dir`, the project, and reloads every window when a file changes.
/// `data_dir` is left out too, when it lies in the project.
pub(crate) fn watch(app: &AppHandle, dir: &Path, data_dir: &Path) {
    // Events carry the paths as watched: absolute, as the data directory.
    let dir = match std::path::absolute(dir) {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("[reload] Cannot watch {}: {error}.", dir.display());
            return;
        }
    };
    let (sender, events) = mpsc::channel();
    let mut watcher = match notify::recommended_watcher(sender) {
        Ok(watcher) => watcher,
        Err(error) => {
            eprintln!("[reload] Cannot watch the project: {error}.");
            return;
        }
    };

    let left_out: Vec<PathBuf> = LEFT_OUT
        .iter()
        .map(|path| dir.join(path))
        .chain([data_dir.to_owned()])
        .collect();
    // Each entry on its own: a recursive watch of the project would watch
    // the thousands of directories of vendor/ too.
    let entries = match dir.read_dir() {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("[reload] Cannot watch {}: {error}.", dir.display());
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if left_out.contains(&path) {
            continue;
        }
        let mode = if path.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        if let Err(error) = watcher.watch(&path, mode) {
            eprintln!("[reload] Cannot watch {}: {error}.", path.display());
        }
    }

    let app = app.clone();
    let spawned = thread::Builder::new()
        .name("reload".to_owned())
        .spawn(move || {
            // Lives as long as the thread.
            let _watcher = watcher;
            while let Ok(event) = events.recv() {
                let Some(changed) = changed(event, &left_out) else {
                    continue;
                };
                // Until the files stop changing.
                while events.recv_timeout(QUIET).is_ok() {}

                eprintln!(
                    "[reload] {} changed: reloading the windows.",
                    changed.strip_prefix(&dir).unwrap_or(&changed).display()
                );
                for window in app.webview_windows().values() {
                    if let Err(error) = window.reload() {
                        eprintln!("[reload] Could not reload \"{}\": {error}.", window.label());
                    }
                }
            }
        });
    if let Err(error) = spawned {
        eprintln!("[reload] Cannot watch the project: {error}.");
    }
}

/// The file `event` changed, when it counts: written, created, removed or
/// renamed, out of the directories left out, not a file an editor keeps
/// aside (`.notes.php.swp`, `notes.php~`), nor one the application writes.
fn changed(event: notify::Result<Event>, left_out: &[PathBuf]) -> Option<PathBuf> {
    let event = event.ok()?;
    if !matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) {
        return None;
    }

    event.paths.into_iter().find(|path| {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default();

        !name.starts_with('.')
            && !name.ends_with('~')
            && !WRITTEN.iter().any(|suffix| name.ends_with(suffix))
            && !left_out.iter().any(|left_out| path.starts_with(left_out))
    })
}

#[cfg(test)]
mod tests {
    use notify::event::{AccessKind, CreateKind, ModifyKind};

    use super::*;

    fn event(kind: EventKind, path: &str) -> notify::Result<Event> {
        Ok(Event::new(kind).add_path(PathBuf::from(path)))
    }

    #[test]
    fn a_source_file_counts() {
        let left_out = [PathBuf::from("/app/var")];

        assert_eq!(
            changed(
                event(EventKind::Modify(ModifyKind::Any), "/app/src/Note.php"),
                &left_out
            ),
            Some(PathBuf::from("/app/src/Note.php"))
        );
        assert_eq!(
            changed(
                event(EventKind::Create(CreateKind::File), "/app/templates/a.twig"),
                &left_out
            ),
            Some(PathBuf::from("/app/templates/a.twig"))
        );
    }

    #[test]
    fn what_the_application_writes_does_not() {
        let left_out = [PathBuf::from("/app/var"), PathBuf::from("/app/storage")];
        let modified = |path| changed(event(EventKind::Modify(ModifyKind::Any), path), &left_out);

        assert_eq!(modified("/app/var/cache/dev/Container.php"), None);
        assert_eq!(modified("/app/storage/framework/views/a.php"), None);
        assert_eq!(modified("/app/src/.Note.php.swp"), None);
        assert_eq!(modified("/app/src/Note.php~"), None);
        assert_eq!(modified("/app/database/database.sqlite-journal"), None);
        assert_eq!(modified("/app/app.log"), None);
        assert_eq!(
            changed(
                event(EventKind::Access(AccessKind::Any), "/app/src/Note.php"),
                &left_out
            ),
            None
        );
    }
}

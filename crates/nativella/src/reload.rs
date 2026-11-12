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

use notify::event::ModifyKind;
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
    // The project itself, for its files and the directories it gets: a
    // watch on a file is lost when an editor saves it by replacing it. Then
    // each directory on its own: a recursive watch of the project would
    // watch the thousands of directories of vendor/ too.
    if let Err(error) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
        eprintln!("[reload] Cannot watch {}: {error}.", dir.display());
        return;
    }
    let entries = match dir.read_dir() {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("[reload] Cannot watch {}: {error}.", dir.display());
            return;
        }
    };
    for entry in entries.flatten() {
        watch_directory(&mut watcher, &entry.path(), &left_out);
    }

    let app = app.clone();
    let spawned = thread::Builder::new()
        .name("reload".to_owned())
        .spawn(move || {
            while let Ok(event) = events.recv() {
                watch_created(&mut watcher, &event, &dir, &left_out);
                let Some(changed) = changed(event, &left_out) else {
                    continue;
                };
                // Until the files stop changing.
                while let Ok(event) = events.recv_timeout(QUIET) {
                    watch_created(&mut watcher, &event, &dir, &left_out);
                }

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

/// Watches `path` and what it holds, when it is a directory that is not
/// left out.
fn watch_directory(watcher: &mut impl Watcher, path: &Path, left_out: &[PathBuf]) {
    if !path.is_dir() || left_out.iter().any(|left_out| left_out == path) {
        return;
    }
    if let Err(error) = watcher.watch(path, RecursiveMode::Recursive) {
        eprintln!("[reload] Cannot watch {}: {error}.", path.display());
    }
}

/// Watches the directories `event` says the project `dir` got since it
/// started.
fn watch_created(
    watcher: &mut impl Watcher,
    event: &notify::Result<Event>,
    dir: &Path,
    left_out: &[PathBuf],
) {
    let Ok(event) = event else {
        return;
    };
    // Made, or moved there.
    if !matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(ModifyKind::Name(_))
    ) {
        return;
    }
    for path in &event.paths {
        if path.parent() == Some(dir) {
            watch_directory(watcher, path, left_out);
        }
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
    use notify::event::{AccessKind, CreateKind};

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

//! The updates of a macOS app (see macos.rs). Its release is the `.tar.gz`
//! of the new app, as Tauri's updater takes it, and as `tar` makes it:
//!
//! ```bash
//! COPYFILE_DISABLE=1 tar -czf notes.app.tar.gz -C dist Notes.app
//! ```
//!
//! The archive is extracted next to the running app, checked, then
//! exchanged with it in one step: there is no moment without an app. The
//! previous one stays next to the new one, as `<name>.app.old`, which Finder
//! does not take for an app. The running application keeps the files it
//! opened: its executable, and the project it extracted.

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use plist::{Dictionary, Value};
use tar::EntryType;

#[cfg(target_os = "macos")]
use super::{
    DOWNLOAD_LIMIT, Expected, check_manifest, for_this_machine, previous, remove_previous,
};
#[cfg(target_os = "macos")]
use crate::apple::Staging;
#[cfg(target_os = "macos")]
use crate::embedded::{Bundle, PACKAGED_BUNDLE};

/// Extracts the app of `archive`, the release downloaded to `download`,
/// checks it, and puts it in the place of `app`.
#[cfg(target_os = "macos")]
pub(super) fn put_in_place(
    archive: &[u8],
    download: &Path,
    app: &Path,
    expected: &Expected,
) -> Result<(), String> {
    // Named after the download: an application that quits before the end
    // removes it at its next start, as the download.
    let mut staging = download.as_os_str().to_owned();
    staging.push("-app");
    let staging = Staging::create(PathBuf::from(staging)).map_err(|error| error.to_string())?;

    let new_app = extract(archive, &staging.0, DOWNLOAD_LIMIT)?;
    check(&new_app, expected)?;
    let previous = previous(app);
    remove_previous(&previous).map_err(|error| error.to_string())?;
    exchange(&new_app, app).map_err(|error| {
        format!(
            "{} cannot take the place of {}: {error}.",
            new_app.display(),
            app.display()
        )
    })?;
    // The previous app, where the new one was.
    if let Err(error) = fs::rename(&new_app, &previous) {
        eprintln!(
            "[updater] The previous version is not kept as {}: {error}.",
            previous.display()
        );
    }
    if let Err(error) = fs::remove_file(download) {
        eprintln!("[updater] {} is left behind: {error}.", download.display());
    }

    Ok(())
}

/// Extracts the app of the `.tar.gz` file `archive` into `dir`: one
/// `<name>.app` directory, which holds directories and files only, `limit`
/// bytes at most. Returns its path.
pub(super) fn extract(archive: &[u8], dir: &Path, limit: u64) -> Result<PathBuf, String> {
    let unreadable =
        |error: io::Error| format!("The update is not the .tar.gz of an app: {error}.");
    let mut archive = tar::Archive::new(GzDecoder::new(archive));
    let mut app: Option<OsString> = None;
    let mut extracted: u64 = 0;

    for entry in archive.entries().map_err(unreadable)? {
        let mut entry = entry.map_err(unreadable)?;
        let path = entry.path().map_err(unreadable)?.into_owned();
        // The extended attributes macOS's tar keeps, as AppleDouble files,
        // without COPYFILE_DISABLE.
        if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("._"))
        {
            continue;
        }
        let mut parts = Vec::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(part) => parts.push(part.to_owned()),
                _ => {
                    return Err(format!(
                        "The update holds {}, outside of its app.",
                        path.display()
                    ));
                }
            }
        }
        // `./`
        let Some(first) = parts.first() else {
            continue;
        };
        if Path::new(first)
            .extension()
            .is_none_or(|extension| extension != "app")
        {
            return Err(format!(
                "The update holds {}, outside of an app: publish the .tar.gz of the app of nativella build.",
                path.display()
            ));
        }
        match &app {
            None => app = Some(first.clone()),
            Some(app) if app != first => {
                return Err("The update holds several apps.".to_owned());
            }
            Some(_) => {}
        }

        let target = dir.join(parts.iter().collect::<PathBuf>());
        let written = match entry.header().entry_type() {
            EntryType::Directory => fs::create_dir_all(&target),
            EntryType::Regular => {
                extracted = extracted.saturating_add(entry.size());
                if extracted > limit {
                    return Err(format!(
                        "The app of the update is over {} MiB.",
                        limit / 1024 / 1024
                    ));
                }
                let executable = entry.header().mode().unwrap_or(0) & 0o111 != 0;
                write_file(&target, &mut entry, executable)
            }
            _ => {
                return Err(format!(
                    "The update holds {}, a link or a special file, which the apps of nativella build do not hold.",
                    path.display()
                ));
            }
        };
        written.map_err(|error| format!("Cannot extract {}: {error}.", path.display()))?;
    }

    let app = app.ok_or_else(|| {
        "The update holds no app: publish the .tar.gz of the app of nativella build.".to_owned()
    })?;

    Ok(dir.join(app))
}

/// Writes what `entry` holds to the new file `path`.
fn write_file(path: &Path, entry: &mut impl Read, executable: bool) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(if executable { 0o755 } else { 0o644 });
    }
    #[cfg(not(unix))]
    let _ = executable;
    let mut file = options.open(path)?;
    io::copy(entry, &mut file)?;

    file.sync_all()
}

/// Checks that `app` runs on this machine and is the version `expected` of
/// this application.
#[cfg(target_os = "macos")]
fn check(app: &Path, expected: &Expected) -> Result<(), String> {
    let contents = app.join("Contents");
    let executable = executable(&contents)?;
    let mut head = Vec::new();
    fs::File::open(contents.join("MacOS").join(&executable))
        .and_then(|file| file.take(64).read_to_end(&mut head))
        .map_err(|error| format!("The executable of the app is unreadable: {error}."))?;
    if !for_this_machine(&head) {
        return Err(format!(
            "The app is not one for {}: check the platforms of the endpoint.",
            super::platform()
        ));
    }

    let bundle = fs::File::open(contents.join("Resources").join(PACKAGED_BUNDLE))
        .and_then(Bundle::read)
        .map_err(|error| format!("The app carries no readable application: {error}."))?
        .ok_or_else(|| {
            "The app carries no application: publish the one of nativella build.".to_owned()
        })?;

    check_manifest(bundle.manifest(), expected)
}

/// The executable of the app whose `Contents/` is `contents`, as its
/// Info.plist names it: a file of `MacOS/`.
fn executable(contents: &Path) -> Result<String, String> {
    let info: Dictionary = plist::from_file(contents.join("Info.plist"))
        .map_err(|error| format!("The Info.plist of the app is unreadable: {error}."))?;
    let executable = info
        .get("CFBundleExecutable")
        .and_then(Value::as_string)
        .ok_or_else(|| "The Info.plist of the app has no CFBundleExecutable.".to_owned())?;
    let mut components = Path::new(executable).components();
    if !matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(_)), None)
    ) {
        return Err(format!(
            "The executable of the app, {executable}, is not a file of its MacOS directory."
        ));
    }

    Ok(executable.to_owned())
}

/// Exchanges `new_app` and `app`, in one step: `renamex_np()` with
/// `RENAME_SWAP`.
#[cfg(target_os = "macos")]
fn exchange(new_app: &Path, app: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let from = CString::new(new_app.as_os_str().as_bytes())?;
    let to = CString::new(app.as_os_str().as_bytes())?;
    // SAFETY: two NUL-terminated paths, which the call only reads.
    if unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_SWAP) } != 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use flate2::Compression;
    use flate2::write::GzEncoder;

    use super::*;

    /// A `.tar.gz` of `entries`: their path, their type, their mode and what
    /// they hold.
    fn archive(entries: &[(&str, EntryType, u32, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
        for &(path, kind, mode, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(kind);
            header.set_mode(mode);
            header.set_size(data.len() as u64);
            if kind == EntryType::Symlink {
                header.set_link_name("/etc/passwd").unwrap();
            }
            // Written as is, so that `..` and absolute paths reach the
            // archive.
            header.as_old_mut().name[..path.len()].copy_from_slice(path.as_bytes());
            header.set_cksum();
            builder.append(&header, data).unwrap();
        }

        builder.into_inner().unwrap().finish().unwrap()
    }

    fn directory(name: &str) -> PathBuf {
        let dir =
            Path::new(env!("OUT_DIR")).join(format!("updater-app-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        dir
    }

    fn app_entries() -> Vec<(&'static str, EntryType, u32, &'static [u8])> {
        vec![
            ("./", EntryType::Directory, 0o755, b""),
            ("Notes.app/", EntryType::Directory, 0o755, b""),
            ("Notes.app/Contents/", EntryType::Directory, 0o755, b""),
            (
                "Notes.app/Contents/Info.plist",
                EntryType::Regular,
                0o644,
                b"<plist><dict><key>CFBundleExecutable</key><string>Notes</string></dict></plist>",
            ),
            (
                "Notes.app/Contents/MacOS/Notes",
                EntryType::Regular,
                0o755,
                b"executable",
            ),
            ("._Notes.app", EntryType::Regular, 0o644, b"AppleDouble"),
        ]
    }

    #[test]
    fn the_app_of_the_archive_is_extracted() {
        let dir = directory("extract");

        let app = extract(&archive(&app_entries()), &dir, 1024).unwrap();

        assert_eq!(app, dir.join("Notes.app"));
        assert_eq!(
            fs::read(app.join("Contents/MacOS/Notes")).unwrap(),
            b"executable"
        );
        assert!(!dir.join("._Notes.app").exists());
        assert_eq!(executable(&app.join("Contents")).unwrap(), "Notes");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode =
                |path: &str| fs::metadata(app.join(path)).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode("Contents/MacOS/Notes"), 0o755);
            assert_eq!(mode("Contents/Info.plist"), 0o644);
        }

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn nothing_but_one_app_is_extracted() {
        let dir = directory("refused");
        for (name, entry, message) in [
            (
                "escape",
                (
                    "Notes.app/../../escaped",
                    EntryType::Regular,
                    0o644,
                    &b"x"[..],
                ),
                "outside of its app",
            ),
            (
                "absolute",
                ("/escaped", EntryType::Regular, 0o644, &b"x"[..]),
                "outside of its app",
            ),
            (
                "link",
                (
                    "Notes.app/Contents/passwd",
                    EntryType::Symlink,
                    0o777,
                    &b""[..],
                ),
                "a link",
            ),
            (
                "loose",
                ("notes.txt", EntryType::Regular, 0o644, &b"x"[..]),
                "outside of an app",
            ),
            (
                "other",
                (
                    "Tasks.app/Contents/Info.plist",
                    EntryType::Regular,
                    0o644,
                    &b"x"[..],
                ),
                "several apps",
            ),
        ] {
            let mut entries = app_entries();
            entries.push(entry);
            let target = dir.join(name);
            fs::create_dir(&target).unwrap();

            let error = extract(&archive(&entries), &target, 1024).unwrap_err();

            assert!(error.contains(message), "{name}: {error}");
        }
        assert!(!dir.join("escaped").exists());

        let error = extract(&archive(&app_entries()), &dir.join("limit"), 8).unwrap_err();
        assert!(error.contains("over"), "{error}");
        assert!(extract(b"not an archive", &dir, 1024).is_err());

        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_new_app_takes_the_place_of_the_running_one_in_one_step() {
        let dir = directory("exchange");
        for (app, version) in [("Notes.app", "1.2.0"), ("new/Notes.app", "1.3.0")] {
            fs::create_dir_all(dir.join(app)).unwrap();
            fs::write(dir.join(app).join("version"), version).unwrap();
        }

        exchange(&dir.join("new/Notes.app"), &dir.join("Notes.app")).unwrap();

        let version = |app: &str| fs::read_to_string(dir.join(app).join("version")).unwrap();
        assert_eq!(version("Notes.app"), "1.3.0");
        assert_eq!(version("new/Notes.app"), "1.2.0");
        assert!(exchange(&dir.join("missing.app"), &dir.join("Notes.app")).is_err());

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_executable_is_a_file_of_the_app() {
        let dir = directory("executable");
        for (name, executable_name) in [("outside", "../Notes"), ("absolute", "/bin/sh")] {
            let contents = dir.join(name);
            fs::create_dir_all(&contents).unwrap();
            fs::write(
                contents.join("Info.plist"),
                format!("<plist><dict><key>CFBundleExecutable</key><string>{executable_name}</string></dict></plist>"),
            )
            .unwrap();

            assert!(executable(&contents).is_err(), "{executable_name}");
        }

        fs::remove_dir_all(dir).unwrap();
    }
}

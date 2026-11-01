//! What the packaging writes: files named after the application, dated for
//! reproducible builds, each written next to its path, which it takes once
//! complete. A packaging that fails leaves no file cut short, and the
//! previous one in place.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
#[cfg(desktop)]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::project::Manifest;

/// The name of the files of an application, from its name: of the
/// executable, of the APK, of the Debian package, of its executable and of
/// its desktop entry. "Nativella Notes" makes `nativella-notes`: lower case
/// ASCII letters and digits, `+`, `.` and `-`, as Debian names packages, and
/// no path.
pub fn package_name(manifest: &Manifest) -> String {
    let name: String = manifest
        .name
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '+' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect();
    let name = name
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    match name.chars().next() {
        Some(first) if first.is_ascii_alphanumeric() && name.len() > 1 => name,
        _ => format!("app-{name}"),
    }
}

/// A file being written next to its path.
pub(crate) struct Output {
    file: File,
    staging: Staging,
    path: PathBuf,
}

/// Where an output is written, until it takes the place of its path: removed
/// unless it does.
struct Staging(PathBuf);

impl Output {
    /// Starts writing `path`, aside, in a new file of a name no one can
    /// guess: it could be a link to another file otherwise.
    pub fn create(path: &Path) -> io::Result<Self> {
        let name = path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("The output {} is not the path of a file.", path.display()),
            )
        })?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;

        let mut random = [0; 8];
        getrandom::fill(&mut random).map_err(io::Error::other)?;
        let random: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let staging = parent.join(format!(".{}.{random}.tmp", name.to_string_lossy()));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&staging)?;

        Ok(Self {
            file,
            staging: Staging(staging),
            path: path.to_owned(),
        })
    }

    pub fn file(&mut self) -> &mut File {
        &mut self.file
    }

    /// Where the file is written: inside the project, it must not ship.
    pub fn staging(&self) -> &Path {
        &self.staging.0
    }

    /// Closes the file, then puts it in place of its path.
    pub fn finish(self) -> io::Result<()> {
        let Self {
            file,
            mut staging,
            path,
        } = self;
        file.sync_all()?;
        drop(file);

        fs::rename(&staging.0, &path)?;
        staging.0 = PathBuf::new();

        Ok(())
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if self.0.as_os_str().is_empty() {
            return;
        }
        if let Err(error) = fs::remove_file(&self.0) {
            eprintln!("Warning: cannot remove {}: {error}.", self.0.display());
        }
    }
}

/// The date of what the packaging makes: `SOURCE_DATE_EPOCH`, in seconds,
/// for reproducible builds, as https://reproducible-builds.org/specs/source-date-epoch/
/// defines it, or now.
#[cfg(desktop)]
pub(crate) fn build_time() -> io::Result<SystemTime> {
    let Some(epoch) = std::env::var_os("SOURCE_DATE_EPOCH") else {
        return Ok(SystemTime::now());
    };

    epoch
        .to_str()
        .and_then(|epoch| epoch.parse().ok())
        .map(|seconds| UNIX_EPOCH + Duration::from_secs(seconds))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "SOURCE_DATE_EPOCH is {}, not a number of seconds since 1970.",
                    epoch.to_string_lossy()
                ),
            )
        })
}

/// Refuses an `output` that is the `input` of the packaging: writing it would
/// destroy it.
#[cfg(desktop)]
pub(crate) fn refuse_input(output: &Path, input: &Path) -> io::Result<()> {
    let same = output
        .canonicalize()
        .and_then(|output| Ok(output == input.canonicalize()?))
        .unwrap_or(false);
    if !same {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "The packaging would write over {}, which it reads: choose another output.",
            input.display()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn manifest(name: &str) -> Manifest {
        Manifest {
            name: name.to_owned(),
            identifier: "com.example.notes".to_owned(),
            version: "1.2.0".to_owned(),
            icon: None,
            window: serde_json::Map::new(),
            deep_links: Vec::new(),
            permissions: Vec::new(),
            updates: None,
        }
    }

    #[test]
    fn the_files_are_named_after_the_application() {
        assert_eq!(
            package_name(&manifest("Nativella Notes")),
            "nativella-notes"
        );
        assert_eq!(package_name(&manifest("Notes/Pro")), "notes-pro");
        assert_eq!(package_name(&manifest("..")), "app-..");
        assert_eq!(package_name(&manifest("Café & Co.")), "caf-co.");
        assert_eq!(package_name(&manifest("été")), "app-t");
    }

    #[test]
    fn the_output_takes_its_place_once_complete() {
        let dir = Path::new(env!("OUT_DIR")).join(format!("output-test-{}", std::process::id()));
        let path = dir.join("app");

        let mut output = Output::create(&path).unwrap();
        output.file().write_all(b"cut").unwrap();
        drop(output);
        assert!(!path.exists(), "A failure leaves nothing.");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);

        fs::write(&path, "previous").unwrap();
        let mut output = Output::create(&path).unwrap();
        output.file().write_all(b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"previous");
        output.finish().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);

        assert!(refuse_input(&path, &dir.join("./app")).is_err());
        assert!(refuse_input(&dir.join("other"), &path).is_ok());

        fs::remove_dir_all(dir).unwrap();
    }
}

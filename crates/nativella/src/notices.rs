//! The third-party notices: the copyright notices and the licenses of what
//! the build contains, which most of those licenses ask to ship with it.
//!
//! `castor nativella:build` gathers them (.castor/notices.php) and build.rs
//! compresses them into the executable, and so into every application
//! `nativella build` makes of it; `castor mobile:ios-template` into the
//! library of the template app, and so into every iOS app `nativella ios`
//! makes of it. An Android application carries them as an asset of its APK
//! instead, from the template.

#[cfg(desktop)]
use std::fs::File;
#[cfg(desktop)]
use std::io::Write;
use std::io::{self, Read};
#[cfg(desktop)]
use std::path::{Path, PathBuf};

#[cfg(desktop)]
use crate::output::Output;

/// The name of the notices: next to the executables `nativella build`
/// writes, and an asset of the APKs.
pub const FILE: &str = "THIRD_PARTY_NOTICES.txt";

/// How the notices start. Keep in sync with write() in .castor/notices.php.
#[cfg(desktop)]
const TITLE: &[u8] = b"Third-party notices of Nativella ";

/// Writes the notices `text` next to the standalone executable `output`, in
/// place of those of a previous build, and of no other file: a project may
/// have notices of its own under that name. Returns where.
#[cfg(desktop)]
pub fn write_next_to(output: &Path, text: &str) -> io::Result<PathBuf> {
    let path = output.with_file_name(FILE);
    match File::open(&path) {
        Ok(previous) => {
            let mut title = Vec::new();
            previous
                .take(TITLE.len() as u64)
                .read_to_end(&mut title)?;
            if title != TITLE {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!(
                        "{} exists and is not the notices of a previous build, which this one would replace: move it, or choose another output directory.",
                        path.display()
                    ),
                ));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    let mut target = Output::create(&path)?;
    target.file().write_all(text.as_bytes())?;
    target.finish()?;

    Ok(path)
}

/// The notices, gzipped, empty in a development build.
#[cfg(not(target_os = "android"))]
static COMPRESSED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/notices.txt.gz"));

/// The notices of the build, `None` for a development build, which has none.
#[cfg(not(target_os = "android"))]
pub fn text() -> io::Result<Option<String>> {
    if COMPRESSED.is_empty() {
        return Ok(None);
    }

    let mut text = String::new();
    flate2::read::GzDecoder::new(COMPRESSED).read_to_string(&mut text)?;

    Ok(Some(text))
}

/// The notices of the application: the asset [`FILE`] of its APK.
#[cfg(target_os = "android")]
pub fn text() -> io::Result<Option<String>> {
    let Some(mut asset) = crate::embedded::asset::open(FILE)? else {
        return Ok(None);
    };

    let mut text = String::new();
    asset.read_to_string(&mut text)?;

    Ok(Some(text))
}

#[cfg(all(test, desktop))]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn the_notices_replace_those_of_a_previous_build_only() {
        let dir = Path::new(env!("OUT_DIR")).join(format!("notices-test-{}", std::process::id()));
        let output = dir.join("notes");
        let first = "Third-party notices of Nativella 0.5.0, linux-x86_64\n";
        let second = "Third-party notices of Nativella 0.6.0, linux-x86_64\n";

        let path = write_next_to(&output, first).unwrap();
        assert_eq!(path, dir.join(FILE));
        write_next_to(&output, second).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), second);

        fs::write(&path, "The notices of the project").unwrap();
        let error = write_next_to(&output, second).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "The notices of the project"
        );

        fs::remove_dir_all(dir).unwrap();
    }
}

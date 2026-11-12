//! What the apps of iOS (`nativella ios`) and of macOS (`nativella build` on
//! a Mac) share: Apple's code signature, as codesign writes it (see
//! signature.rs), the version numbers of their Info.plist, and how they take
//! the place of the previous one.
//!
//! Both are a directory, `<name>.app`, laid out differently (see [`Layout`]).

pub(crate) mod macho;
pub(crate) mod resources;
pub(crate) mod signature;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use semver::Version;

/// Where an app keeps its files.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Layout {
    /// At its root, as iOS apps: the Info.plist, the executable, the
    /// resources and `_CodeSignature/`.
    Ios,
    /// In `Contents/`, as macOS apps: the Info.plist, the executable in
    /// `MacOS/`, the resources in `Resources/`, and `_CodeSignature/`.
    Macos,
}

impl Layout {
    /// The directory of the app that holds its Info.plist, its executable,
    /// its resources and its signature.
    pub(crate) fn contents(self, app: &Path) -> PathBuf {
        match self {
            Self::Ios => app.to_owned(),
            Self::Macos => app.join("Contents"),
        }
    }

    /// The path of the executable `name` in [`Layout::contents`], with `/`.
    pub(crate) fn executable(self, name: &str) -> String {
        match self {
            Self::Ios => name.to_owned(),
            Self::Macos => format!("MacOS/{name}"),
        }
    }
}

/// `CFBundleShortVersionString` and `CFBundleVersion`: the version of the
/// project as three integers, which Apple requires, without its pre-release
/// or build suffix.
pub(crate) fn bundle_version(version: &str) -> io::Result<String> {
    let Version {
        major,
        minor,
        patch,
        ..
    } = Version::parse(version).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "The version \"{version}\" cannot number an app of Apple's, which takes three integers: major.minor.patch."
            ),
        )
    })?;

    Ok(format!("{major}.{minor}.{patch}"))
}

/// A directory that the packaging removes when it ends, whether it failed
/// or not.
pub(crate) struct Staging(pub(crate) PathBuf);

impl Staging {
    pub(crate) fn create(path: PathBuf) -> io::Result<Self> {
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
        fs::create_dir(&path)?;

        Ok(Self(path))
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0)
            && error.kind() != io::ErrorKind::NotFound
        {
            eprintln!("Warning: cannot remove {}: {error}.", self.0.display());
        }
    }
}

/// Moves the app made in `app_dir` to `output`, in place of a previous one.
pub(crate) fn replace_app(app_dir: &Path, output: &Path, layout: Layout) -> io::Result<()> {
    if output.exists() {
        // Never a directory that is not an app: the output could be a typo.
        if !layout.contents(output).join("Info.plist").is_file() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "{} exists and is not an app: choose another output.",
                    output.display()
                ),
            ));
        }
        fs::remove_dir_all(output)?;
    }

    fs::rename(app_dir, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_versions() {
        assert_eq!(bundle_version("1.2.3").unwrap(), "1.2.3");
        assert_eq!(bundle_version("2.3.4-beta.1+abc").unwrap(), "2.3.4");

        for version in ["", "1.2", "1.2.3.4", "v1.0.0"] {
            assert!(bundle_version(version).is_err(), "{version}");
        }
    }
}

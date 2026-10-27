//! `nativella deb`: a Debian package of a project, for Debian, Ubuntu and
//! the distributions made of them. It holds the standalone executable of
//! `nativella build` in `/usr/bin`, a desktop entry and the icon, so that the
//! application shows among the others, and depends on the webview.
//!
//! A package is an `ar` archive of three members: `debian-binary`, the
//! version of the format, `control.tar.gz`, what dpkg reads, and
//! `data.tar.gz`, the files, from the root of the system.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::Value;

use crate::apk::icon::Icon;
use crate::project::{Manifest, Project};
use crate::{embedded, notices};

/// The sizes of the icon desktops pick from, in pixels.
const ICON_SIZES: &[u32] = &[32, 48, 64, 128, 256, 512];

/// What the executable needs on the system, as the releases of `nativella`
/// are built: Ubuntu 22.04 has these. Keep in sync with docs/installation.md.
const DEPENDS: &str = "libwebkit2gtk-4.1-0, libc6 (>= 2.35)";
/// Only the applications with a tray icon need it.
const RECOMMENDS: &str = "libayatana-appindicator3-1";

/// What describes the package, from composer.json.
#[derive(Debug, PartialEq)]
struct Control {
    package: String,
    version: String,
    architecture: &'static str,
    /// The first author of composer.json, or the application: dpkg warns
    /// about a package without one, each time it reads it.
    maintainer: String,
    homepage: Option<String>,
    description: String,
}

/// The name of the package, of its executable and of its desktop entry, from
/// the name of the application: "Nativella Notes" makes `nativella-notes`.
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

/// Debian's name of the architecture of this executable, which the package
/// carries a copy of.
pub fn architecture() -> io::Result<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" => Ok("amd64"),
        "aarch64" => Ok("arm64"),
        other => Err(io::Error::other(format!(
            "Debian packages of {other} are not supported."
        ))),
    }
}

/// Writes the package of `project` to `output`. Returns the local overrides
/// the executable left out.
pub fn build(project: &Project, output: &Path) -> io::Result<Vec<String>> {
    let composer: Value = serde_json::from_slice(&fs::read(project.dir.join("composer.json"))?)
        .map_err(|error| io::Error::other(format!("composer.json is invalid: {error}.")))?;
    let control = control(&project.manifest, &composer)?;
    let package = &control.package;

    // The executable, next to the package: `nativella build` leaves what it
    // writes out of the project.
    let executable = output.with_file_name(format!(".{package}.executable"));
    let built = embedded::build(project, &executable)
        .and_then(|local_overrides| Ok((local_overrides, fs::read(&executable)?)));
    let _ = fs::remove_file(&executable);
    let (local_overrides, executable) = built?;

    let icon = Icon::of(project)?;
    let mut files = vec![
        DataFile::new(format!("usr/bin/{package}"), executable, 0o755),
        DataFile::new(
            format!("usr/share/applications/{package}.desktop"),
            desktop_entry(&project.manifest, &control).into_bytes(),
            0o644,
        ),
    ];
    for &size in ICON_SIZES {
        files.push(DataFile::new(
            format!("usr/share/icons/hicolor/{size}x{size}/apps/{package}.png"),
            icon.png(size),
            0o644,
        ));
    }
    if let Some(text) = notices::text()? {
        files.push(DataFile::new(
            format!("usr/share/doc/{package}/{}", notices::FILE),
            text.into_bytes(),
            0o644,
        ));
    }

    let installed_kib = files
        .iter()
        .map(|file| file.contents.len() as u64)
        .sum::<u64>()
        .div_ceil(1024);
    let control_text = control.text(installed_kib);
    let data = tar_gz(&files)?;
    let control_archive = tar_gz(&[DataFile::new(
        "control".to_owned(),
        control_text.into_bytes(),
        0o644,
    )])?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut archive = File::create(output)?;
    archive.write_all(b"!<arch>\n")?;
    ar_member(&mut archive, "debian-binary", b"2.0\n")?;
    ar_member(&mut archive, "control.tar.gz", &control_archive)?;
    ar_member(&mut archive, "data.tar.gz", &data)?;

    Ok(local_overrides)
}

/// The control of the package, from the manifest and the composer.json of
/// the project.
fn control(manifest: &Manifest, composer: &Value) -> io::Result<Control> {
    if !manifest
        .version
        .starts_with(|character: char| character.is_ascii_digit())
    {
        return Err(io::Error::other(format!(
            "The version \"{}\" does not start with a digit, which Debian packages need: set \"version\" in extra.nativella.",
            manifest.version
        )));
    }
    if manifest.version.contains(|character: char| {
        !character.is_ascii_alphanumeric() && !".+-~".contains(character)
    }) {
        return Err(io::Error::other(format!(
            "The version \"{}\" holds characters that Debian versions cannot: letters, digits and .+-~ only.",
            manifest.version
        )));
    }

    let text = |value: &Value| value.as_str().map(one_line).filter(|text| !text.is_empty());

    let maintainer = composer["authors"]
        .as_array()
        .and_then(|authors| {
            authors.iter().find_map(|author| {
                let name = text(&author["name"])?;
                Some(match text(&author["email"]) {
                    Some(email) => format!("{name} <{email}>"),
                    None => name,
                })
            })
        })
        .unwrap_or_else(|| one_line(&manifest.name));

    Ok(Control {
        package: package_name(manifest),
        version: manifest.version.clone(),
        architecture: architecture()?,
        maintainer,
        homepage: text(&composer["homepage"]),
        description: text(&composer["description"]).unwrap_or_else(|| manifest.name.clone()),
    })
}

impl Control {
    fn text(&self, installed_kib: u64) -> String {
        let mut text = format!(
            "Package: {}\nVersion: {}\nArchitecture: {}\nMaintainer: {}\n",
            self.package, self.version, self.architecture, self.maintainer
        );
        text += &format!(
            "Installed-Size: {installed_kib}\nDepends: {DEPENDS}\nRecommends: {RECOMMENDS}\nSection: misc\nPriority: optional\n"
        );
        if let Some(homepage) = &self.homepage {
            text += &format!("Homepage: {homepage}\n");
        }
        text += &format!("Description: {}\n", self.description);

        text
    }
}

/// The entry that shows the application among the others, and opens the
/// URLs of its deep link schemes. Its name, as the executable's, is the one
/// GTK gives the windows (`WM_CLASS`, or the application id under Wayland):
/// that is how the desktop finds the icon of a window.
fn desktop_entry(manifest: &Manifest, control: &Control) -> String {
    let package = &control.package;
    let mut entry = format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec={package} %U\nIcon={package}\nTerminal=false\nStartupWMClass={package}\n",
        one_line(&manifest.name),
        control.description,
    );
    if !manifest.deep_links.is_empty() {
        let types: String = manifest
            .deep_links
            .iter()
            .map(|scheme| format!("x-scheme-handler/{scheme};"))
            .collect();
        entry += &format!("MimeType={types}\n");
    }

    entry
}

/// `value` on one line: control fields and desktop entries end at a newline.
fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A file of the package.
struct DataFile {
    /// From the root of the system, without the leading slash.
    path: String,
    contents: Vec<u8>,
    mode: u32,
}

impl DataFile {
    fn new(path: String, contents: Vec<u8>, mode: u32) -> Self {
        Self {
            path,
            contents,
            mode,
        }
    }
}

/// `files`, and the directories that hold them, as dpkg reads them:
/// relative to `./`, owned by root.
fn tar_gz(files: &[DataFile]) -> io::Result<Vec<u8>> {
    let mtime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::best()));

    let mut directories: Vec<String> = Vec::new();
    for file in files {
        let mut parent = String::from("./");
        for part in file
            .path
            .split('/')
            .rev()
            .skip(1)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            parent = format!("{parent}{part}/");
            if !directories.contains(&parent) {
                directories.push(parent.clone());
            }
        }
    }
    for directory in std::iter::once("./".to_owned()).chain(directories) {
        let mut header = header(tar::EntryType::Directory, 0o755, 0, mtime);
        archive.append_data(&mut header, &directory, io::empty())?;
    }
    for file in files {
        let mut header = header(
            tar::EntryType::Regular,
            file.mode,
            file.contents.len() as u64,
            mtime,
        );
        archive.append_data(
            &mut header,
            format!("./{}", file.path),
            file.contents.as_slice(),
        )?;
    }

    archive.into_inner()?.finish()
}

fn header(kind: tar::EntryType, mode: u32, size: u64, mtime: u64) -> tar::Header {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(kind);
    header.set_mode(mode);
    header.set_size(size);
    header.set_mtime(mtime);
    header.set_uid(0);
    header.set_gid(0);
    // Names, not only ids: dpkg reads them first.
    let _ = header.set_username("root");
    let _ = header.set_groupname("root");

    header
}

/// A member of an `ar` archive: a header of 60 bytes, then the contents,
/// padded to an even length.
fn ar_member(archive: &mut impl Write, name: &str, contents: &[u8]) -> io::Result<()> {
    let mtime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    writeln!(
        archive,
        "{name:<16}{mtime:<12}{:<6}{:<6}{:<8o}{:<10}`",
        0,
        0,
        0o100_644,
        contents.len()
    )?;
    archive.write_all(contents)?;
    if contents.len() % 2 == 1 {
        archive.write_all(b"\n")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn manifest(name: &str) -> Manifest {
        Manifest {
            name: name.to_owned(),
            identifier: "com.example.notes".to_owned(),
            version: "1.2.0".to_owned(),
            icon: None,
            window: serde_json::Map::new(),
            deep_links: vec!["notes".to_owned()],
            permissions: Vec::new(),
        }
    }

    #[test]
    fn the_package_is_named_after_the_application() {
        assert_eq!(
            package_name(&manifest("Nativella Notes")),
            "nativella-notes"
        );
        assert_eq!(package_name(&manifest("Café & Co.")), "caf-co.");
        assert_eq!(package_name(&manifest("été")), "app-t");
    }

    #[test]
    fn the_control_comes_from_composer_json() {
        let composer = json!({
            "description": "Notes,\n  on the desktop.",
            "homepage": "https://notes.example",
            "authors": [{"name": "Ada"}, {"name": "Grace", "email": "grace@example.com"}],
        });
        let notes = manifest("Notes");

        let control = control(&notes, &composer).unwrap();

        assert_eq!(
            control.text(42),
            format!(
                "Package: notes\nVersion: 1.2.0\nArchitecture: {}\nMaintainer: Ada\nInstalled-Size: 42\nDepends: {DEPENDS}\nRecommends: {RECOMMENDS}\nSection: misc\nPriority: optional\nHomepage: https://notes.example\nDescription: Notes, on the desktop.\n",
                architecture().unwrap()
            )
        );
        assert_eq!(
            desktop_entry(&notes, &control),
            "[Desktop Entry]\nType=Application\nName=Notes\nComment=Notes, on the desktop.\nExec=notes %U\nIcon=notes\nTerminal=false\nStartupWMClass=notes\nMimeType=x-scheme-handler/notes;\n"
        );
    }

    #[test]
    fn without_a_description_the_name_describes_it() {
        let control = control(&manifest("Notes"), &json!({})).unwrap();

        assert_eq!(control.description, "Notes");
        assert_eq!(control.maintainer, "Notes");
    }

    #[test]
    fn a_version_debian_cannot_take_is_refused() {
        let mut notes = manifest("Notes");

        notes.version = "v1.2.0".to_owned();
        assert!(control(&notes, &json!({})).is_err());
        notes.version = "1.2.0 beta".to_owned();
        assert!(control(&notes, &json!({})).is_err());
    }

    #[test]
    fn an_ar_member_is_padded_to_an_even_length() {
        let mut archive = Vec::new();

        ar_member(&mut archive, "debian-binary", b"2.0").unwrap();

        assert_eq!(archive.len(), 60 + 4);
        assert!(archive.starts_with(b"debian-binary   "));
        assert_eq!(&archive[48..60], b"3         `\n");
        assert!(archive.ends_with(b"2.0\n"));
    }
}

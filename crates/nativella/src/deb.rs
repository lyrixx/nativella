//! `nativella deb`: a Debian package of a project, for Debian, Ubuntu and
//! the distributions made of them. It holds the standalone executable of
//! `nativella build` in `/usr/bin`, a desktop entry and the icon, so that the
//! application shows among the others, and depends on the webview.
//!
//! A package is an `ar` archive of three members: `debian-binary`, the
//! version of the format, `control.tar.gz`, what dpkg reads, and
//! `data.tar.gz`, the files, from the root of the system.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::UNIX_EPOCH;

use flate2::Compression;
use flate2::write::GzEncoder;
use md5::{Digest, Md5};
use serde_json::Value;

use crate::apk::icon::Icon;
use crate::embedded::{self, LeftOut};
use crate::notices;
use crate::output::{Output, build_time, package_name};
use crate::project::{Manifest, Project};

/// The sizes of the icon desktops pick from, in pixels. Keep the largest in
/// sync with LARGEST in apk/icon.rs.
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

/// The version of the package: the `version` of the manifest, its
/// pre-release after a `~` rather than a `-`. dpkg sorts what follows a `~`
/// before the end, as semantic versions do a pre-release: 1.2.0~beta.1 comes
/// before 1.2.0. It would take 1.2.0-beta.1 for 1.2.0, revision beta.1, and
/// the final 1.2.0 for a downgrade.
///
/// The other dashes become dots: after a `~` each, 1.2.0-alpha-1 would come
/// before 1.2.0-alpha, and it comes after.
pub fn version(manifest: &Manifest) -> String {
    let (core, suffix) = manifest.version.split_at(
        manifest
            .version
            .find(['-', '+'])
            .unwrap_or(manifest.version.len()),
    );
    let suffix = match suffix.strip_prefix('-') {
        Some(pre_release) => format!("~{pre_release}"),
        None => suffix.to_owned(),
    };

    format!("{core}{}", suffix.replace('-', "."))
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

/// Writes the package of `project` to `output`. Returns what the executable
/// left out of the project.
pub fn build(project: &Project, output: &Path) -> io::Result<LeftOut> {
    let control = control(&project.manifest, &composer(project)?)?;

    // The executable, in memory, as the rest of the package. The previous
    // package does not ship in it, should it be inside the project.
    let mut executable = Vec::new();
    let previous: &[&Path] = if output.exists() { &[output] } else { &[] };
    let left_out = embedded::write_executable(project, &mut executable, previous)?;
    write(project, &control, executable, output)?;

    Ok(left_out)
}

/// The composer.json of `project`, null without one: a plain site may have
/// none, and the manifest describes the package then.
fn composer(project: &Project) -> io::Result<Value> {
    match fs::read(project.dir.join("composer.json")) {
        Ok(composer) => serde_json::from_slice(&composer)
            .map_err(|error| io::Error::other(format!("composer.json is invalid: {error}."))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Value::Null),
        Err(error) => Err(error),
    }
}

/// Writes the package of `project`, described by `control`, of the
/// standalone executable `executable`, to `output`.
fn write(
    project: &Project,
    control: &Control,
    executable: Vec<u8>,
    output: &Path,
) -> io::Result<()> {
    let package = &control.package;
    let icon = Icon::of(project)?;
    let mut files = vec![
        DataFile::new(format!("usr/bin/{package}"), executable, 0o755),
        DataFile::new(
            format!("usr/share/applications/{package}.desktop"),
            desktop_entry(&project.manifest, control).into_bytes(),
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
    let mtime = build_time()?
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let data = tar_gz(&files, mtime)?;
    let control_archive = tar_gz(
        &[
            DataFile::new("control".to_owned(), control_text.into_bytes(), 0o644),
            DataFile::new("md5sums".to_owned(), md5sums(&files).into_bytes(), 0o644),
        ],
        mtime,
    )?;

    let mut target = Output::create(output)?;
    let archive = target.file();
    archive.write_all(b"!<arch>\n")?;
    ar_member(archive, "debian-binary", b"2.0\n", mtime)?;
    ar_member(archive, "control.tar.gz", &control_archive, mtime)?;
    ar_member(archive, "data.tar.gz", &data, mtime)?;

    target.finish()
}

/// The control of the package, from the manifest and the composer.json of
/// the project.
fn control(manifest: &Manifest, composer: &Value) -> io::Result<Control> {
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
        version: version(manifest),
        architecture: architecture()?,
        maintainer,
        homepage: text(&composer["homepage"]),
        description: text(&composer["description"]).unwrap_or_else(|| one_line(&manifest.name)),
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

/// The MD5 of each file, as `debsums` checks them against the installed
/// files: `<digest>  <path>`, a line each.
fn md5sums(files: &[DataFile]) -> String {
    files
        .iter()
        .map(|file| {
            let digest: String = Md5::digest(&file.contents)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();

            format!("{digest}  {}\n", file.path)
        })
        .collect()
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
/// relative to `./`, owned by root, dated `mtime`.
fn tar_gz(files: &[DataFile], mtime: u64) -> io::Result<Vec<u8>> {
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
fn ar_member(archive: &mut impl Write, name: &str, contents: &[u8], mtime: u64) -> io::Result<()> {
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
            updates: None,
        }
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
        let control = control(&manifest("Notes\nPro"), &json!({})).unwrap();

        assert_eq!(control.description, "Notes Pro");
        assert_eq!(control.maintainer, "Notes Pro");
    }

    #[test]
    fn a_project_without_composer_json() {
        let root =
            Path::new(env!("OUT_DIR")).join(format!("deb-plain-test-{}", std::process::id()));
        fs::create_dir_all(root.join("notes/public")).unwrap();
        fs::write(root.join("notes/public/index.php"), "<?php").unwrap();
        let project = Project::load(&root.join("notes")).unwrap();

        let control = control(&project.manifest, &composer(&project).unwrap()).unwrap();

        assert_eq!(control.package, "notes");
        assert_eq!(control.maintainer, "Notes");
        assert_eq!(control.description, "Notes");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_pre_release_comes_before_the_release() {
        let mut notes = manifest("Notes");

        notes.version = "1.2.0-beta.1+build-7".to_owned();

        assert_eq!(
            control(&notes, &json!({})).unwrap().version,
            "1.2.0~beta.1+build.7"
        );
        notes.version = "1.2.0-alpha-1".to_owned();
        assert_eq!(version(&notes), "1.2.0~alpha.1");
        notes.version = "1.2.0+git-abc".to_owned();
        assert_eq!(version(&notes), "1.2.0+git.abc");
    }

    #[test]
    fn the_digests_of_the_files() {
        let files = [
            DataFile::new("usr/bin/notes".to_owned(), b"ELF".to_vec(), 0o755),
            DataFile::new("usr/share/doc/notes/empty".to_owned(), Vec::new(), 0o644),
        ];

        assert_eq!(
            md5sums(&files),
            "b61b2d6c6fa62903b882aaa53452c111  usr/bin/notes\nd41d8cd98f00b204e9800998ecf8427e  usr/share/doc/notes/empty\n"
        );
    }

    /// The members of the `ar` archive `archive`: their names and contents.
    fn ar_members(archive: &[u8]) -> Vec<(String, &[u8])> {
        assert!(archive.starts_with(b"!<arch>\n"));
        let mut members = Vec::new();
        let mut offset = 8;
        while offset < archive.len() {
            let header = &archive[offset..offset + 60];
            assert_eq!(&header[58..], b"`\n");
            let name = String::from_utf8(header[..16].to_vec()).unwrap();
            let size: usize = std::str::from_utf8(&header[48..58])
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            members.push((
                name.trim().to_owned(),
                &archive[offset + 60..offset + 60 + size],
            ));
            offset += 60 + size + size % 2;
        }

        members
    }

    /// A file of a `.tar.gz`, as dpkg reads it.
    struct TarFile {
        path: String,
        mode: u32,
        owner: (u64, u64),
        mtime: u64,
        content: Vec<u8>,
    }

    fn tar_files(archive: &[u8]) -> Vec<TarFile> {
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(archive));
        archive
            .entries()
            .unwrap()
            .map(|entry| {
                let mut entry = entry.unwrap();
                let header = entry.header();
                let mut file = TarFile {
                    path: header.path().unwrap().to_string_lossy().into_owned(),
                    mode: header.mode().unwrap(),
                    owner: (header.uid().unwrap(), header.gid().unwrap()),
                    mtime: header.mtime().unwrap(),
                    content: Vec::new(),
                };
                std::io::Read::read_to_end(&mut entry, &mut file.content).unwrap();

                file
            })
            .collect()
    }

    #[test]
    fn a_package_dpkg_reads() {
        let root = Path::new(env!("OUT_DIR")).join(format!("deb-test-{}", std::process::id()));
        fs::create_dir_all(root.join("project/public")).unwrap();
        fs::write(root.join("project/public/index.php"), "<?php").unwrap();
        let project = Project::load(&root.join("project")).unwrap();
        let control = control(&manifest("Notes"), &json!({"description": "Notes."})).unwrap();
        let output = root.join("notes.deb");

        write(&project, &control, b"ELF".to_vec(), &output).unwrap();

        let package = fs::read(&output).unwrap();
        let members = ar_members(&package);
        let names: Vec<&str> = members.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["debian-binary", "control.tar.gz", "data.tar.gz"]);
        assert_eq!(members[0].1, b"2.0\n");

        let control_files = tar_files(members[1].1);
        let paths: Vec<&str> = control_files
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        assert_eq!(paths, ["./", "control", "md5sums"]);
        let control_text = String::from_utf8(control_files[1].content.clone()).unwrap();
        assert!(
            control_text.starts_with("Package: notes\nVersion: 1.2.0\n"),
            "{control_text}"
        );
        let md5sums = String::from_utf8(control_files[2].content.clone()).unwrap();
        assert!(
            md5sums.starts_with("b61b2d6c6fa62903b882aaa53452c111  usr/bin/notes\n"),
            "{md5sums}"
        );

        let data = tar_files(members[2].1);
        // Directories first, each before what it holds, owned by root, all
        // of one date.
        assert_eq!(data[0].path, "./");
        assert!(data.iter().all(|file| file.owner == (0, 0)));
        assert!(data.iter().all(|file| file.mtime == data[0].mtime));
        let position = |path: &str| data.iter().position(|file| file.path == path).unwrap();
        let executable = &data[position("usr/bin/notes")];
        assert_eq!(executable.mode, 0o755);
        assert_eq!(executable.content, b"ELF");
        assert_eq!(
            data[position("usr/share/applications/notes.desktop")].mode,
            0o644
        );
        for size in ICON_SIZES {
            let icon = format!("usr/share/icons/hicolor/{size}x{size}/apps/notes.png");
            assert!(position(&icon[..icon.rfind('/').unwrap() + 1]) < position(&icon));
        }
        assert_eq!(
            md5sums.lines().count(),
            data.iter().filter(|file| !file.path.ends_with('/')).count()
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_ar_member_is_padded_to_an_even_length() {
        let mut archive = Vec::new();

        ar_member(&mut archive, "debian-binary", b"2.0", 1_700_000_000).unwrap();

        assert_eq!(archive.len(), 60 + 4);
        assert!(archive.starts_with(b"debian-binary   1700000000  "));
        assert_eq!(&archive[48..60], b"3         `\n");
        assert!(archive.ends_with(b"2.0\n"));
    }
}

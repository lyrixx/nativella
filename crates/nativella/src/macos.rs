//! `nativella build` on a Mac: the macOS app of a PHP project, a directory
//! `<name>.app` that Finder opens, the Dock shows and macOS checks.
//!
//! - `Contents/MacOS/<name>`: this executable, which carries no project, but
//!   its signature;
//! - `Contents/Resources/nativella.bundle`: the bundle of the project, which
//!   the executable finds there at startup (see embedded.rs);
//! - `Contents/Resources/AppIcon.icns`: the icon of the project, or
//!   Nativella's;
//! - `Contents/Info.plist`: its name, identifier and version, the reasons
//!   macOS gives the user for what its pages may use, and the schemes of its
//!   deep links, which macOS registers once it has seen the app;
//! - signed ad hoc, as `codesign -s -` signs (see apple/signature.rs): the
//!   signature binds the Info.plist and the resources to the executable, so
//!   macOS takes the app as a whole, under its identifier. No developer signs
//!   it: a Mac runs it once the user allows it, as it does with the apps from
//!   the Internet that Apple did not notarize.
//!
//! Without a usage description, the webview does not even show the camera
//! and the microphone to the pages: `navigator.mediaDevices` is undefined.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;

use plist::{Dictionary, Value};

use crate::apk::icon::Icon;
use crate::apple::macho::Executable;
use crate::apple::signature::{self, Signer};
use crate::apple::{Layout, Staging, bundle_version, replace_app};
use crate::embedded::{self, LeftOut, PACKAGED_BUNDLE};
use crate::project::{Manifest, Permission, Project};

/// The icon of the app, in `Contents/Resources/`, as `CFBundleIconFile`
/// names it, without its extension.
const ICON: &str = "AppIcon";

/// The images of the `.icns` file: their type, and their size in pixels.
/// PNG images, which macOS takes in every type since 10.7, for each size
/// Finder and the Dock draw, at 1× and 2×.
const ICNS_IMAGES: [(&[u8; 4], u32); 10] = [
    (b"icp4", 16),
    (b"ic11", 32),
    (b"icp5", 32),
    (b"ic12", 64),
    (b"ic07", 128),
    (b"ic13", 256),
    (b"ic08", 256),
    (b"ic14", 512),
    (b"ic09", 512),
    (b"ic10", 1024),
];

/// Writes the macOS app of `project` to `output`, a `.app` directory, in
/// place of a previous one. Returns what it left out of the project.
pub fn package(project: &Project, output: &Path) -> io::Result<LeftOut> {
    package_from(&mut embedded::nativella_executable()?, project, output)
}

/// Writes the macOS app of `project`, whose executable is `nativella`, to
/// `output`.
fn package_from(
    nativella: &mut impl Read,
    project: &Project,
    output: &Path,
) -> io::Result<LeftOut> {
    let manifest = &project.manifest;
    let version = bundle_version(&manifest.version)?;
    if output
        .extension()
        .is_none_or(|extension| extension != "app")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} must end with .app: a macOS app is a .app directory.",
                output.display()
            ),
        ));
    }

    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let name = output
        .file_name()
        .ok_or_else(|| invalid("The output has no file name."))?;
    // Next to the output, for the app to move there in one rename.
    let staging = Staging::create(parent.join(format!(
        ".{}.nativella-{}",
        name.to_string_lossy(),
        std::process::id()
    )))?;
    let app_dir = staging.0.join(name);
    let contents = Layout::Macos.contents(&app_dir);
    fs::create_dir_all(contents.join("MacOS"))?;
    fs::create_dir_all(contents.join("Resources"))?;

    let executable = app_name(manifest);
    let executable_path = contents.join(Layout::Macos.executable(&executable));
    let mut target = BufWriter::new(create_executable(&executable_path)?);
    io::copy(nativella, &mut target)?;
    target.flush()?;
    drop(target);
    let minimum_os = Executable::open(&executable_path)?.minimum_os();

    // The bundle leaves out what the packaging writes, should it be inside
    // the project: the app being made, and a previous one.
    let mut excluded = vec![staging.0.as_path()];
    if output.exists() {
        excluded.push(output);
    }
    let resources = contents.join("Resources");
    let mut bundle = BufWriter::new(File::create(resources.join(PACKAGED_BUNDLE))?);
    let left_out = embedded::write_bundle(project, &mut bundle, &excluded)?;
    bundle.flush()?;
    drop(bundle);

    fs::write(
        resources.join(format!("{ICON}.icns")),
        icns(&Icon::of(project)?),
    )?;
    plist::to_file_xml(
        contents.join("Info.plist"),
        &info(manifest, &executable, &version, minimum_os.as_deref()),
    )
    .map_err(|error| io::Error::other(format!("Cannot write the Info.plist: {error}.")))?;

    signature::sign(&app_dir, Layout::Macos, &Signer::AdHoc)?;
    replace_app(&app_dir, output, Layout::Macos)?;

    Ok(left_out)
}

/// The name of the app, as Finder shows it, and of its executable: the
/// name of the application, but `/` and `:`, which macOS does not take in a
/// file name, and a leading `.`, which would hide it. A name of dots only
/// leaves the last segment of the identifier, as for the other packages.
pub fn app_name(manifest: &Manifest) -> String {
    let name = manifest.name.replace(['/', ':'], "-");
    let name = name.trim_start_matches('.').trim();
    if name.is_empty() {
        return manifest
            .identifier
            .rsplit('.')
            .next()
            .unwrap_or_default()
            .to_owned();
    }

    name.to_owned()
}

/// The Info.plist of the app.
fn info(
    manifest: &Manifest,
    executable: &str,
    version: &str,
    minimum_os: Option<&str>,
) -> Dictionary {
    let mut info = Dictionary::new();
    for (key, value) in [
        ("CFBundleDevelopmentRegion", "en"),
        ("CFBundleDisplayName", manifest.name.as_str()),
        ("CFBundleExecutable", executable),
        ("CFBundleIconFile", ICON),
        ("CFBundleIdentifier", manifest.identifier.as_str()),
        ("CFBundleInfoDictionaryVersion", "6.0"),
        ("CFBundleName", manifest.name.as_str()),
        ("CFBundlePackageType", "APPL"),
        ("CFBundleShortVersionString", version),
        ("CFBundleVersion", version),
    ] {
        info.insert(key.to_owned(), value.into());
    }
    if let Some(minimum_os) = minimum_os {
        info.insert("LSMinimumSystemVersion".to_owned(), minimum_os.into());
    }
    info.insert("NSHighResolutionCapable".to_owned(), true.into());
    for (key, reason) in usage_descriptions(&manifest.permissions, &manifest.name) {
        info.insert(key.to_owned(), reason.into());
    }
    if !manifest.deep_links.is_empty() {
        let mut url_type = Dictionary::new();
        url_type.insert(
            "CFBundleURLName".to_owned(),
            manifest.identifier.clone().into(),
        );
        url_type.insert(
            "CFBundleURLSchemes".to_owned(),
            manifest
                .deep_links
                .iter()
                .map(|scheme| Value::from(scheme.as_str()))
                .collect::<Vec<_>>()
                .into(),
        );
        info.insert(
            "CFBundleURLTypes".to_owned(),
            vec![Value::from(url_type)].into(),
        );
    }

    info
}

/// The keys of the Info.plist that hold the reason macOS gives the user of
/// the app `name` when its pages use the camera, the microphone or the
/// location, with the reason, for what it asks for. Computers ignore the
/// phone APIs. Keep in sync with the Info.plist of the nativella command,
/// next to Cargo.toml, which `nativella run` has.
fn usage_descriptions(asked: &[Permission], name: &str) -> BTreeMap<&'static str, String> {
    let mut descriptions = BTreeMap::new();
    if asked.contains(&Permission::Camera) {
        descriptions.insert(
            "NSCameraUsageDescription",
            format!("{name} uses the camera."),
        );
    }
    if asked.contains(&Permission::Microphone) {
        descriptions.insert(
            "NSMicrophoneUsageDescription",
            format!("{name} uses the microphone."),
        );
    }
    if asked.contains(&Permission::Geolocation) {
        // The first since macOS 10.14, the second since macOS 11.
        for key in [
            "NSLocationUsageDescription",
            "NSLocationWhenInUseUsageDescription",
        ] {
            descriptions.insert(key, format!("{name} uses the location of the computer."));
        }
    }

    descriptions
}

/// `icon` as an `.icns` file: its magic and its length, then each image,
/// its type and its length, then its PNG file.
fn icns(icon: &Icon) -> Vec<u8> {
    let mut pngs = BTreeMap::new();
    let mut images = Vec::new();
    for (kind, size) in ICNS_IMAGES {
        let png = pngs.entry(size).or_insert_with(|| icon.png(size));
        images.extend(kind);
        images.extend(
            u32::try_from(8 + png.len())
                .expect("An icon is small.")
                .to_be_bytes(),
        );
        images.extend(png.iter());
    }

    let mut file = Vec::with_capacity(8 + images.len());
    file.extend(b"icns");
    file.extend(
        u32::try_from(8 + images.len())
            .expect("An icon is small.")
            .to_be_bytes(),
    );
    file.extend(images);

    file
}

/// Creates the executable file `path`.
fn create_executable(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o755);
    }

    options.open(path)
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rsa::sha2::{Digest, Sha256};

    use super::*;
    use crate::apple::macho;
    use crate::apple::signature::tests::{blobs, parse};
    use crate::embedded::{Bundle, app_bundle_of};

    /// A project asking for the camera and the location, with a deep link
    /// and a local override, which does not ship.
    fn project(name: &str) -> (Project, PathBuf) {
        let root =
            Path::new(env!("OUT_DIR")).join(format!("macos-test-{name}-{}", std::process::id()));
        let project_dir = root.join("note-taker");
        fs::create_dir_all(project_dir.join("public")).unwrap();
        fs::write(project_dir.join("public/index.php"), "<?php echo 'hi';").unwrap();
        fs::write(
            project_dir.join("composer.json"),
            r#"{"name": "acme/note-taker", "extra": {"nativella": {"name": "Note Taker", "version": "1.2.3-beta.1", "deepLinks": ["notes"], "permissions": ["camera", "geolocation", "haptics"]}}}"#,
        )
        .unwrap();
        fs::write(project_dir.join(".env.local"), "SECRET=1").unwrap();

        (Project::load(&project_dir).unwrap(), project_dir)
    }

    fn sha256(path: &Path) -> [u8; 32] {
        Sha256::digest(fs::read(path).unwrap()).into()
    }

    #[test]
    fn an_app() {
        let (project, project_dir) = project("app");
        let output = project_dir.join("dist/Note Taker.app");
        let nativella = macho::tests::executable(true);

        let left_out = package_from(&mut &nativella[..], &project, &output).unwrap();

        assert_eq!(left_out.local_overrides, [".env.local"]);
        let contents = output.join("Contents");
        let executable = contents.join("MacOS/Note Taker");
        let bundle = app_bundle_of(&executable).unwrap();
        assert_eq!(bundle, contents.join("Resources/nativella.bundle"));
        let bundle = Bundle::read(File::open(bundle).unwrap()).unwrap().unwrap();
        assert_eq!(bundle.manifest().identifier, "dev.nativella.note-taker");

        let info: Dictionary = plist::from_file(contents.join("Info.plist")).unwrap();
        let string = |key: &str| info.get(key).and_then(Value::as_string);
        assert_eq!(string("CFBundleExecutable"), Some("Note Taker"));
        assert_eq!(
            string("CFBundleIdentifier"),
            Some("dev.nativella.note-taker")
        );
        assert_eq!(string("CFBundleName"), Some("Note Taker"));
        assert_eq!(string("CFBundleShortVersionString"), Some("1.2.3"));
        assert_eq!(string("CFBundleIconFile"), Some("AppIcon"));
        assert_eq!(string("CFBundlePackageType"), Some("APPL"));
        assert_eq!(string("LSMinimumSystemVersion"), Some("11.0"));
        assert_eq!(
            string("NSCameraUsageDescription"),
            Some("Note Taker uses the camera.")
        );
        assert_eq!(
            string("NSLocationUsageDescription"),
            Some("Note Taker uses the location of the computer.")
        );
        assert_eq!(string("NSMicrophoneUsageDescription"), None);
        let url_types = info["CFBundleURLTypes"].as_array().unwrap();
        let url_type = url_types[0].as_dictionary().unwrap();
        assert_eq!(
            url_type["CFBundleURLSchemes"].as_array().unwrap(),
            &[Value::from("notes")]
        );

        // Each size, from the icon of Nativella.
        let icns = fs::read(contents.join("Resources/AppIcon.icns")).unwrap();
        assert_eq!(&icns[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
            icns.len()
        );
        let mut kinds = Vec::new();
        let mut offset = 8;
        while offset < icns.len() {
            kinds.push(String::from_utf8(icns[offset..offset + 4].to_vec()).unwrap());
            assert!(icns[offset + 8..].starts_with(b"\x89PNG"));
            offset += u32::from_be_bytes(icns[offset + 4..offset + 8].try_into().unwrap()) as usize;
        }
        assert_eq!(kinds.len(), ICNS_IMAGES.len());
        assert_eq!(kinds[0], "icp4");

        // Signed ad hoc, as codesign signs: the code directory binds the
        // Info.plist and the seal of the resources.
        let blobs = blobs(&executable);
        assert_eq!(blobs.keys().copied().collect::<Vec<_>>(), [0, 2, 0x10000]);
        let directory = parse(&blobs[&0]);
        assert_eq!(directory.flags, 0x2, "Ad hoc.");
        assert_eq!(directory.identifier, "dev.nativella.note-taker");
        assert_eq!(directory.special[0], sha256(&contents.join("Info.plist")));
        assert_eq!(
            directory.special[2],
            sha256(&contents.join("_CodeSignature/CodeResources"))
        );
        let seal: Dictionary =
            plist::from_file(contents.join("_CodeSignature/CodeResources")).unwrap();
        let sealed: Vec<_> = seal["files2"]
            .as_dictionary()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            sealed,
            ["Resources/AppIcon.icns", "Resources/nativella.bundle"]
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&executable).unwrap().permissions().mode() & 0o777,
                0o755
            );
        }

        // Again: the app replaces the previous one, which does not ship.
        package_from(&mut &nativella[..], &project, &output).unwrap();
        assert!(executable.is_file());

        fs::remove_dir_all(project_dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn an_app_is_a_directory_of_its_own() {
        let (project, project_dir) = project("output");
        let nativella = macho::tests::executable(true);

        let error =
            package_from(&mut &nativella[..], &project, &project_dir.join("notes")).unwrap_err();
        assert!(error.to_string().contains(".app"), "{error}");

        // Never in place of a directory that is not an app.
        fs::create_dir_all(project_dir.join("Documents.app/Contents")).unwrap();
        let error = package_from(
            &mut &nativella[..],
            &project,
            &project_dir.join("Documents.app"),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);

        fs::remove_dir_all(project_dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn the_name_of_an_app_is_a_file_name() {
        let manifest = |name: &str| Manifest {
            name: name.to_owned(),
            ..project("names").0.manifest
        };

        assert_eq!(app_name(&manifest("Note Taker")), "Note Taker");
        assert_eq!(app_name(&manifest("Notes: 1/2")), "Notes- 1-2");
        assert_eq!(app_name(&manifest(".hidden")), "hidden");
        assert_eq!(app_name(&manifest("..")), "note-taker");

        fs::remove_dir_all(
            Path::new(env!("OUT_DIR")).join(format!("macos-test-names-{}", std::process::id())),
        )
        .unwrap();
    }
}

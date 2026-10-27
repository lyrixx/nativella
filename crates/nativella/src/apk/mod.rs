//! `nativella android`: the Android application of a PHP project, made from
//! the template APK, without any Android tool.
//!
//! The template is the application library, PHP linked in, with the Java
//! side of Tauri and of its plugins: an APK built once per release, without
//! any project, for the package `dev.nativella.runtime`, the identifier of
//! tauri.conf.json (`castor mobile:android-template`). An application is the
//! template:
//!
//! - with the bundle of the project as the asset [`PACKAGED_BUNDLE`], which the
//!   library reads at startup (see embedded.rs);
//! - with the identity of the project in its manifest: its application ID,
//!   made of its identifier, its name, its version, and the permissions of
//!   the phone APIs it asks for, not the others (see manifest.rs);
//! - with its icon, or Nativella's, as the launcher icon, in place of the
//!   files of the template's, which its resource table names (see icon.rs and
//!   resources.rs);
//! - aligned, then signed (see ../zip.rs and signing.rs).
//!
//! Its classes stay in the package of the template, which the library names
//! in its JNI functions: only the application ID changes, as when the
//! `applicationId` of a Gradle build differs from its `namespace`.

pub(crate) mod icon;
mod manifest;
mod resources;
mod signing;
mod string_pool;

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;

use self::icon::Icon;
pub use self::signing::Key;
use crate::embedded::{self, PACKAGED_BUNDLE};
use crate::project::{Permission, Project};
use crate::zip;

const MANIFEST: &str = "AndroidManifest.xml";
const RESOURCES: &str = "resources.arsc";

/// The screen densities a launcher icon may be drawn for, in dots per inch:
/// from ldpi to xxxhdpi.
const DENSITIES: std::ops::RangeInclusive<u16> = 120..=640;

/// Writes the APK of `project` to `output`, made from `template` and signed
/// with `key`. Returns the local overrides it left out, relative to the
/// project.
pub fn package(
    project: &Project,
    template: &Path,
    output: &Path,
    key: &Key,
) -> io::Result<Vec<String>> {
    let application_id = application_id(&project.manifest.identifier)?;
    let version_code = version_code(&project.manifest.version)?;
    let icon = Icon::of(project)?;

    let mut template_file = File::open(template).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("Cannot read the template {}: {error}.", template.display()),
        )
    })?;
    let entries = zip::entries(&mut template_file)?;
    let asset = format!("assets/{PACKAGED_BUNDLE}");
    if entries.iter().any(|entry| entry.name == asset) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} is an application already, not the template: take nativella-android-template-arm64.apk, from the release of this nativella command.",
                template.display()
            ),
        ));
    }
    let manifest = entries
        .iter()
        .find(|entry| entry.name == MANIFEST)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "The template has no AndroidManifest.xml.",
            )
        })?;
    let manifest = zip::read(&mut template_file, manifest)?;
    // Built for the version of the crate: see tauri.conf.json.
    let nativella_version = env!("CARGO_PKG_VERSION");
    if let Some(template_version) = manifest::version_name(&manifest)?
        && template_version != nativella_version
    {
        eprintln!(
            "Warning: the template is the one of Nativella {template_version}, this command the one of {nativella_version}: take both from the same release."
        );
    }
    let launcher_icons = launcher_icons(&mut template_file, &entries, &manifest, &icon)?;
    let removed_permissions = removed_permissions(&project.manifest.permissions);
    let manifest = manifest::rewrite(
        &manifest,
        &manifest::Identity {
            application_id: &application_id,
            label: &project.manifest.name,
            version_code,
            version_name: &project.manifest.version,
            removed_permissions: &removed_permissions,
            removed_features: &removed_features(&project.manifest.permissions),
        },
    )?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut apk = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(output)?;

    let mut writer = zip::Writer::new(&mut apk);
    for entry in &entries {
        if entry.name == MANIFEST {
            writer.add_deflated(MANIFEST, &manifest)?;
        } else if let Some((_, png)) = launcher_icons.iter().find(|(path, _)| *path == entry.name) {
            // Stored, as aapt2 stores images.
            writer.add_stored_with(&entry.name, |out| out.write_all(png))?;
        } else if !is_v1_signature(&entry.name) {
            writer.copy(entry, &mut template_file)?;
        }
    }
    // Stored, for the library to read it in place, and written as it goes.
    // It leaves out what the packaging writes and reads, should it be inside
    // the project: the APK, which exists by now, the template and the key.
    let mut local_overrides = Vec::new();
    writer.add_stored_with(&asset, |out| {
        local_overrides = embedded::write_bundle(project, out, &[output, template, key.path()])?;
        Ok(())
    })?;
    let unfinished = writer.finish()?;
    signing::sign(&mut apk, &unfinished, key)?;

    Ok(local_overrides)
}

/// The launcher icons of the template, the files of the icon of its manifest
/// `manifest`, one per screen density, each with `icon` drawn for it: by
/// path. Tauri's Android project has PNG files, and no adaptive icon.
fn launcher_icons(
    template: &mut File,
    entries: &[zip::Entry],
    manifest: &[u8],
    icon: &Icon,
) -> io::Result<Vec<(String, Vec<u8>)>> {
    let table = entries
        .iter()
        .find(|entry| entry.name == RESOURCES)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "The template has no resources.arsc.",
            )
        })?;
    let files = resources::files(&zip::read(template, table)?, manifest::icon(manifest)?)?;
    if files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "The launcher icon of the template has no file.",
        ));
    }

    files
        .into_iter()
        .map(|file| {
            if !file.path.ends_with(".png")
                || !DENSITIES.contains(&file.density)
                || !entries.iter().any(|entry| entry.name == file.path)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "The launcher icon of the template, {}, is not the PNG file of a screen density.",
                        file.path
                    ),
                ));
            }
            let png = icon.launcher(file.density);

            Ok((file.path, png))
        })
        .collect()
}

/// The application ID of `identifier`: the identifier, its dashes as
/// underscores, which Android accepts in an application ID, not dashes.
/// Android also wants two segments at least, each starting with a letter.
fn application_id(identifier: &str) -> io::Result<String> {
    let application_id = identifier.replace('-', "_");
    let segments: Vec<&str> = application_id.split('.').collect();
    let valid = segments.len() >= 2
        && segments.iter().all(|segment| {
            segment.starts_with(|char: char| char.is_ascii_alphabetic())
                && segment
                    .chars()
                    .all(|char| char.is_ascii_alphanumeric() || char == '_')
        });

    if valid {
        return Ok(application_id);
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "The identifier \"{identifier}\" cannot be the application ID of an Android application: set extra.nativella.identifier in composer.json to two segments at least, each starting with a letter, such as com.acme.notes."
        ),
    ))
}

/// The Android permissions the template has for the phone APIs the
/// application does not ask for, and no API it asks for needs: the template
/// has them all.
fn removed_permissions(asked: &[Permission]) -> Vec<&'static str> {
    removed(asked, android_permissions)
}

/// The same for the features of the manifest: `uses-feature` elements.
fn removed_features(asked: &[Permission]) -> Vec<&'static str> {
    removed(asked, android_features)
}

/// What `needs` lists for the APIs not in `asked`, once, less what the APIs
/// in `asked` need: two APIs may need the same permission.
fn removed(
    asked: &[Permission],
    needs: fn(Permission) -> &'static [&'static str],
) -> Vec<&'static str> {
    let kept: Vec<&str> = asked
        .iter()
        .flat_map(|&permission| needs(permission))
        .copied()
        .collect();
    let mut removed = Vec::new();
    for &permission in Permission::ALL
        .iter()
        .filter(|permission| !asked.contains(permission))
    {
        for &name in needs(permission) {
            if !kept.contains(&name) && !removed.contains(&name) {
                removed.push(name);
            }
        }
    }

    removed
}

/// The Android permissions of a phone API, which its plugin brings to the
/// manifest. Keep in sync with the AndroidManifest.xml of the plugins, and of
/// their libraries: androidx.biometric declares its own. Those of the pages
/// (`camera`, `microphone`) no plugin declares: .castor/mobile.php adds them
/// to the manifest of the template.
fn android_permissions(permission: Permission) -> &'static [&'static str] {
    match permission {
        Permission::BarcodeScanner | Permission::Camera => &["android.permission.CAMERA"],
        Permission::Biometrics => &[
            "android.permission.USE_BIOMETRIC",
            "android.permission.USE_FINGERPRINT",
        ],
        Permission::Geolocation => &[
            "android.permission.ACCESS_COARSE_LOCATION",
            "android.permission.ACCESS_FINE_LOCATION",
        ],
        Permission::Haptics => &["android.permission.VIBRATE"],
        Permission::Microphone => &[
            "android.permission.MODIFY_AUDIO_SETTINGS",
            "android.permission.RECORD_AUDIO",
        ],
    }
}

/// The hardware a phone API requires: Google Play hides the application from
/// the phones that lack it. The camera plugin asks for one.
fn android_features(permission: Permission) -> &'static [&'static str] {
    match permission {
        Permission::BarcodeScanner | Permission::Camera => &["android.hardware.camera.any"],
        Permission::Biometrics
        | Permission::Geolocation
        | Permission::Haptics
        | Permission::Microphone => &[],
    }
}

/// The JAR signature of a template that Gradle signed: it would no longer
/// match, and the v2 signature replaces it.
fn is_v1_signature(name: &str) -> bool {
    name.strip_prefix("META-INF/").is_some_and(|file| {
        !file.contains('/')
            && (file == "MANIFEST.MF"
                || [".SF", ".RSA", ".DSA", ".EC"]
                    .iter()
                    .any(|extension| file.ends_with(extension)))
    })
}

/// The greatest version code Google Play accepts.
const MAX_VERSION_CODE: u32 = 2_100_000_000;

/// The version code of `version`, as Tauri computes it: major × 1,000,000 +
/// minor × 1,000 + patch. Android updates an application only with a greater
/// one.
fn version_code(version: &str) -> io::Result<u32> {
    let invalid = |reason: &str| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "The version \"{version}\" cannot number an Android application (its version code, major × 1,000,000 + minor × 1,000 + patch): {reason}."
            ),
        )
    };

    // Without its pre-release or build suffix: 1.2.3-beta.1+abc.
    let core = version.split(['-', '+']).next().unwrap_or(version);
    let parts = core
        .split('.')
        .map(str::parse)
        .collect::<Result<Vec<u32>, _>>()
        .map_err(|_| invalid("it is not major.minor.patch"))?;
    let (major, minor, patch) = match parts[..] {
        [major] => (major, 0, 0),
        [major, minor] => (major, minor, 0),
        [major, minor, patch] => (major, minor, patch),
        _ => return Err(invalid("it is not major.minor.patch")),
    };
    if minor >= 1000 || patch >= 1000 {
        return Err(invalid(
            "the minor version and the patch must stay under 1000",
        ));
    }

    major
        .checked_mul(1_000_000)
        .and_then(|code| code.checked_add(minor * 1000 + patch))
        .filter(|&code| code <= MAX_VERSION_CODE)
        .ok_or_else(|| {
            invalid("Google Play accepts 2,100,000,000 at most, a major version of 2100")
        })
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;

    use flate2::read::GzDecoder;
    use p256::ecdsa::SigningKey;
    use p256::pkcs8::{EncodePrivateKey, LineEnding};

    use super::*;

    #[test]
    fn application_ids() {
        assert_eq!(
            application_id("dev.nativella.note-taker").unwrap(),
            "dev.nativella.note_taker"
        );
        assert_eq!(
            application_id("com.Acme.notes2").unwrap(),
            "com.Acme.notes2"
        );

        for identifier in ["notes", "com.acme.2048", "com.-acme", "com..acme"] {
            let error = application_id(identifier).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{identifier}");
            assert!(error.to_string().contains("extra.nativella.identifier"));
        }
    }

    #[test]
    fn version_codes() {
        assert_eq!(version_code("0.1.1").unwrap(), 1001);
        assert_eq!(version_code("2.3.4-beta.1+abc").unwrap(), 2_003_004);
        assert_eq!(version_code("3").unwrap(), 3_000_000);
        assert_eq!(version_code("2100.0.0").unwrap(), MAX_VERSION_CODE);

        for version in ["", "1.x", "1.2.3.4", "v1.0.0"] {
            let error = version_code(version).unwrap_err().to_string();
            assert!(
                error.contains("not major.minor.patch"),
                "{version}: {error}"
            );
        }
        let error = version_code("1.1000.0").unwrap_err().to_string();
        assert!(error.contains("under 1000"), "{error}");
        let error = version_code("2101.0.0").unwrap_err().to_string();
        assert!(error.contains("2,100,000,000 at most"), "{error}");
    }

    #[test]
    fn only_the_permissions_asked_for_stay() {
        assert_eq!(
            removed_permissions(&[Permission::Geolocation]),
            [
                "android.permission.CAMERA",
                "android.permission.USE_BIOMETRIC",
                "android.permission.USE_FINGERPRINT",
                "android.permission.VIBRATE",
                "android.permission.MODIFY_AUDIO_SETTINGS",
                "android.permission.RECORD_AUDIO",
            ]
        );
        assert!(removed_permissions(Permission::ALL).is_empty());
    }

    #[test]
    fn a_permission_two_apis_need_stays_for_either() {
        // The camera is the barcode scanner's, and the pages'.
        for asked in [Permission::BarcodeScanner, Permission::Camera] {
            let removed = removed_permissions(&[asked, Permission::Haptics]);

            assert!(!removed.contains(&"android.permission.CAMERA"), "{asked:?}");
            assert!(
                !removed.contains(&"android.permission.VIBRATE"),
                "{asked:?}"
            );
            assert!(
                removed.contains(&"android.permission.RECORD_AUDIO"),
                "{asked:?}"
            );
        }
    }

    #[test]
    fn only_the_hardware_asked_for_stays() {
        assert_eq!(
            removed_features(&[Permission::Microphone]),
            ["android.hardware.camera.any"]
        );
        assert!(removed_features(&[Permission::BarcodeScanner]).is_empty());
        assert!(removed_features(Permission::ALL).is_empty());
    }

    #[test]
    fn v1_signatures() {
        for name in [
            "META-INF/MANIFEST.MF",
            "META-INF/CERT.SF",
            "META-INF/CERT.RSA",
            "META-INF/KEY.EC",
        ] {
            assert!(is_v1_signature(name), "{name}");
        }
        for name in [
            "META-INF/services/foo",
            "META-INF/androidx.core_core.version",
            "assets/CERT.SF",
        ] {
            assert!(!is_v1_signature(name), "{name}");
        }
    }

    /// The launcher icon of the template: one PNG file per density, the
    /// second one named as a release build shortens it.
    const LAUNCHER_ICONS: [(&str, u16); 2] = [
        ("res/mipmap-mdpi-v4/ic_launcher.png", 160),
        ("res/o-.png", 640),
    ];

    /// A template APK: its manifest, its resource table, the files of its
    /// launcher icon, `launcher_icons`, and a library.
    fn template(path: &Path, launcher_icons: &[(&str, u16)]) {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .unwrap();
        let mut writer = zip::Writer::new(&mut file);
        writer
            .add_deflated(MANIFEST, &manifest::tests::template(false))
            .unwrap();
        let table = resources::tests::table(0x7f0e_0000, launcher_icons);
        writer
            .add_stored_with(RESOURCES, |out| out.write_all(&table))
            .unwrap();
        for (path, _) in launcher_icons {
            writer
                .add_stored_with(path, |out| out.write_all(b"The icon of the template"))
                .unwrap();
        }
        writer
            .add_stored_with("lib/arm64-v8a/libnativella_lib.so", |out| {
                out.write_all(b"ELF")
            })
            .unwrap();
        let unfinished = writer.finish().unwrap();
        let end = zip::end_of_central_directory(&unfinished, unfinished.entries_length).unwrap();
        file.write_all(&unfinished.central_directory).unwrap();
        file.write_all(&end).unwrap();
    }

    /// The paths of the files of the bundle in `apk`.
    fn bundled(apk: &Path) -> Vec<PathBuf> {
        let mut file = File::open(apk).unwrap();
        let entries = zip::entries(&mut file).unwrap();
        let asset = entries
            .iter()
            .find(|entry| entry.name == format!("assets/{PACKAGED_BUNDLE}"))
            .unwrap();
        let bundle = zip::read(&mut file, asset).unwrap();

        // The archive comes first, its description after it.
        let mut archive = tar::Archive::new(GzDecoder::new(&bundle[..]));
        archive
            .entries()
            .unwrap()
            .map(|entry| entry.unwrap().path().unwrap().into_owned())
            .collect()
    }

    #[test]
    fn an_application_of_the_template() {
        let root = Path::new(env!("OUT_DIR")).join(format!("apk-test-{}", std::process::id()));
        let project_dir = root.join("note-taker");
        fs::create_dir_all(project_dir.join("public")).unwrap();
        fs::write(project_dir.join("public/index.php"), "<?php echo 'hi';").unwrap();
        fs::write(
            project_dir.join("composer.json"),
            r#"{"name": "acme/note-taker"}"#,
        )
        .unwrap();
        fs::write(project_dir.join(".env.local"), "SECRET=1").unwrap();
        // The key, the template and the APK inside the project: none of them
        // ships.
        let pem = SigningKey::from_slice(&[7; 32])
            .unwrap()
            .to_pkcs8_pem(LineEnding::LF)
            .unwrap();
        fs::write(project_dir.join("key.pem"), pem.as_bytes()).unwrap();
        template(&project_dir.join("template.apk"), &LAUNCHER_ICONS);
        let project = Project::load(&project_dir).unwrap();
        let key = Key::load(&project_dir.join("key.pem")).unwrap();
        let output = project_dir.join("dist/note-taker.apk");

        assert_eq!(
            package(&project, &project_dir.join("template.apk"), &output, &key).unwrap(),
            [".env.local"],
            "Local overrides never ship."
        );

        let mut bundled = bundled(&output);
        bundled.sort();
        assert_eq!(
            bundled,
            [
                PathBuf::from("composer.json"),
                PathBuf::from("public/index.php"),
            ]
        );
        let mut file = File::open(&output).unwrap();
        let entries = zip::entries(&mut file).unwrap();
        let manifest = entries.iter().find(|entry| entry.name == MANIFEST).unwrap();
        let manifest = zip::read(&mut file, manifest).unwrap();
        let (_, attributes) = &manifest::tests::elements(&manifest)[0];
        assert!(
            attributes.contains(&("package".to_owned(), "dev.nativella.note_taker".to_owned()))
        );
        let library = entries
            .iter()
            .find(|entry| entry.name.ends_with(".so"))
            .unwrap();
        assert_eq!(zip::read(&mut file, library).unwrap(), b"ELF");
        // Nativella's icon, the project having none.
        let icon = Icon::of(&project).unwrap();
        for (path, density) in LAUNCHER_ICONS {
            let entry = entries.iter().find(|entry| entry.name == path).unwrap();
            assert_eq!(zip::read(&mut file, entry).unwrap(), icon.launcher(density));
        }

        // An application is no template.
        let error = package(&project, &output, &root.join("again.apk"), &key).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);

        // Nor is an APK with an adaptive icon, which Nativella does not draw.
        let adaptive = root.join("adaptive.apk");
        template(
            &adaptive,
            &[
                LAUNCHER_ICONS[0],
                ("res/mipmap-anydpi-v26/ic_launcher.xml", 0xfffe),
            ],
        );
        let error = package(&project, &adaptive, &root.join("adaptive-app.apk"), &key).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("ic_launcher.xml"), "{error}");

        fs::remove_dir_all(root).unwrap();
    }
}

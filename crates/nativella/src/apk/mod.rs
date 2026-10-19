//! `nativella android`: the Android application of a PHP project, made from
//! the template APK, without any Android tool.
//!
//! The template is the application library, PHP linked in, with the Java
//! side of Tauri and of its plugins: an APK built once per release, without
//! any project, for the package `dev.nativella.runtime`, the identifier of
//! tauri.conf.json (`castor mobile:android-template`). An application is the
//! template:
//!
//! - with the bundle of the project as the asset [`ANDROID_ASSET`], which the
//!   library reads at startup (see embedded.rs);
//! - with the identity of the project in its manifest: its application ID,
//!   made of its identifier, its name, its version, and the permissions of
//!   the phone APIs it asks for, not the others (see manifest.rs);
//! - aligned, then signed (see zip.rs and signing.rs).
//!
//! Its classes stay in the package of the template, which the library names
//! in its JNI functions: only the application ID changes, as when the
//! `applicationId` of a Gradle build differs from its `namespace`.

mod manifest;
mod signing;
mod string_pool;
mod zip;

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;

pub use self::signing::Key;
use crate::embedded::{self, ANDROID_ASSET};
use crate::project::{Permission, Project};

const MANIFEST: &str = "AndroidManifest.xml";

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

    let mut template_file = File::open(template).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("Cannot read the template {}: {error}.", template.display()),
        )
    })?;
    let entries = zip::entries(&mut template_file)?;
    let asset = format!("assets/{ANDROID_ASSET}");
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
    let removed_permissions = removed_permissions(&project.manifest.permissions);
    let manifest = manifest::rewrite(
        &manifest,
        &manifest::Identity {
            application_id: &application_id,
            label: &project.manifest.name,
            version_code,
            version_name: &project.manifest.version,
            removed_permissions: &removed_permissions,
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

/// The Android permissions of the phone APIs the application does not ask
/// for: the template has them all.
fn removed_permissions(asked: &[Permission]) -> Vec<&'static str> {
    Permission::ALL
        .iter()
        .filter(|permission| !asked.contains(permission))
        .flat_map(|&permission| android_permissions(permission))
        .copied()
        .collect()
}

/// The Android permissions of a phone API, which its plugin brings to the
/// manifest. Keep in sync with the AndroidManifest.xml of the plugins, and of
/// their libraries: androidx.biometric declares its own.
fn android_permissions(permission: Permission) -> &'static [&'static str] {
    match permission {
        Permission::Biometrics => &[
            "android.permission.USE_BIOMETRIC",
            "android.permission.USE_FINGERPRINT",
        ],
        Permission::Geolocation => &[
            "android.permission.ACCESS_COARSE_LOCATION",
            "android.permission.ACCESS_FINE_LOCATION",
        ],
        Permission::Haptics => &["android.permission.VIBRATE"],
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
                "android.permission.USE_BIOMETRIC",
                "android.permission.USE_FINGERPRINT",
                "android.permission.VIBRATE",
            ]
        );
        assert!(removed_permissions(Permission::ALL).is_empty());
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

    /// A template APK: its manifest, and a library.
    fn template(path: &Path) {
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
            .find(|entry| entry.name == format!("assets/{ANDROID_ASSET}"))
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
        template(&project_dir.join("template.apk"));
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

        // An application is no template.
        let error = package(&project, &output, &root.join("again.apk"), &key).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);

        fs::remove_dir_all(root).unwrap();
    }
}

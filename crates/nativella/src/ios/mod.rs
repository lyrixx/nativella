//! `nativella ios`: the iOS app of a PHP project, made from a template app,
//! without Xcode, on any computer.
//!
//! A template is the application, PHP linked in, with the Swift side of Tauri
//! and of its plugins, built once per release without any project, for the
//! package `dev.nativella.runtime` of tauri.conf.json (`castor
//! mobile:ios-template`): one for the simulator, one for devices, each a ZIP
//! archive of `Payload/Nativella.app`, as an IPA is. An app is the template:
//!
//! - with the bundle of the project as the file [`PACKAGED_BUNDLE`], next to
//!   the executable, which reads it at startup (see embedded.rs);
//! - with the identity of the project in its Info.plist: its bundle ID, its
//!   name, its version, and the reasons iOS gives the user for the phone APIs
//!   it asks for, and for them only;
//! - signed (see signature.rs): ad hoc for the simulator, which runs nothing
//!   unsigned on Apple silicon; with the certificate of the developer and the
//!   provisioning profile that lets the app run on devices, embedded as
//!   `embedded.mobileprovision`, for devices.
//!
//! Its executable keeps the name of the template's: iOS finds it through
//! `CFBundleExecutable`. Nothing in it depends on the identifier: Tauri takes
//! it from the bundle, at startup (see lib.rs), and iOS gives each bundle ID a
//! container of its own, where the data and cache directories are.

mod certificate;
mod entitlements;
mod macho;
mod profile;
mod resources;
mod signature;

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Component, Path, PathBuf};

use plist::{Dictionary, Value};

use self::certificate::Certificate;
use self::profile::Profile;
use self::signature::Signer;
use crate::embedded::{self, PACKAGED_BUNDLE};
use crate::project::{Permission, Project};
use crate::zip::{self, Entry};

/// What the template of devices names the provisioning profile it embeds.
const EMBEDDED_PROFILE: &str = "embedded.mobileprovision";

/// What signs an app for devices: the certificate of the developer, and the
/// provisioning profile that lets the app run on devices, issued for it.
pub struct Identity {
    certificate: Certificate,
    profile: Profile,
}

impl Identity {
    /// The certificate and its private key in the PKCS #12 file
    /// `certificate`, encrypted with `password`, and the provisioning profile
    /// `profile`.
    pub fn load(certificate: &Path, password: &str, profile: &Path) -> io::Result<Self> {
        let certificate = Certificate::load(certificate, password)?;
        let profile = Profile::load(profile)?;
        profile.check(&certificate)?;

        Ok(Self {
            certificate,
            profile,
        })
    }
}

/// Where an app runs, as the `CFBundleSupportedPlatforms` of its Info.plist
/// says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Platform {
    Simulator,
    Device,
}

/// Writes the iOS app of `project` to `output`, made from `template`: a
/// directory `<name>.app` for the simulator, which `xcrun simctl install`
/// takes, signed ad hoc; an IPA for devices, signed by `identity`. Returns
/// the local overrides it left out, relative to the project.
pub fn package(
    project: &Project,
    template: &Path,
    output: &Path,
    identity: Option<&Identity>,
) -> io::Result<Vec<String>> {
    let version = bundle_version(&project.manifest.version)?;

    let mut template_file = File::open(template).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("Cannot read the template {}: {error}.", template.display()),
        )
    })?;
    let entries = zip::entries(&mut template_file)?;
    let app = app_of(&entries)?;
    if entries
        .iter()
        .any(|entry| entry.name == format!("{app}{PACKAGED_BUNDLE}"))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} is an app already, not a template: take a template of the release of this nativella command.",
                template.display()
            ),
        ));
    }
    let info_entry = entries
        .iter()
        .find(|entry| entry.name == format!("{app}Info.plist"))
        .ok_or_else(|| invalid("The template has no Info.plist."))?;
    let info_bytes = zip::read(&mut template_file, info_entry)?;
    let mut info: Dictionary = plist::from_bytes(&info_bytes).map_err(|error| {
        invalid(&format!(
            "The Info.plist of the template is invalid: {error}."
        ))
    })?;

    let platform = platform(&info)?;
    let extension = match (platform, identity) {
        (Platform::Simulator, None) => "app",
        (Platform::Device, Some(_)) => "ipa",
        (Platform::Simulator, Some(_)) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "This template is the simulator's, whose apps are signed ad hoc: --certificate and --profile sign the apps of devices, made of the template of devices.",
            ));
        }
        (Platform::Device, None) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "This template is the one of devices, which run signed apps only: sign it with --certificate and --profile.",
            ));
        }
    };
    if output.extension().is_none_or(|actual| actual != extension) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} must end with .{extension}: an app for the {}.",
                output.display(),
                match platform {
                    Platform::Simulator => "simulator is a .app directory",
                    Platform::Device => "devices is an .ipa file",
                }
            ),
        ));
    }
    // Built for the version of the crate: see tauri.conf.json.
    let nativella_version = env!("CARGO_PKG_VERSION");
    if let Some(template_version) = info
        .get("CFBundleShortVersionString")
        .and_then(Value::as_string)
        && template_version != nativella_version
    {
        eprintln!(
            "Warning: the template is the one of Nativella {template_version}, this command the one of {nativella_version}: take both from the same release."
        );
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
    let app_dir = match platform {
        Platform::Simulator => staging.0.join(name),
        Platform::Device => staging
            .0
            .join("Payload")
            .join(Path::new(name).with_extension("app")),
    };

    extract(&mut template_file, &entries, &app, &app_dir)?;
    drop(template_file);
    // Whatever the archiver of the template recorded.
    #[cfg(unix)]
    if let Some(executable) = info.get("CFBundleExecutable").and_then(Value::as_string) {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(app_dir.join(executable), fs::Permissions::from_mode(0o755))?;
    }

    // The bundle leaves out what the packaging writes and reads, should it be
    // inside the project: the app being made, a previous one, the template,
    // the certificate and the profile.
    let mut excluded = vec![staging.0.as_path(), template];
    if output.exists() {
        excluded.push(output);
    }
    if let Some(identity) = identity {
        excluded.push(identity.certificate.path());
        excluded.push(identity.profile.path());
    }
    let mut bundle = BufWriter::new(File::create(app_dir.join(PACKAGED_BUNDLE))?);
    let local_overrides = embedded::write_bundle(project, &mut bundle, &excluded)?;
    bundle.flush()?;
    drop(bundle);

    rewrite_info(&mut info, project, &version);
    // In the format of the template: Xcode writes binary property lists.
    let info_path = app_dir.join("Info.plist");
    let written = if info_bytes.starts_with(b"bplist") {
        plist::to_file_binary(&info_path, &info)
    } else {
        plist::to_file_xml(&info_path, &info)
    };
    written.map_err(|error| io::Error::other(format!("Cannot write the Info.plist: {error}.")))?;

    match identity {
        None => signature::sign(&app_dir, &Signer::AdHoc)?,
        Some(identity) => {
            let identifier = &project.manifest.identifier;
            fs::copy(identity.profile.path(), app_dir.join(EMBEDDED_PROFILE))?;
            let entitlements = identity.profile.entitlements(identifier)?;
            signature::sign(
                &app_dir,
                &Signer::Developer {
                    certificate: &identity.certificate,
                    entitlements: &entitlements,
                },
            )?;
        }
    }

    match platform {
        Platform::Simulator => replace_app(&app_dir, output)?,
        Platform::Device => write_ipa(&staging.0, output)?,
    }

    Ok(local_overrides)
}

/// A directory that the packaging removes when it ends, whether it failed
/// or not.
struct Staging(PathBuf);

impl Staging {
    fn create(path: PathBuf) -> io::Result<Self> {
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

/// `Payload/<name>.app/`: the app of the template, the only one of its
/// archive.
fn app_of(entries: &[Entry]) -> io::Result<String> {
    let mut apps = entries.iter().filter_map(|entry| {
        let rest = entry.name.strip_prefix("Payload/")?;
        let (app, _) = rest.split_once('/')?;

        app.ends_with(".app").then(|| format!("Payload/{app}/"))
    });
    let app = apps
        .next()
        .ok_or_else(|| invalid("The template holds no Payload/<name>.app: it is no iOS app."))?;
    if apps.any(|other| other != app) {
        return Err(invalid("The template holds several apps in its Payload."));
    }

    Ok(app)
}

/// The platform of an app, from the Info.plist that Xcode writes.
fn platform(info: &Dictionary) -> io::Result<Platform> {
    let platforms: Vec<&str> = info
        .get("CFBundleSupportedPlatforms")
        .and_then(Value::as_array)
        .map(|platforms| platforms.iter().filter_map(Value::as_string).collect())
        .unwrap_or_default();

    match platforms[..] {
        ["iPhoneSimulator"] => Ok(Platform::Simulator),
        ["iPhoneOS"] => Ok(Platform::Device),
        _ => Err(invalid(&format!(
            "The template is for {platforms:?}, not for the iOS simulator (iPhoneSimulator) nor for iOS devices (iPhoneOS)."
        ))),
    }
}

/// Extracts the app `app` of the template, `Payload/<name>.app/`, to
/// `app_dir`. Leaves out what an IPA may hold beside it: its directory
/// `Payload/`, `SwiftSupport/`, `Symbols/`.
fn extract(file: &mut File, entries: &[Entry], app: &str, app_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(app_dir)?;

    for entry in entries {
        let Some(relative) = entry.name.strip_prefix(app) else {
            continue;
        };
        if relative.is_empty() {
            continue;
        }
        // An archive could hold ../, or an absolute path: it would write
        // outside of the app.
        let relative = Path::new(relative.trim_end_matches('/'));
        if entry.name.contains('\\')
            || !relative
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err(invalid(&format!(
                "The template holds {}, outside of its app.",
                entry.name
            )));
        }
        if entry.is_special() {
            return Err(invalid(&format!(
                "The template holds {}, a symbolic link or a special file, which no app has.",
                entry.name
            )));
        }

        let path = app_dir.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = BufWriter::new(create_file(
            &path,
            entry.unix_permissions().unwrap_or(0o644),
        )?);
        zip::extract(file, entry, &mut out)?;
        out.flush()?;
    }

    Ok(())
}

/// Creates the file `path`, with the Unix `permissions` on Unix.
fn create_file(path: &Path, permissions: u32) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(permissions);
    }
    #[cfg(not(unix))]
    let _ = permissions;

    options.open(path)
}

/// The identity of the project in the Info.plist of the app: its bundle ID,
/// its name, its version, and the reasons of the phone APIs it asks for.
///
/// The bundle ID is the identifier as it is: Apple allows letters, digits,
/// dashes and dots in one (`CFBundleIdentifier`), what project.rs accepts.
fn rewrite_info(info: &mut Dictionary, project: &Project, version: &str) {
    let manifest = &project.manifest;
    info.insert(
        "CFBundleIdentifier".to_owned(),
        manifest.identifier.clone().into(),
    );
    info.insert("CFBundleName".to_owned(), manifest.name.clone().into());
    info.insert(
        "CFBundleDisplayName".to_owned(),
        manifest.name.clone().into(),
    );
    info.insert("CFBundleShortVersionString".to_owned(), version.into());
    info.insert("CFBundleVersion".to_owned(), version.into());

    for &permission in Permission::ALL {
        let Some((key, reason)) = usage_description(permission, &manifest.name) else {
            continue;
        };
        if manifest.permissions.contains(&permission) {
            info.insert(key.to_owned(), reason.into());
        } else {
            info.remove(key);
        }
    }
}

/// The key of the Info.plist that holds the reason iOS gives the user of the
/// app `name` when it uses a phone API, without which iOS refuses the API,
/// and the reason: none for an API without permission. Keep in sync with the
/// iOS side of the plugins.
fn usage_description(permission: Permission, name: &str) -> Option<(&'static str, String)> {
    match permission {
        Permission::Biometrics => Some((
            "NSFaceIDUsageDescription",
            format!("{name} uses Face ID to check that it is you."),
        )),
        // The plugin asks for the location while the app is in use.
        Permission::Geolocation => Some((
            "NSLocationWhenInUseUsageDescription",
            format!("{name} uses the location of the phone."),
        )),
        Permission::Haptics => None,
    }
}

/// `CFBundleShortVersionString` and `CFBundleVersion`: the version of the
/// project as three integers, which Apple requires, without its pre-release
/// or build suffix.
fn bundle_version(version: &str) -> io::Result<String> {
    let invalid_version = || {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "The version \"{version}\" cannot number an iOS app, which takes three integers: major.minor.patch."
            ),
        )
    };

    // Without its pre-release or build suffix: 1.2.3-beta.1+abc.
    let core = version.split(['-', '+']).next().unwrap_or(version);
    let parts = core
        .split('.')
        .map(str::parse)
        .collect::<Result<Vec<u64>, _>>()
        .map_err(|_| invalid_version())?;

    match parts[..] {
        [major] => Ok(format!("{major}.0.0")),
        [major, minor] => Ok(format!("{major}.{minor}.0")),
        [major, minor, patch] => Ok(format!("{major}.{minor}.{patch}")),
        _ => Err(invalid_version()),
    }
}

/// Moves the app made in `app_dir` to `output`, in place of a previous one.
fn replace_app(app_dir: &Path, output: &Path) -> io::Result<()> {
    if output.exists() {
        // Never a directory that is not an app: the output could be a typo.
        if !output.join("Info.plist").is_file() {
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

/// Writes the IPA of the app in `staging/Payload/`: a ZIP archive of it,
/// with the Unix permissions of its files.
fn write_ipa(staging: &Path, output: &Path) -> io::Result<()> {
    let mut ipa = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(output)?;
    let mut writer = zip::Writer::new(&mut ipa);
    add_to_ipa(&mut writer, staging, Path::new("Payload"))?;
    let unfinished = writer.finish()?;

    zip::close(&mut ipa, &unfinished)
}

/// Adds the directory `relative` of `root` to the IPA, then what it holds,
/// in order.
fn add_to_ipa(writer: &mut zip::Writer, root: &Path, relative: &Path) -> io::Result<()> {
    let name = relative.to_string_lossy().replace('\\', "/");
    writer.add_directory(&name)?;

    let mut entries = fs::read_dir(root.join(relative))?.collect::<Result<Vec<_>, _>>()?;
    // Reproducible order.
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = relative.join(entry.file_name());
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            add_to_ipa(writer, root, &path)?;
            continue;
        }

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode() & 0o777
        };
        // Executable or not, as Tauri does: the file system cannot tell.
        #[cfg(not(unix))]
        let permissions = 0o755;
        let mut file = File::open(entry.path())?;
        writer.add_deflated_with(
            &path.to_string_lossy().replace('\\', "/"),
            permissions,
            |out| io::copy(&mut file, out).map(|_| ()),
        )?;
    }

    Ok(())
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use flate2::read::GzDecoder;
    use rsa::sha2::{Digest, Sha256};

    use super::*;
    use crate::ios::certificate::tests::{PASSWORD, testdata};
    use crate::ios::signature::tests::{blobs, parse};

    /// A template, as `castor mobile:ios-template` makes it: the app Xcode
    /// built, in `Payload/`, its Info.plist binary, its executable signed by
    /// the linker for the simulator, unsigned for devices.
    fn template(path: &Path, platform: Platform, entries: &[(&str, &[u8])]) {
        let mut info = Dictionary::new();
        for (key, value) in [
            ("CFBundleExecutable", "Nativella"),
            ("CFBundleIdentifier", "dev.nativella.runtime"),
            ("CFBundleName", "Nativella"),
            ("CFBundleShortVersionString", env!("CARGO_PKG_VERSION")),
            ("CFBundleVersion", env!("CARGO_PKG_VERSION")),
            ("NSFaceIDUsageDescription", "Nativella uses Face ID."),
        ] {
            info.insert(key.to_owned(), value.into());
        }
        let platform_name = match platform {
            Platform::Simulator => "iPhoneSimulator",
            Platform::Device => "iPhoneOS",
        };
        info.insert(
            "CFBundleSupportedPlatforms".to_owned(),
            vec![Value::from(platform_name)].into(),
        );
        let mut info_bytes = Vec::new();
        plist::to_writer_binary(&mut info_bytes, &info).unwrap();
        let executable = macho::tests::executable(platform == Platform::Simulator);

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .unwrap();
        let mut writer = zip::Writer::new(&mut file);
        writer.add_directory("Payload").unwrap();
        writer.add_directory("Payload/Nativella.app").unwrap();
        let app_entries: [(&str, &[u8]); 4] = [
            ("Info.plist", &info_bytes),
            ("Nativella", &executable),
            ("PkgInfo", b"APPL????"),
            ("LaunchScreen.storyboardc/Info.plist", b"nib"),
        ];
        for (name, data) in app_entries.iter().chain(entries) {
            let permissions = if *name == "Nativella" { 0o755 } else { 0o644 };
            writer
                .add_deflated_with(
                    &format!("Payload/Nativella.app/{name}"),
                    permissions,
                    |out| out.write_all(data),
                )
                .unwrap();
        }
        let unfinished = writer.finish().unwrap();
        zip::close(&mut file, &unfinished).unwrap();
    }

    /// A project asking for the location, with a local override, and a
    /// template of `platform` in it: none of them ships.
    fn project(name: &str, platform: Platform) -> (Project, PathBuf) {
        let root =
            Path::new(env!("OUT_DIR")).join(format!("ios-test-{name}-{}", std::process::id()));
        let project_dir = root.join("note-taker");
        fs::create_dir_all(project_dir.join("public")).unwrap();
        fs::write(project_dir.join("public/index.php"), "<?php echo 'hi';").unwrap();
        fs::write(
            project_dir.join("composer.json"),
            r#"{"name": "acme/note-taker", "extra": {"nativella": {"version": "1.2.3-beta.1", "permissions": ["geolocation"]}}}"#,
        )
        .unwrap();
        fs::write(project_dir.join(".env.local"), "SECRET=1").unwrap();
        template(&project_dir.join("template.zip"), platform, &[]);

        (Project::load(&project_dir).unwrap(), project_dir)
    }

    /// The paths of the files of the bundle `path`.
    fn bundled(path: &Path) -> Vec<PathBuf> {
        let bundle = fs::read(path).unwrap();
        // The archive comes first, its description after it.
        let mut archive = tar::Archive::new(GzDecoder::new(&bundle[..]));
        let mut paths: Vec<_> = archive
            .entries()
            .unwrap()
            .map(|entry| entry.unwrap().path().unwrap().into_owned())
            .collect();
        paths.sort();

        paths
    }

    fn sha256(path: &Path) -> [u8; 32] {
        Sha256::digest(fs::read(path).unwrap()).into()
    }

    #[test]
    fn an_app_for_the_simulator() {
        let (project, project_dir) = project("simulator", Platform::Simulator);
        let output = project_dir.join("dist/note-taker.app");

        let local_overrides =
            package(&project, &project_dir.join("template.zip"), &output, None).unwrap();

        assert_eq!(local_overrides, [".env.local"]);
        assert_eq!(
            bundled(&output.join(PACKAGED_BUNDLE)),
            [
                PathBuf::from("composer.json"),
                PathBuf::from("public/index.php")
            ]
        );

        // The identity of the project, in the format of the template.
        let info_path = output.join("Info.plist");
        assert!(fs::read(&info_path).unwrap().starts_with(b"bplist00"));
        let info: Dictionary = plist::from_file(&info_path).unwrap();
        let string = |key: &str| info.get(key).and_then(Value::as_string);
        assert_eq!(
            string("CFBundleIdentifier"),
            Some("dev.nativella.note-taker")
        );
        assert_eq!(string("CFBundleName"), Some("Note Taker"));
        assert_eq!(string("CFBundleDisplayName"), Some("Note Taker"));
        assert_eq!(string("CFBundleShortVersionString"), Some("1.2.3"));
        assert_eq!(string("CFBundleVersion"), Some("1.2.3"));
        assert_eq!(
            string("NSLocationWhenInUseUsageDescription"),
            Some("Note Taker uses the location of the phone.")
        );
        assert_eq!(string("NSFaceIDUsageDescription"), None);

        // Signed ad hoc: the code directory binds the Info.plist, the seal of
        // the files, and the pages of the executable.
        let executable = output.join("Nativella");
        let blobs = blobs(&executable);
        assert_eq!(blobs.keys().copied().collect::<Vec<_>>(), [0, 2, 0x10000]);
        let directory = parse(&blobs[&0]);
        assert_eq!(directory.flags, 0x2, "Ad hoc.");
        assert_eq!(directory.identifier, "dev.nativella.note-taker");
        assert_eq!(directory.team, None);
        assert_eq!(directory.special.len(), 3);
        assert_eq!(directory.special[0], sha256(&info_path));
        assert_eq!(
            directory.special[1],
            <[u8; 32]>::from(Sha256::digest(&blobs[&2]))
        );
        assert_eq!(
            directory.special[2],
            sha256(&output.join("_CodeSignature/CodeResources"))
        );
        let bytes = fs::read(&executable).unwrap();
        let signed = &bytes[..directory.code_limit as usize];
        assert_eq!(directory.code.len(), signed.len().div_ceil(4096));
        for (page, hash) in signed.chunks(4096).zip(&directory.code) {
            assert_eq!(<[u8; 32]>::from(Sha256::digest(page)), *hash);
        }
        assert_eq!(blobs[&0x10000].len(), 8, "No CMS signature.");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&executable).unwrap().permissions().mode() & 0o777,
                0o755
            );
        }

        // Again: the app replaces the previous one, which does not ship.
        package(&project, &project_dir.join("template.zip"), &output, None).unwrap();
        assert_eq!(bundled(&output.join(PACKAGED_BUNDLE)).len(), 2);

        fs::remove_dir_all(project_dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn an_app_for_devices() {
        let (project, project_dir) = project("device", Platform::Device);
        let output = project_dir.join("dist/note-taker.ipa");
        let identity = Identity::load(
            &testdata("developer.p12"),
            PASSWORD,
            &testdata("development.mobileprovision"),
        )
        .unwrap();

        package(
            &project,
            &project_dir.join("template.zip"),
            &output,
            Some(&identity),
        )
        .unwrap();

        let mut ipa = File::open(&output).unwrap();
        let entries = zip::entries(&mut ipa).unwrap();
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Payload/",
                "Payload/note-taker.app/",
                "Payload/note-taker.app/Info.plist",
                "Payload/note-taker.app/LaunchScreen.storyboardc/",
                "Payload/note-taker.app/LaunchScreen.storyboardc/Info.plist",
                "Payload/note-taker.app/Nativella",
                "Payload/note-taker.app/PkgInfo",
                "Payload/note-taker.app/_CodeSignature/",
                "Payload/note-taker.app/_CodeSignature/CodeResources",
                "Payload/note-taker.app/embedded.mobileprovision",
                "Payload/note-taker.app/nativella.bundle",
            ]
        );
        let executable_entry = &entries[5];
        assert_eq!(executable_entry.unix_permissions(), Some(0o755));
        let executable = project_dir.join("extracted");
        zip::extract(
            &mut ipa,
            executable_entry,
            &mut File::create(&executable).unwrap(),
        )
        .unwrap();

        // Signed by the developer, with the entitlements of the profile, for
        // this app.
        let blobs = blobs(&executable);
        assert_eq!(
            blobs.keys().copied().collect::<Vec<_>>(),
            [0, 2, 5, 7, 0x10000]
        );
        let directory = parse(&blobs[&0]);
        assert_eq!(directory.flags, 0);
        assert_eq!(directory.identifier, "dev.nativella.note-taker");
        assert_eq!(directory.team.as_deref(), Some("TEAM123456"));
        assert_eq!(directory.special.len(), 7);
        assert_eq!(directory.exec_segment_flags, 0x11, "Debuggers may attach.");
        let entitlements: Dictionary = plist::from_bytes(&blobs[&5][8..]).unwrap();
        assert_eq!(
            entitlements["application-identifier"].as_string(),
            Some("TEAM123456.dev.nativella.note-taker")
        );
        assert!(blobs[&0x10000].len() > 8, "A CMS signature.");

        fs::remove_dir_all(project_dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_template_for_the_wrong_target() {
        let (project, project_dir) = project("targets", Platform::Simulator);
        let template_path = project_dir.join("template.zip");
        let identity = Identity::load(
            &testdata("developer.p12"),
            PASSWORD,
            &testdata("development.mobileprovision"),
        )
        .unwrap();

        // Simulator apps are signed ad hoc.
        let error = package(
            &project,
            &template_path,
            &project_dir.join("a.ipa"),
            Some(&identity),
        )
        .unwrap_err();
        assert!(error.to_string().contains("simulator"), "{error}");
        // Apps for the simulator are directories.
        let error =
            package(&project, &template_path, &project_dir.join("a.ipa"), None).unwrap_err();
        assert!(error.to_string().contains(".app"), "{error}");
        // Devices run signed apps.
        template(&template_path, Platform::Device, &[]);
        let error =
            package(&project, &template_path, &project_dir.join("a.app"), None).unwrap_err();
        assert!(error.to_string().contains("--certificate"), "{error}");
        // An app is no template.
        template(
            &template_path,
            Platform::Simulator,
            &[(PACKAGED_BUNDLE, b"bundle")],
        );
        let error =
            package(&project, &template_path, &project_dir.join("a.app"), None).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);

        fs::remove_dir_all(project_dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_template_cannot_write_outside_of_its_app() {
        let (project, project_dir) = project("outside", Platform::Simulator);
        let template_path = project_dir.join("template.zip");
        template(
            &template_path,
            Platform::Simulator,
            &[("../../escaped", b"x")],
        );

        let error =
            package(&project, &template_path, &project_dir.join("a.app"), None).unwrap_err();

        assert!(error.to_string().contains("outside of its app"), "{error}");
        assert!(!project_dir.join("escaped").exists());
        fs::remove_dir_all(project_dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn bundle_versions() {
        assert_eq!(bundle_version("1.2.3").unwrap(), "1.2.3");
        assert_eq!(bundle_version("2.3.4-beta.1+abc").unwrap(), "2.3.4");
        assert_eq!(bundle_version("3").unwrap(), "3.0.0");
        assert_eq!(bundle_version("3.1").unwrap(), "3.1.0");

        for version in ["", "1.x", "1.2.3.4", "v1.0.0"] {
            assert!(bundle_version(version).is_err(), "{version}");
        }
    }
}

//! `updater.*`: a standalone executable updates itself from the releases of
//! the application. The `endpoint` of `updates` in composer.json serves a
//! JSON document in the format of Tauri's updater: the latest version, and
//! for each platform the URL of its executable and the minisign signature of
//! it, which the `publicKey` checks.
//!
//! The executable downloads in the background, next to the running one, and
//! replaces it once its signature holds and it says it is the announced
//! version of this application, for this machine: the next start runs the
//! new version. PHP learns it from `updater.installed`, or `updater.failed`.

use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{Call, failed, lock};
use crate::embedded::{self, Bundle};
use crate::events;
use crate::project::Updates;

/// The document is small: a server that takes longer is down.
const DOCUMENT_TIMEOUT: Duration = Duration::from_secs(30);
/// An executable is tens of megabytes: it may take a while, but not stall.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
/// In bytes. The executable is checked in memory: an endpoint gone wrong
/// must not fill it, nor the disk.
const DOWNLOAD_LIMIT: u64 = 1024 * 1024 * 1024;

/// The document of the endpoint, as Tauri's updater reads it.
#[derive(Debug, Deserialize)]
struct Release {
    /// Without the `v` of Tauri's.
    #[serde(deserialize_with = "version")]
    version: Version,
    notes: Option<String>,
    pub_date: Option<String>,
    platforms: HashMap<String, Artifact>,
}

/// What this run of the application did of its updates.
#[derive(Default)]
pub(crate) struct Progress {
    /// The version being downloaded: two installs would write the same
    /// executable.
    installing: Mutex<Option<Version>>,
    /// The version that took the place of the executable: the next start
    /// runs it.
    installed: Mutex<Option<Version>>,
}

impl Progress {
    /// The version the endpoint is compared to: the one being installed, or
    /// installed, rather than the one running.
    fn baseline(&self, running: &Version) -> Version {
        let installing = lock(&self.installing).clone();

        installing
            .or_else(|| lock(&self.installed).clone())
            .unwrap_or_else(|| running.clone())
    }

    /// Fails while an update is being installed.
    fn ensure_idle(&self) -> Result<(), String> {
        match &*lock(&self.installing) {
            Some(version) => Err(format!("Version {version} is being installed.")),
            None => Ok(()),
        }
    }

    /// The right to install `version`, until the claim is dropped.
    fn claim(self: &Arc<Self>, version: &Version) -> Result<Claim, String> {
        let mut installing = lock(&self.installing);
        if let Some(other) = &*installing {
            return Err(format!("Version {other} is being installed."));
        }
        *installing = Some(version.clone());

        Ok(Claim(Arc::clone(self)))
    }
}

/// Ends the install when dropped, whether it succeeded or not.
struct Claim(Arc<Progress>);

impl Claim {
    /// The version claimed took the place of the executable.
    fn installed(self) {
        let version = lock(&self.0.installing).clone();
        *lock(&self.0.installed) = version;
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        *lock(&self.0.installing) = None;
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Artifact {
    url: String,
    /// The `.minisig` file, base64-encoded, as Tauri's signer writes it.
    signature: String,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    Some(match method {
        "check" => newer(call).map(|release| match release {
            Some(release) => json!({ "update": {
                "version": release.version.to_string(),
                "notes": release.notes,
                "date": release.pub_date,
            } }),
            None => json!({ "update": null }),
        }),
        "install" => install(call),
        _ => return None,
    })
}

/// Downloads the newer release in the background, if there is one: its
/// version, or null.
fn install(call: &Call<'_>) -> Result<Value, String> {
    let progress = Arc::clone(&call.state().updater);
    // After an update, Linux no longer finds the running executable where it
    // started (`… (deleted)`): it was a standalone one.
    if lock(&progress.installed).is_none() && !embedded::is_standalone() {
        return Err(
            "Only a standalone executable updates itself: nativella run runs the project as it is."
                .to_owned(),
        );
    }
    progress.ensure_idle()?;
    let Some(mut release) = newer(call)? else {
        return Ok(json!({ "version": null }));
    };
    let claim = progress.claim(&release.version)?;
    let updates = updates(call)?;
    let artifact = release.platforms.remove(&platform()).ok_or_else(|| {
        format!(
            "Version {} has no executable for {}.",
            release.version,
            platform()
        )
    })?;

    // Where the application started from, as the system has it once the
    // file is replaced.
    let executable = tauri::utils::platform::current_exe().map_err(failed)?;
    // Next to the executable, to take its place in one step: now, so that a
    // directory the user cannot write to is an error of the call.
    let (download, file) = download_file(&executable)?;

    let app = call.app.clone();
    let expected = Expected {
        identifier: call.app.config().identifier.clone(),
        version: release.version.clone(),
        above: call.app.package_info().version.clone(),
    };
    thread::Builder::new()
        .name("updater".to_owned())
        .spawn(move || {
            let version = expected.version.to_string();
            match replace(file, &download, &executable, &artifact, &updates, &expected) {
                Ok(()) => {
                    claim.installed();
                    events::send(&app, "updater.installed", json!({ "version": version }));
                }
                Err(error) => {
                    eprintln!("[updater] Version {version} is not installed: {error}");
                    events::send(
                        &app,
                        "updater.failed",
                        json!({ "version": version, "message": error }),
                    );
                }
            }
        })
        .map_err(failed)?;

    Ok(json!({ "version": release.version.to_string() }))
}

/// What the downloads of the updates of `executable` are named with, next
/// to it.
fn download_prefix(executable: &Path) -> String {
    let name = executable
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    format!(".{name}.update-")
}

/// Removes the downloads an earlier run left next to the executable: an
/// application that quits while it downloads does not remove its own. At
/// startup, when no other instance runs.
pub(crate) fn remove_leftovers() {
    match tauri::utils::platform::current_exe() {
        Ok(executable) => remove_leftovers_of(&executable),
        Err(error) => eprintln!("[updater] Unfinished downloads are not removed: {error}."),
    }
}

fn remove_leftovers_of(executable: &Path) {
    let prefix = download_prefix(executable);
    let Some(entries) = executable.parent().and_then(|dir| dir.read_dir().ok()) else {
        return;
    };
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(&prefix) {
            continue;
        }
        if let Err(error) = fs::remove_file(entry.path()) {
            eprintln!(
                "[updater] {} is left behind: {error}.",
                entry.path().display()
            );
        }
    }
}

/// A file of its own next to `executable`, for the download.
fn download_file(executable: &Path) -> Result<(PathBuf, File), String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let download = executable.with_file_name(format!(
        "{}{}-{nanos}",
        download_prefix(executable),
        std::process::id()
    ));
    let file = File::options()
        .write(true)
        .create_new(true)
        .open(&download)
        .map_err(|error| {
            format!(
                "Cannot write next to the executable, in {}: {error}. An executable installed by a package manager updates with it.",
                download.parent().unwrap_or(&download).display()
            )
        })?;

    Ok((download, file))
}

/// The release of the endpoint, when it is newer than this version, or
/// than the one installed since it started.
fn newer(call: &Call<'_>) -> Result<Option<Release>, String> {
    let updates = updates(call)?;
    let release = fetch(&updates.endpoint)?;
    let baseline = call
        .state()
        .updater
        .baseline(&call.app.package_info().version);

    Ok((release.version > baseline).then_some(release))
}

fn updates(call: &Call<'_>) -> Result<Updates, String> {
    call.state().updates().cloned().ok_or_else(|| {
        "The application has no \"updates\" in extra.nativella of its composer.json.".to_owned()
    })
}

fn fetch(endpoint: &str) -> Result<Release, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(DOCUMENT_TIMEOUT))
        .https_only(https_only(endpoint))
        .build()
        .into();
    let mut response = agent
        .get(endpoint)
        .call()
        .map_err(|error| format!("The updates endpoint {endpoint} failed: {error}."))?;

    serde_json::from_reader(response.body_mut().as_reader())
        .map_err(|error| format!("The document of {endpoint} is invalid: {error}."))
}

/// An endpoint over HTTPS never leads to plain HTTP, through a redirection
/// or the URL of an executable. A local one, for testing, may.
fn https_only(endpoint: &str) -> bool {
    endpoint.starts_with("https://")
}

/// The version of the endpoint, which Tauri's releases may name `v1.2.0`.
fn parse_version(version: &str) -> Result<Version, String> {
    Version::parse(version.trim_start_matches('v')).map_err(|error| {
        format!("The version \"{version}\" of the endpoint is not a semantic version: {error}.")
    })
}

fn version<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Version, D::Error> {
    parse_version(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
}

/// The key of this platform in the document: `linux-x86_64`,
/// `darwin-aarch64`, `windows-x86_64`, as Tauri names them.
fn platform() -> String {
    let os = match env::consts::OS {
        "macos" => "darwin",
        os => os,
    };

    format!("{os}-{}", env::consts::ARCH)
}

/// What the downloaded executable must say of itself, in the bundle it
/// carries: signed, it is still not necessarily the one announced.
struct Expected {
    /// The identifier of this application.
    identifier: String,
    /// The version the endpoint announced.
    version: Version,
    /// The running version: no going back.
    above: Version,
}

/// Downloads the executable of `artifact` into `file`, at `download`,
/// checks it, and puts it in the place of `executable`.
fn replace(
    file: File,
    download: &Path,
    executable: &Path,
    artifact: &Artifact,
    updates: &Updates,
    expected: &Expected,
) -> Result<(), String> {
    let result = download_to(
        &artifact.url,
        file,
        https_only(&updates.endpoint),
        DOWNLOAD_LIMIT,
    )
    .and_then(|()| {
        let bytes = fs::read(download).map_err(failed)?;
        verify(&updates.public_key, &artifact.signature, &bytes)?;
        check(&bytes, expected)
    })
    .and_then(|()| swap(executable, download).map_err(failed));
    if result.is_err()
        && let Err(error) = fs::remove_file(download)
        && error.kind() != io::ErrorKind::NotFound
    {
        eprintln!("[updater] {} is left behind: {error}.", download.display());
    }

    result
}

/// Checks that `bytes`, a signed executable, run on this machine and are
/// the version `expected` of this application.
fn check(bytes: &[u8], expected: &Expected) -> Result<(), String> {
    if !for_this_machine(bytes) {
        return Err(format!(
            "The executable is not one for {}: check the platforms of the endpoint.",
            platform()
        ));
    }
    let manifest = match Bundle::read(Cursor::new(bytes)) {
        Ok(Some(bundle)) => bundle.manifest().clone(),
        Ok(None) => {
            return Err(
                "The executable carries no application: publish the one of nativella build."
                    .to_owned(),
            );
        }
        Err(error) => return Err(format!("The executable is unreadable: {error}")),
    };
    if manifest.identifier != expected.identifier {
        return Err(format!(
            "The executable is the one of {}, not of {}.",
            manifest.identifier, expected.identifier
        ));
    }
    let version = Version::parse(&manifest.version).map_err(|error| {
        format!(
            "The version \"{}\" of the executable is not a semantic version: {error}.",
            manifest.version
        )
    })?;
    if version != expected.version {
        return Err(format!(
            "The executable is version {version}, not the announced {}.",
            expected.version
        ));
    }
    if version <= expected.above {
        return Err(format!(
            "Version {version} is not newer than the running {}.",
            expected.above
        ));
    }

    Ok(())
}

/// Whether the executable that starts with `head` runs on this machine: a
/// document may list one under the wrong platform.
#[cfg(target_os = "linux")]
fn for_this_machine(head: &[u8]) -> bool {
    // ELF, 64 bits, little-endian, then its machine.
    let machine: u16 = match env::consts::ARCH {
        "x86_64" => 62,
        "aarch64" => 183,
        "riscv64" => 243,
        _ => return true,
    };

    head.starts_with(b"\x7fELF\x02\x01") && head.get(18..20) == Some(&machine.to_le_bytes()[..])
}

/// A Mach-O executable of this processor, or a universal one.
#[cfg(target_os = "macos")]
fn for_this_machine(head: &[u8]) -> bool {
    let cpu: u32 = match env::consts::ARCH {
        "x86_64" => 0x0100_0007,
        "aarch64" => 0x0100_000c,
        _ => return true,
    };

    head.starts_with(&[0xca, 0xfe, 0xba, 0xbe])
        || (head.starts_with(&[0xcf, 0xfa, 0xed, 0xfe])
            && head.get(4..8) == Some(&cpu.to_le_bytes()[..]))
}

/// A PE executable of this processor.
#[cfg(windows)]
fn for_this_machine(head: &[u8]) -> bool {
    let machine: u16 = match env::consts::ARCH {
        "x86_64" => 0x8664,
        "aarch64" => 0xaa64,
        _ => return true,
    };
    let Some(offset) = head
        .get(0x3c..0x40)
        .and_then(|offset| offset.try_into().ok())
        .and_then(|offset| usize::try_from(u32::from_le_bytes(offset)).ok())
    else {
        return false;
    };

    head.starts_with(b"MZ")
        && head.get(offset..offset + 4) == Some(&b"PE\0\0"[..])
        && head.get(offset + 4..offset + 6) == Some(&machine.to_le_bytes()[..])
}

fn download_to(url: &str, mut file: File, https_only: bool, limit: u64) -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .https_only(https_only)
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|error| format!("The download of {url} failed: {error}."))?;
    let mut body = response.body_mut().with_config().limit(limit).reader();
    io::copy(&mut body, &mut file)
        // On the disk before it takes the place of the executable: a power
        // cut after the rename would leave an empty one.
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("The download of {url} failed: {error}."))?;

    Ok(())
}

/// Checks that `bytes` were signed with the secret key of `public_key`.
/// Both come as Tauri's signer gives them, base64-encoded files, or as
/// minisign writes them.
fn verify(public_key: &str, signature: &str, bytes: &[u8]) -> Result<(), String> {
    let public_key = decoded(public_key);
    let public_key = if public_key.contains('\n') {
        PublicKey::decode(&public_key)
    } else {
        PublicKey::from_base64(public_key.trim())
    }
    .map_err(|error| format!("The publicKey of updates is invalid: {error}."))?;
    let signature = Signature::decode(&decoded(signature))
        .map_err(|error| format!("The signature of the update is invalid: {error}."))?;

    public_key
        .verify(bytes, &signature, false)
        .map_err(|_| "The signature of the update does not match its publicKey.".to_owned())
}

/// `value` decoded from base64 when it is, as it is otherwise.
fn decoded(value: &str) -> String {
    BASE64_STANDARD
        .decode(value.trim())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_else(|| value.to_owned())
}

/// Puts `download` in the place of `executable`, which may be running. The
/// previous executable stays next to it, as `<name>.old`, to go back to by
/// hand.
#[cfg(unix)]
fn swap(executable: &Path, download: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(download, fs::Permissions::from_mode(0o755))?;
    let previous = previous(executable);
    remove_previous(&previous)?;
    // Another name for the same file: nothing to copy.
    if let Err(error) = fs::hard_link(executable, &previous) {
        eprintln!(
            "[updater] The previous version is not kept as {}: {error}.",
            previous.display()
        );
    }
    // In one step: the running process keeps the file it started from.
    fs::rename(download, executable)
}

/// Windows does not replace a running executable, but renames it: the
/// previous one goes back in its place if the new one cannot take it.
#[cfg(windows)]
fn swap(executable: &Path, download: &Path) -> io::Result<()> {
    let previous = previous(executable);
    remove_previous(&previous)?;
    fs::rename(executable, &previous)?;
    if let Err(error) = fs::rename(download, executable) {
        if let Err(back) = fs::rename(&previous, executable) {
            eprintln!(
                "[updater] {} is gone, the previous version is {}: {back}.",
                executable.display(),
                previous.display()
            );
        }
        return Err(error);
    }

    Ok(())
}

fn previous(executable: &Path) -> PathBuf {
    let mut name = executable.file_name().unwrap_or_default().to_os_string();
    name.push(".old");

    executable.with_file_name(name)
}

/// Removes the executable an earlier update replaced.
fn remove_previous(previous: &Path) -> io::Result<()> {
    match fs::remove_file(previous) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A key of minisign, and its signature of "hello", base64-encoded as
    /// Tauri's signer prints them.
    const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDAxNEU4ODFGN0VCRDYyNUEKUldSYVlyMStINGhPQVFyVmdkaGhnY2JvMzFSVHNoaTBhemZ6Q05kYnpFbmVVRTY5cGU4NVF0NGQK";
    const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHJzaWduIHNlY3JldCBrZXkKUlVSYVlyMStINGhPQWFtTk1FWFdvVG5EcXlrdVcraVlLQmNqb3JkMzFwc2l6ZC81WkpUZ3RoWGFXR1VrTFBNSUx2WkN6WGl4cXU3Q0VVZnNCTjV6WGhoTFpWRStuU2lMMkFnPQp0cnVzdGVkIGNvbW1lbnQ6IHNpZ25lZCBieSB0ZXN0Cnp4b0FqK0VvdHJTblkyMXBNd1dRWlZQSzJZYmdWSzhpQU93eFNWUFAyYWJlZUlMOGlRN0w2MEhYVGZIenZyOXVDQ051dEJaNm9QYXlSS0UvaFhxZkNnPT0K";

    #[test]
    fn a_signed_executable_is_accepted() {
        assert_eq!(verify(PUBLIC_KEY, SIGNATURE, b"hello"), Ok(()));
        // As minisign writes them.
        assert_eq!(
            verify(&decoded(PUBLIC_KEY), &decoded(SIGNATURE), b"hello"),
            Ok(())
        );
        // The second line of the key file.
        let key = decoded(PUBLIC_KEY);
        assert_eq!(
            verify(key.lines().nth(1).unwrap(), SIGNATURE, b"hello"),
            Ok(())
        );
    }

    #[test]
    fn anything_else_is_refused() {
        assert_eq!(
            verify(PUBLIC_KEY, SIGNATURE, b"hello!"),
            Err("The signature of the update does not match its publicKey.".to_owned())
        );
        assert!(verify("not a key", SIGNATURE, b"hello").is_err());
        assert!(verify(PUBLIC_KEY, "not a signature", b"hello").is_err());
    }

    #[test]
    fn versions_are_semantic() {
        let current = Version::new(1, 2, 0);

        assert!(parse_version("1.3.0").unwrap() > current);
        assert_eq!(parse_version("v1.2.1").unwrap().to_string(), "1.2.1");
        assert!(parse_version("1.2.0-beta.1").unwrap() < current);
        assert!(parse_version("latest").is_err());
    }

    #[test]
    fn the_document_of_tauri_s_updater() {
        let release: Release = serde_json::from_value(json!({
            "version": "1.3.0",
            "notes": "Faster search.",
            "pub_date": "2026-10-27T10:00:00Z",
            "platforms": {
                "linux-x86_64": { "signature": SIGNATURE, "url": "https://notes.example/notes" },
            },
        }))
        .unwrap();

        assert_eq!(release.version, Version::new(1, 3, 0));
        assert_eq!(
            release.platforms["linux-x86_64"].url,
            "https://notes.example/notes"
        );
        assert!(platform().contains('-'));

        let tauri: Release =
            serde_json::from_value(json!({ "version": "v1.3.0", "platforms": {} })).unwrap();
        assert_eq!(tauri.version, Version::new(1, 3, 0));
        assert!(
            serde_json::from_value::<Release>(json!({ "version": "latest", "platforms": {} }))
                .is_err()
        );
    }

    #[test]
    fn https_never_leads_to_http() {
        assert!(https_only("https://notes.example/latest.json"));
        assert!(!https_only("http://127.0.0.1:8000/latest.json"));
    }

    #[test]
    fn one_install_at_a_time() {
        let progress = Arc::new(Progress::default());
        let version = Version::new(1, 3, 0);

        let claim = progress.claim(&version).unwrap();
        let refused = Err("Version 1.3.0 is being installed.".to_owned());
        assert_eq!(progress.ensure_idle(), refused);
        assert!(progress.claim(&version).is_err());
        drop(claim);
        assert_eq!(progress.ensure_idle(), Ok(()));
        assert!(progress.claim(&version).is_ok());
    }

    #[test]
    fn a_version_installed_or_being_installed_is_not_offered_again() {
        let progress = Arc::new(Progress::default());
        let running = Version::new(1, 2, 0);
        assert_eq!(progress.baseline(&running), running);

        let claim = progress.claim(&Version::new(1, 3, 0)).unwrap();
        assert_eq!(progress.baseline(&running), Version::new(1, 3, 0));
        claim.installed();
        assert_eq!(progress.baseline(&running), Version::new(1, 3, 0));
        assert_eq!(progress.ensure_idle(), Ok(()));

        // A failed install leaves the one before.
        drop(progress.claim(&Version::new(1, 4, 0)).unwrap());
        assert_eq!(progress.baseline(&running), Version::new(1, 3, 0));
    }

    /// The start of an executable of this machine, then the bundle of
    /// `identifier` at `version`, as embedded.rs writes it, without its
    /// project.
    fn executable(identifier: &str, version: &str) -> Vec<u8> {
        let mut bytes = fs::read(env::current_exe().unwrap()).unwrap();
        bytes.truncate(4096);
        let manifest = json!({
            "build": "1",
            "manifest": { "name": "Notes", "identifier": identifier, "version": version },
        })
        .to_string();
        bytes.extend_from_slice(manifest.as_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&(manifest.len() as u64).to_le_bytes());
        bytes.extend_from_slice(b"NTVLLA01");

        bytes
    }

    fn expected() -> Expected {
        Expected {
            identifier: "dev.nativella.notes".to_owned(),
            version: Version::new(1, 3, 0),
            above: Version::new(1, 2, 0),
        }
    }

    #[test]
    fn the_announced_version_of_this_application_is_accepted() {
        assert_eq!(
            check(&executable("dev.nativella.notes", "1.3.0"), &expected()),
            Ok(())
        );
    }

    #[test]
    fn another_application_or_version_is_refused() {
        assert_eq!(
            check(&executable("dev.nativella.tasks", "1.3.0"), &expected()),
            Err(
                "The executable is the one of dev.nativella.tasks, not of dev.nativella.notes."
                    .to_owned()
            )
        );
        // An older build, announced as the newest.
        assert_eq!(
            check(&executable("dev.nativella.notes", "1.1.0"), &expected()),
            Err("The executable is version 1.1.0, not the announced 1.3.0.".to_owned())
        );
        let downgrade = Expected {
            version: Version::new(1, 1, 0),
            ..expected()
        };
        assert_eq!(
            check(&executable("dev.nativella.notes", "1.1.0"), &downgrade),
            Err("Version 1.1.0 is not newer than the running 1.2.0.".to_owned())
        );
        assert!(check(&executable("dev.nativella.notes", "v1.3"), &expected()).is_err());
    }

    #[test]
    fn an_executable_without_its_application_is_refused() {
        let mut bytes = fs::read(env::current_exe().unwrap()).unwrap();
        bytes.truncate(4096);

        assert_eq!(
            check(&bytes, &expected()),
            Err(
                "The executable carries no application: publish the one of nativella build."
                    .to_owned()
            )
        );
    }

    #[test]
    fn an_executable_of_another_machine_is_refused() {
        let mut bytes = executable("dev.nativella.notes", "1.3.0");
        // The machine (ELF, PE) or the processor (Mach-O) of the header.
        if bytes.starts_with(b"MZ") {
            let offset = u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
            bytes[offset + 4] ^= 0xff;
        } else {
            bytes[4] ^= 0xff;
            bytes[18] ^= 0xff;
        }

        assert!(!for_this_machine(&bytes));
        assert!(check(&bytes, &expected()).is_err());
    }

    fn directory(name: &str) -> PathBuf {
        let dir = Path::new(env!("OUT_DIR")).join(format!("updater-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        dir
    }

    #[test]
    fn the_new_executable_takes_the_place_of_the_previous_one() {
        let dir = directory("swap");
        let executable = dir.join("notes");
        fs::write(&executable, "1.2.0").unwrap();
        let (download, _) = download_file(&executable).unwrap();
        fs::write(&download, "1.3.0").unwrap();

        swap(&executable, &download).unwrap();
        assert_eq!(fs::read_to_string(&executable).unwrap(), "1.3.0");
        assert_eq!(fs::read_to_string(dir.join("notes.old")).unwrap(), "1.2.0");
        assert!(!download.exists());

        // The next update replaces the one kept.
        let (download, _) = download_file(&executable).unwrap();
        fs::write(&download, "1.4.0").unwrap();
        swap(&executable, &download).unwrap();
        assert_eq!(fs::read_to_string(&executable).unwrap(), "1.4.0");
        assert_eq!(fs::read_to_string(dir.join("notes.old")).unwrap(), "1.3.0");

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_download_is_a_new_file() {
        let dir = directory("download");
        let executable = dir.join("notes");

        let (first, _) = download_file(&executable).unwrap();
        let (second, _) = download_file(&executable).unwrap();
        assert_ne!(first, second);
        assert!(
            first
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".notes.update-")
        );

        fs::remove_dir_all(dir).unwrap();
    }

    /// Serves "hello!" once, at the address returned.
    fn serve_hello() -> (std::net::SocketAddr, thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request).unwrap();
            // The client may hang up before the end.
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nhello!",
            );
        });

        (address, server)
    }

    #[test]
    fn a_download_over_the_limit_fails() {
        let dir = directory("limit");
        let executable = dir.join("notes");

        let (address, server) = serve_hello();
        let (download, file) = download_file(&executable).unwrap();
        assert_eq!(
            download_to(&format!("http://{address}/notes"), file, false, 7),
            Ok(())
        );
        server.join().unwrap();
        assert_eq!(fs::read_to_string(&download).unwrap(), "hello!");

        let (address, server) = serve_hello();
        let (_, file) = download_file(&executable).unwrap();
        assert!(download_to(&format!("http://{address}/notes"), file, false, 5).is_err());
        server.join().unwrap();

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unfinished_downloads_are_removed() {
        let dir = directory("leftovers");
        let executable = dir.join("notes");
        fs::write(&executable, "1.2.0").unwrap();
        fs::write(dir.join("notes.old"), "1.1.0").unwrap();
        let (download, _) = download_file(&executable).unwrap();
        // The download of another application in the same directory.
        let (other, _) = download_file(&dir.join("tasks")).unwrap();

        remove_leftovers_of(&executable);
        assert!(!download.exists());
        assert!(other.exists());
        assert!(executable.exists());
        assert!(dir.join("notes.old").exists());

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_refused_download_leaves_the_executable_as_it_was() {
        let (address, server) = serve_hello();

        let dir = directory("replace");
        let executable = dir.join("notes");
        fs::write(&executable, "1.2.0").unwrap();
        let (download, file) = download_file(&executable).unwrap();
        let artifact = Artifact {
            url: format!("http://{address}/notes"),
            signature: SIGNATURE.to_owned(),
        };
        let updates = Updates {
            endpoint: format!("http://{address}/latest.json"),
            public_key: PUBLIC_KEY.to_owned(),
        };

        assert_eq!(
            replace(
                file,
                &download,
                &executable,
                &artifact,
                &updates,
                &expected()
            ),
            Err("The signature of the update does not match its publicKey.".to_owned())
        );
        server.join().unwrap();
        assert_eq!(fs::read_to_string(&executable).unwrap(), "1.2.0");
        assert!(!download.exists());
        assert!(!dir.join("notes.old").exists());

        fs::remove_dir_all(dir).unwrap();
    }
}

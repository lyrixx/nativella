//! `updater.*`: a standalone executable updates itself from the releases of
//! the application. The `endpoint` of `updates` in composer.json serves a
//! JSON document in the format of Tauri's updater: the latest version, and
//! for each platform the URL of its executable and the minisign signature of
//! it, which the `publicKey` checks.
//!
//! The executable downloads in the background, next to the running one, and
//! replaces it once its signature holds: the next start runs the new
//! version. PHP learns it from `updater.installed`, or `updater.failed`.

use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::thread;
use std::time::Duration;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{Call, failed};
use crate::project::Updates;
use crate::{embedded, events};

/// The document is small: a server that takes longer is down.
const DOCUMENT_TIMEOUT: Duration = Duration::from_secs(30);
/// An executable is tens of megabytes: it may take a while, but not stall.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// The document of the endpoint, as Tauri's updater reads it.
#[derive(Debug, Deserialize)]
struct Release {
    version: String,
    notes: Option<String>,
    pub_date: Option<String>,
    platforms: HashMap<String, Artifact>,
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
                "version": release.version,
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
    if !embedded::is_standalone() {
        return Err(
            "Only a standalone executable updates itself: nativella run runs the project as it is."
                .to_owned(),
        );
    }
    let Some(mut release) = newer(call)? else {
        return Ok(json!({ "version": null }));
    };
    let updates = updates(call)?;
    let artifact = release.platforms.remove(&platform()).ok_or_else(|| {
        format!(
            "Version {} has no executable for {}.",
            release.version,
            platform()
        )
    })?;

    // Next to the executable, to take its place in one step: now, so that a
    // directory the user cannot write to is an error of the call.
    let executable = env::current_exe().map_err(failed)?;
    let name = executable
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let download = executable.with_file_name(format!(".{name}.update"));
    let file = File::create(&download).map_err(|error| {
        format!(
            "Cannot write next to the executable, in {}: {error}. An executable installed by a package manager updates with it.",
            download.parent().unwrap_or(&download).display()
        )
    })?;

    let app = call.app.clone();
    let version = release.version.clone();
    thread::Builder::new()
        .name("updater".to_owned())
        .spawn(
            move || match replace(file, &download, &executable, &artifact, &updates) {
                Ok(()) => events::send(&app, "updater.installed", json!({ "version": version })),
                Err(error) => {
                    eprintln!("[updater] Version {version} is not installed: {error}");
                    events::send(
                        &app,
                        "updater.failed",
                        json!({ "version": version, "message": error }),
                    );
                }
            },
        )
        .map_err(failed)?;

    Ok(json!({ "version": release.version }))
}

/// The release of the endpoint, when it is newer than this version, with a
/// version without the `v` of Tauri's.
fn newer(call: &Call<'_>) -> Result<Option<Release>, String> {
    let updates = updates(call)?;
    let mut release = fetch(&updates.endpoint)?;
    let version = parse_version(&release.version)?;
    if version <= call.app.package_info().version {
        return Ok(None);
    }
    release.version = version.to_string();

    Ok(Some(release))
}

fn updates(call: &Call<'_>) -> Result<Updates, String> {
    call.state().updates().cloned().ok_or_else(|| {
        "The application has no \"updates\" in extra.nativella of its composer.json.".to_owned()
    })
}

fn fetch(endpoint: &str) -> Result<Release, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(DOCUMENT_TIMEOUT))
        .build()
        .into();
    let mut response = agent
        .get(endpoint)
        .call()
        .map_err(|error| format!("The updates endpoint {endpoint} failed: {error}."))?;

    serde_json::from_reader(response.body_mut().as_reader())
        .map_err(|error| format!("The document of {endpoint} is invalid: {error}."))
}

/// The version of the endpoint, which Tauri's releases may name `v1.2.0`.
fn parse_version(version: &str) -> Result<Version, String> {
    Version::parse(version.trim_start_matches('v')).map_err(|error| {
        format!("The version \"{version}\" of the endpoint is not a semantic version: {error}.")
    })
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

/// Downloads the executable of `artifact` into `file`, at `download`,
/// checks its signature, and puts it in the place of `executable`.
fn replace(
    file: File,
    download: &Path,
    executable: &Path,
    artifact: &Artifact,
    updates: &Updates,
) -> Result<(), String> {
    let result = download_to(&artifact.url, file)
        .and_then(|()| {
            let bytes = fs::read(download).map_err(failed)?;
            verify(&updates.public_key, &artifact.signature, &bytes)
        })
        .and_then(|()| swap(executable, download).map_err(failed));
    if result.is_err() {
        let _ = fs::remove_file(download);
    }

    result
}

fn download_to(url: &str, mut file: File) -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|error| format!("The download of {url} failed: {error}."))?;
    let mut body = response.body_mut().with_config().limit(u64::MAX).reader();
    io::copy(&mut body, &mut file)
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

/// Puts `download` in the place of `executable`, which may be running.
#[cfg(unix)]
fn swap(executable: &Path, download: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(download, fs::Permissions::from_mode(0o755))?;
    // The running process keeps the file it started from.
    fs::rename(download, executable)
}

/// Windows does not replace a running executable, but renames it.
#[cfg(windows)]
fn swap(executable: &Path, download: &Path) -> io::Result<()> {
    let previous = executable.with_extension("old");
    let _ = fs::remove_file(&previous);
    fs::rename(executable, &previous)?;
    fs::rename(download, executable)
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

        assert_eq!(
            release.platforms["linux-x86_64"].url,
            "https://notes.example/notes"
        );
        assert!(platform().contains('-'));
    }
}

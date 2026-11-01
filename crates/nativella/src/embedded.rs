//! Applications that carry their PHP project: a bundle.
//!
//! A bundle is the project as a `.tar.gz`, then what describes it:
//!
//! ```text
//! [project .tar.gz][manifest JSON][archive length: u64 LE][manifest length: u64 LE][MAGIC]
//! ```
//!
//! - `nativella build` appends it to a copy of the `nativella` executable: a
//!   standalone desktop application.
//! - `nativella android` adds it to an APK, as the asset [`PACKAGED_BUNDLE`]
//!   (see apk/mod.rs).
//! - `nativella ios` adds it to an iOS app, as the file [`PACKAGED_BUNDLE`]
//!   next to its executable (see ios/mod.rs).
//!
//! At startup, the application extracts its project once per build, in a
//! cache directory.

use std::env;
use std::fs::{self, File};
#[cfg(test)]
use std::io::Cursor;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};

use crate::project::{Manifest, Project};

const MAGIC: &[u8; 8] = b"NTVLLA01";
const TRAILER_LENGTH: u64 = 8 + 8 + 8;

/// The bundle of a phone application, which it reads in place: an asset of
/// its APK on Android, a file of its app on iOS, next to the executable.
pub const PACKAGED_BUNDLE: &str = "nativella.bundle";

/// Paths of the project that never ship: version control, editors, the
/// builds (`dist`, where `nativella build` writes by default: an earlier
/// build would ship in the next one), tests. Relative to the project, with
/// `/`. Local overrides (`.env.local`, every path ending with `.local`),
/// which may hold secrets, credentials and SQLite databases never ship
/// either: `append_dir` leaves them out on their own, to list them.
const EXCLUDED: &[&str] = &[".git", ".idea", "dist", "node_modules", "tests"];

/// Directories that ship empty: what the application wrote on the computer
/// that built it (caches, logs, sessions, files, databases), whose
/// directories the framework expects to exist. Symfony makes the ones of
/// `var` itself.
const EMPTIED: &[&str] = &[
    "storage/app",
    "storage/framework/cache",
    "storage/framework/sessions",
    "storage/framework/views",
    "storage/logs",
    "var",
];

/// The ends of the names of the files that hold credentials, in lower case:
/// keystores and PKCS #12 files, with their private keys, and provisioning
/// profiles.
const CREDENTIALS: &[&str] = &[".jks", ".keystore", ".mobileprovision", ".p12", ".pfx"];

/// The ends of the names of the files that may hold a private key, in PEM:
/// credentials when they do. Not the others: `composer/ca-bundle` ships the
/// certificates of the authorities in a `.pem` file, which HTTPS needs.
const PEM_FILES: &[&str] = &[".key", ".p8", ".pem"];

/// The credentials of Composer, at the root of the project.
const COMPOSER_AUTH: &str = "auth.json";

/// The ends of the names of SQLite databases, in lower case, and of the
/// files SQLite keeps next to them.
const DATABASES: &[&str] = &[".db", ".sqlite", ".sqlite3"];
const DATABASE_COMPANIONS: &[&str] = &["-journal", "-shm", "-wal"];

/// What a bundle leaves out of the project, for the packaging to list:
/// paths relative to the project, with `/`.
#[derive(Debug, Default, PartialEq)]
pub struct LeftOut {
    /// `.env.local`, every path ending with `.local`.
    pub local_overrides: Vec<String>,
    /// Private keys, keystores, provisioning profiles, the `auth.json` of
    /// Composer: anyone who has the application could read them.
    pub credentials: Vec<String>,
    /// SQLite databases: the ones of the computer that built the
    /// application.
    pub databases: Vec<String>,
}

/// What the trailer says about the bundled project.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Description {
    /// Changes with every build: a new build extracts its project again.
    build: String,
    manifest: Manifest,
}

/// A bundle found in a file or in memory, not extracted yet.
pub struct Bundle<R> {
    reader: R,
    description: Description,
    archive_offset: u64,
    archive_length: u64,
}

/// The project appended to the running executable, extracted if needed.
pub fn find() -> io::Result<Option<Project>> {
    let file = File::open(env::current_exe()?)?;

    match Bundle::read(file)? {
        Some(bundle) => bundle.extract(&cache_dir()?).map(Some),
        None => Ok(None),
    }
}

/// Whether the running executable carries its project: a standalone
/// application, installed for good, as opposed to `nativella run`.
pub fn is_standalone() -> bool {
    env::current_exe()
        .and_then(File::open)
        .and_then(Bundle::read)
        .is_ok_and(|bundle| bundle.is_some())
}

/// The bundle of a phone application, read where its platform keeps it.
#[cfg(target_os = "android")]
pub type Packaged = Bundle<asset::Asset>;
#[cfg(target_os = "ios")]
pub type Packaged = Bundle<File>;

/// The bundle the phone application carries: the asset [`PACKAGED_BUNDLE`]
/// of its APK.
#[cfg(target_os = "android")]
pub fn packaged() -> io::Result<Option<Packaged>> {
    match asset::open(PACKAGED_BUNDLE)? {
        Some(asset) => Bundle::read(asset),
        None => Ok(None),
    }
}

/// The bundle the phone application carries: the file [`PACKAGED_BUNDLE`]
/// of its app, next to its executable, as iOS apps keep their resources.
#[cfg(target_os = "ios")]
pub fn packaged() -> io::Result<Option<Packaged>> {
    match File::open(env::current_exe()?.with_file_name(PACKAGED_BUNDLE)) {
        Ok(file) => Bundle::read(file),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Writes a standalone executable for `project` to `output`. Returns what it
/// left out of the project.
pub fn build(project: &Project, output: &Path) -> io::Result<LeftOut> {
    let mut base = File::open(env::current_exe()?)?;
    if Bundle::read(&mut base)?.is_some() {
        return Err(io::Error::other(
            "This executable is already a standalone application: build with the nativella executable.",
        ));
    }
    base.seek(SeekFrom::Start(0))?;

    let mut target = create(output)?;
    io::copy(&mut base, &mut target)?;
    let left_out = write_bundle(project, &mut target, &[output])?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(output, fs::Permissions::from_mode(0o755))?;
    }

    Ok(left_out)
}

impl<R: Read + Seek> Bundle<R> {
    fn read(mut reader: R) -> io::Result<Option<Self>> {
        let length = reader.seek(SeekFrom::End(0))?;
        if length < TRAILER_LENGTH {
            return Ok(None);
        }

        reader.seek(SeekFrom::End(-(TRAILER_LENGTH as i64)))?;
        let mut trailer = [0; TRAILER_LENGTH as usize];
        reader.read_exact(&mut trailer)?;
        if &trailer[16..] != MAGIC {
            return Ok(None);
        }

        let archive_length = u64::from_le_bytes(trailer[..8].try_into().expect("8 bytes"));
        let manifest_length = u64::from_le_bytes(trailer[8..16].try_into().expect("8 bytes"));
        let corrupted = || {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "The bundled application is corrupted.",
            )
        };

        let manifest_offset = length
            .checked_sub(TRAILER_LENGTH + manifest_length)
            .ok_or_else(corrupted)?;
        let archive_offset = manifest_offset
            .checked_sub(archive_length)
            .ok_or_else(corrupted)?;

        reader.seek(SeekFrom::Start(manifest_offset))?;
        let mut manifest = vec![0; usize::try_from(manifest_length).map_err(|_| corrupted())?];
        reader.read_exact(&mut manifest)?;
        let description = serde_json::from_slice(&manifest).map_err(|_| corrupted())?;

        Ok(Some(Self {
            reader,
            description,
            archive_offset,
            archive_length,
        }))
    }

    pub fn manifest(&self) -> &Manifest {
        &self.description.manifest
    }

    /// Extracts the project under `cache_dir`, unless this build already is.
    pub fn extract(mut self, cache_dir: &Path) -> io::Result<Project> {
        let dir = cache_dir
            .join(&self.description.manifest.identifier)
            .join(&self.description.build);
        let complete = dir.join(".nativella-complete");

        // A cache directory may be purged in part (Android does it when
        // storage runs low): the front controller must still be there.
        if complete.exists() && !dir.join("public/index.php").is_file() {
            eprintln!("[nativella] The extracted application is incomplete: extracting it again.");
            fs::remove_dir_all(&dir)?;
        }

        if !complete.exists() {
            eprintln!(
                "[nativella] Extracting the application to {}",
                dir.display()
            );
            // Another instance may be extracting too: each one works aside and
            // the first rename wins.
            let staging = dir.with_extension(format!("tmp-{}", std::process::id()));
            if staging.exists() {
                fs::remove_dir_all(&staging)?;
            }
            fs::create_dir_all(&staging)?;

            self.reader.seek(SeekFrom::Start(self.archive_offset))?;
            let archive = GzDecoder::new((&mut self.reader).take(self.archive_length));
            let mut archive = tar::Archive::new(archive);
            // Fresh dates: cleaners purge the oldest files of a cache first.
            archive.set_preserve_mtime(false);
            archive.unpack(&staging)?;
            File::create(staging.join(".nativella-complete"))?;

            match fs::rename(&staging, &dir) {
                Ok(()) => {}
                Err(_) if complete.exists() => fs::remove_dir_all(&staging)?,
                Err(error) => return Err(error),
            }
        }

        Ok(Project {
            dir,
            manifest: self.description.manifest,
        })
    }
}

/// Removes the builds an update replaced, next to `current`, the directory
/// of an extracted project: their data lives in the data directory, not here.
///
/// Only once no other instance can run: until then, the previous build may
/// still be running from its directory, PHP loading its files as it goes.
pub fn remove_other_builds(current: &Path) {
    let Some(Ok(entries)) = current.parent().map(fs::read_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // The staging directory (".tmp-<pid>") of an instance extracting
        // now, unless it is old: an extraction that never ended.
        let staging = path
            .extension()
            .is_some_and(|extension| extension.to_string_lossy().starts_with("tmp-"));
        let removable = if staging {
            entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .is_ok_and(|modified| modified.elapsed().is_ok_and(|age| age > ABANDONED))
        } else {
            path != current && path.join(".nativella-complete").exists()
        };

        if removable && let Err(error) = fs::remove_dir_all(&path) {
            eprintln!(
                "[nativella] Could not remove the old build {}: {error}.",
                path.display()
            );
        }
    }
}

/// A staging directory that old belongs to an extraction that was killed:
/// extracting takes seconds.
const ABANDONED: Duration = Duration::from_secs(3600);

fn create(output: &Path) -> io::Result<File> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    File::create(output)
}

/// Writes the bundle of `project` to `target`, as it goes. The `excluded`
/// files, which must exist, are left out, should they be inside the project:
/// what the packaging writes, or reads. Returns what else it left out.
pub(crate) fn write_bundle(
    project: &Project,
    target: &mut dyn Write,
    excluded: &[&Path],
) -> io::Result<LeftOut> {
    let mut walk = Walk {
        excluded: excluded
            .iter()
            .map(|path| path.canonicalize())
            .collect::<io::Result<Vec<_>>>()?,
        icon: project
            .manifest
            .icon
            .as_deref()
            .map(icon_path)
            .transpose()?,
        icon_ships: false,
        left_out: LeftOut::default(),
    };

    let mut archive = Counted {
        inner: &mut *target,
        length: 0,
    };
    let mut builder = tar::Builder::new(GzEncoder::new(&mut archive, Compression::default()));
    // Composer path repositories symlink packages: ship what they point to.
    builder.follow_symlinks(true);
    append_dir(&mut builder, &project.dir, Path::new(""), &mut walk)?;
    builder.into_inner()?.finish()?;
    // The windows of a standalone executable take it from the extracted
    // project.
    if let Some(icon) = &walk.icon
        && !walk.icon_ships
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "The icon {icon} does not ship: make it a file of the project, outside of the paths that never ship (dist, tests…)."
            ),
        ));
    }
    let archive_length = archive.length;

    let build = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let description = Description {
        build: format!("{build:x}"),
        manifest: project.manifest.clone(),
    };
    let manifest = serde_json::to_vec(&description).expect("A manifest serializes to JSON.");

    target.write_all(&manifest)?;
    target.write_all(&archive_length.to_le_bytes())?;
    target.write_all(&(manifest.len() as u64).to_le_bytes())?;
    target.write_all(MAGIC)?;
    target.flush()?;

    Ok(walk.left_out)
}

/// A writer that counts what it writes.
struct Counted<W> {
    inner: W,
    length: u64,
}

impl<W: Write> Write for Counted<W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buffer)?;
        self.length += written as u64;

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// What `append_dir` goes through the project with.
struct Walk {
    /// What the packaging writes or reads, canonical.
    excluded: Vec<PathBuf>,
    /// The icon of the manifest, relative to the project, with `/`, and
    /// whether it is among the files that ship.
    icon: Option<String>,
    icon_ships: bool,
    left_out: LeftOut,
}

/// The icon `icon` of a manifest, relative to the project, with `/`.
fn icon_path(icon: &Path) -> io::Result<String> {
    let mut parts = Vec::new();
    for component in icon.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy()),
            Component::CurDir => {}
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "The icon {} is not a path in the project, which the application carries: give a path relative to the project, without \"..\".",
                        icon.display()
                    ),
                ));
            }
        }
    }

    Ok(parts.join("/"))
}

fn append_dir<W: Write>(
    archive: &mut tar::Builder<W>,
    root: &Path,
    relative: &Path,
    walk: &mut Walk,
) -> io::Result<()> {
    let mut entries = fs::read_dir(root.join(relative))?.collect::<Result<Vec<_>, _>>()?;
    // Reproducible order.
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let path = relative.join(entry.file_name());
        let normalized = path.to_string_lossy().replace('\\', "/");
        if normalized.ends_with(".local") {
            walk.left_out.local_overrides.push(normalized);
            continue;
        }
        if EXCLUDED.contains(&normalized.as_str())
            || entry
                .path()
                .canonicalize()
                .is_ok_and(|path| walk.excluded.contains(&path))
        {
            continue;
        }

        // Follows symlinks, as the archive does.
        let metadata = fs::metadata(entry.path())?;
        if EMPTIED.contains(&normalized.as_str()) {
            if metadata.is_dir() {
                archive.append_dir(&path, entry.path())?;
            }
        } else if metadata.is_dir() {
            append_dir(archive, root, &path, walk)?;
        } else if is_credential(&normalized, &entry.path())? {
            walk.left_out.credentials.push(normalized);
        } else if is_database(&normalized) {
            walk.left_out.databases.push(normalized);
        } else {
            walk.icon_ships |= walk.icon.as_ref() == Some(&normalized);
            archive.append_path_with_name(entry.path(), &path)?;
        }
    }

    Ok(())
}

/// Whether the file `path`, `normalized` in the project, holds credentials.
fn is_credential(normalized: &str, path: &Path) -> io::Result<bool> {
    let name = normalized.to_lowercase();
    if name == COMPOSER_AUTH || CREDENTIALS.iter().any(|end| name.ends_with(end)) {
        return Ok(true);
    }
    if !PEM_FILES.iter().any(|end| name.ends_with(end)) {
        return Ok(false);
    }

    // "-----BEGIN PRIVATE KEY-----", or ENCRYPTED, EC, RSA, OPENSSH.
    let marker = b"PRIVATE KEY-----";
    Ok(fs::read(path)?
        .windows(marker.len())
        .any(|window| window == marker))
}

/// Whether the file `normalized` in the project is a SQLite database, or a
/// file SQLite keeps next to one.
fn is_database(normalized: &str) -> bool {
    let name = normalized.to_lowercase();
    let database = DATABASE_COMPANIONS
        .iter()
        .find_map(|companion| name.strip_suffix(companion))
        .unwrap_or(&name);

    DATABASES.iter().any(|end| database.ends_with(end))
}

/// The assets of an APK, read in place through the asset manager of the
/// application.
#[cfg(target_os = "android")]
pub(crate) mod asset {
    use std::ffi::CString;
    use std::io::{self, Read, Seek, SeekFrom};
    use std::ptr::NonNull;

    use jni::JavaVM;
    use jni::objects::{GlobalRef, JObject};

    /// An asset of the APK.
    pub struct Asset {
        asset: ndk::asset::Asset,
        /// The Java asset manager behind the native one, which lives as long
        /// as it does.
        _manager: GlobalRef,
    }

    // SAFETY: an AAsset may move to another thread, as long as two threads do
    // not use it at once: Asset is not Sync, and reading and seeking take
    // &mut self.
    unsafe impl Send for Asset {}

    /// Opens the asset `name` of the APK, `None` if it has none.
    pub fn open(name: &str) -> io::Result<Option<Asset>> {
        let context = ndk_context::android_context();
        // SAFETY: tao initializes the context with the VM of the process and
        // the context of the application, before the entry point runs.
        let vm = unsafe { JavaVM::from_raw(context.vm().cast()) }.map_err(io::Error::other)?;
        let mut env = vm.attach_current_thread().map_err(io::Error::other)?;
        // SAFETY: a global reference, which tao keeps for the whole process.
        let application = unsafe { JObject::from_raw(context.context().cast()) };
        let manager = env
            .call_method(
                &application,
                "getAssets",
                "()Landroid/content/res/AssetManager;",
                &[],
            )
            .and_then(|manager| manager.l())
            .and_then(|manager| env.new_global_ref(manager))
            .map_err(io::Error::other)?;

        // SAFETY: a live JNI environment, and a reference to an AssetManager.
        let native = unsafe {
            ndk_sys::AAssetManager_fromJava(env.get_raw().cast(), manager.as_obj().as_raw().cast())
        };
        let native = NonNull::new(native)
            .ok_or_else(|| io::Error::other("The application has no asset manager."))?;
        // SAFETY: the native asset manager of `manager`, which Asset keeps.
        let native = unsafe { ndk::asset::AssetManager::from_ptr(native) };
        let name = CString::new(name).map_err(io::Error::other)?;

        Ok(native.open(&name).map(|asset| Asset {
            asset,
            _manager: manager,
        }))
    }

    impl Read for Asset {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.asset.read(buffer)
        }
    }

    impl Seek for Asset {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            self.asset.seek(position)
        }
    }
}

/// The per-user cache directory of the desktop platform.
fn cache_dir() -> io::Result<PathBuf> {
    let from_env = |name: &str| {
        env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let missing = || io::Error::new(io::ErrorKind::NotFound, "The user has no home directory.");

    if cfg!(windows) {
        from_env("LOCALAPPDATA").ok_or_else(missing)
    } else if cfg!(target_os = "macos") {
        from_env("HOME")
            .map(|home| home.join("Library/Caches"))
            .ok_or_else(missing)
    } else {
        from_env("XDG_CACHE_HOME")
            .or_else(|| from_env("HOME").map(|home| home.join(".cache")))
            .ok_or_else(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bundle_round_trips() {
        let root = Path::new(env!("OUT_DIR")).join(format!("bundle-test-{}", std::process::id()));
        let project_dir = root.join("project");
        fs::create_dir_all(project_dir.join("public")).unwrap();
        fs::create_dir_all(project_dir.join("var/cache")).unwrap();
        fs::create_dir_all(project_dir.join("dist")).unwrap();
        fs::create_dir_all(project_dir.join("public/dist")).unwrap();
        fs::write(project_dir.join("public/index.php"), "<?php echo 'hi';").unwrap();
        fs::write(project_dir.join("public/dist/app.js"), "shipped").unwrap();
        fs::write(project_dir.join("var/cache/big"), "never shipped").unwrap();
        fs::write(project_dir.join("dist/notes"), "an earlier build").unwrap();
        fs::write(project_dir.join(".env.local"), "SECRET=1").unwrap();
        // What the application wrote on this computer.
        fs::create_dir_all(project_dir.join("var/share")).unwrap();
        fs::write(project_dir.join("var/notes.db"), "SQLite format 3").unwrap();
        fs::write(project_dir.join("var/share/pool"), "cached").unwrap();
        fs::create_dir_all(project_dir.join("storage/app/private")).unwrap();
        fs::write(project_dir.join("storage/app/private/upload"), "a file").unwrap();
        fs::create_dir_all(project_dir.join("database")).unwrap();
        fs::write(project_dir.join("database/app.sqlite"), "SQLite format 3").unwrap();
        fs::write(project_dir.join("database/app.sqlite-wal"), "WAL").unwrap();
        // Credentials, and a certificate that is none.
        fs::write(project_dir.join("auth.json"), r#"{"github-oauth": {}}"#).unwrap();
        fs::write(project_dir.join("release.p12"), "PKCS #12").unwrap();
        fs::write(
            project_dir.join("storage/oauth-private.key"),
            "-----BEGIN RSA PRIVATE KEY-----\n",
        )
        .unwrap();
        fs::create_dir_all(project_dir.join("vendor/composer/ca-bundle/res")).unwrap();
        fs::write(
            project_dir.join("vendor/composer/ca-bundle/res/cacert.pem"),
            "-----BEGIN CERTIFICATE-----\n",
        )
        .unwrap();
        let project = Project::load(&project_dir).unwrap();

        let mut bytes = Vec::new();
        assert_eq!(
            write_bundle(&project, &mut bytes, &[]).unwrap(),
            LeftOut {
                local_overrides: vec![".env.local".to_owned()],
                credentials: vec![
                    "auth.json".to_owned(),
                    "release.p12".to_owned(),
                    "storage/oauth-private.key".to_owned(),
                ],
                databases: vec![
                    "database/app.sqlite".to_owned(),
                    "database/app.sqlite-wal".to_owned(),
                ],
            }
        );
        let found = Bundle::read(Cursor::new(bytes)).unwrap().unwrap();
        assert_eq!(found.manifest().identifier, "dev.nativella.project");

        let extracted = found.extract(&root.join("cache")).unwrap();
        assert_eq!(
            fs::read_to_string(extracted.dir.join("public/index.php")).unwrap(),
            "<?php echo 'hi';"
        );
        assert!(!extracted.dir.join("var/cache/big").exists());
        assert!(
            !extracted.dir.join("dist").exists(),
            "The builds do not ship."
        );
        assert!(
            extracted.dir.join("public/dist/app.js").is_file(),
            "Only the dist of the project root is left out."
        );
        assert!(extracted.dir.join("var").is_dir(), "var ships empty.");
        assert_eq!(fs::read_dir(extracted.dir.join("var")).unwrap().count(), 0);
        assert!(extracted.dir.join("storage/app").is_dir());
        assert!(!extracted.dir.join("storage/app/private").exists());
        assert!(!extracted.dir.join(".env.local").exists());
        assert!(
            extracted
                .dir
                .join("vendor/composer/ca-bundle/res/cacert.pem")
                .is_file(),
            "Certificates ship."
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_icon_must_ship() {
        let root = Path::new(env!("OUT_DIR")).join(format!("icon-test-{}", std::process::id()));
        fs::create_dir_all(root.join("public")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(root.join("public/index.php"), "<?php").unwrap();
        fs::write(root.join("public/icon.png"), "PNG").unwrap();
        fs::write(root.join("tests/icon.png"), "PNG").unwrap();
        let mut project = Project::load(&root).unwrap();

        project.manifest.icon = Some(PathBuf::from("./public/icon.png"));
        write_bundle(&project, &mut Vec::new(), &[]).unwrap();

        for icon in [
            root.join("public/icon.png"),
            PathBuf::from("public/../public/icon.png"),
        ] {
            project.manifest.icon = Some(icon);
            let error = write_bundle(&project, &mut Vec::new(), &[]).unwrap_err();
            assert!(
                error.to_string().contains("relative to the project"),
                "{error}"
            );
        }
        for icon in ["tests/icon.png", "public/missing.png"] {
            project.manifest.icon = Some(PathBuf::from(icon));
            let error = write_bundle(&project, &mut Vec::new(), &[]).unwrap_err();
            assert!(error.to_string().contains("does not ship"), "{error}");
        }

        fs::remove_dir_all(root).unwrap();
    }
}

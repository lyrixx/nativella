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

use std::collections::BTreeSet;
use std::env;
use std::fs::{self, File};
#[cfg(test)]
use std::io::Cursor;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::output::Output;
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

/// What a directory behind a symbolic link leaves out when it is a Composer
/// package, linked from a path repository: its repository, and what its own
/// development installed and tests it with. The project has the dependencies
/// it runs with in its own `vendor`.
const LINKED_PACKAGE_EXCLUDED: &[&str] = &[".git", "node_modules", "tests", "vendor"];

/// The ends of the names of the files that hold credentials, in lower case:
/// keystores, PKCS #12 files and the keys of PuTTY, with their private keys,
/// and provisioning profiles.
const CREDENTIALS: &[&str] = &[
    ".jks",
    ".keystore",
    ".mobileprovision",
    ".p12",
    ".pfx",
    ".ppk",
];

/// The ends of the names of the files that may hold a private key, in PEM:
/// credentials when they do. Not the others: `composer/ca-bundle` ships the
/// certificates of the authorities in a `.pem` file, which HTTPS needs.
const PEM_FILES: &[&str] = &[".key", ".p8", ".pem"];

/// The longest of the other files that are read for a private key: the JSON
/// ones, as the key of a Google service account is, and those without an
/// extension, as OpenSSH names its keys (`id_ed25519`). A key file is a few
/// KiB; a project has too many of the others to read them whole.
const KEY_FILE_MAX_LENGTH: u64 = 64 * 1024;

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
    /// Symbolic links to a file or a directory that does not exist.
    pub broken_links: Vec<String>,
}

/// What the trailer says about the bundled project.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Description {
    /// The digest of the project: a build of other files, or for another
    /// version of Nativella, extracts its project again.
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
        Some(bundle) => {
            let cache_dir = dirs::cache_dir().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "The user has no home directory.")
            })?;
            bundle.extract(&cache_dir).map(Some)
        }
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
    let mut target = Output::create(output)?;
    // What the packaging writes, should it be inside the project: the
    // executable, and the previous one.
    let staging = target.staging().to_owned();
    let mut excluded = vec![staging.as_path()];
    if output.exists() {
        excluded.push(output);
    }
    let left_out = write_executable(project, target.file(), &excluded)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staging, fs::Permissions::from_mode(0o755))?;
    }
    target.finish()?;

    Ok(left_out)
}

/// Writes a standalone executable for `project` to `target`: this
/// executable, then the bundle, which leaves out the `excluded` files.
/// Returns what else it left out of the project.
pub(crate) fn write_executable(
    project: &Project,
    target: &mut dyn Write,
    excluded: &[&Path],
) -> io::Result<LeftOut> {
    let mut base = File::open(env::current_exe()?)?;
    if Bundle::read(&mut base)?.is_some() {
        return Err(io::Error::other(
            "This executable is already a standalone application: build with the nativella executable.",
        ));
    }
    base.seek(SeekFrom::Start(0))?;
    io::copy(&mut base, target)?;

    write_bundle(project, target, excluded)
}

impl<R: Read + Seek> Bundle<R> {
    pub(crate) fn read(mut reader: R) -> io::Result<Option<Self>> {
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

        let manifest_offset = TRAILER_LENGTH
            .checked_add(manifest_length)
            .and_then(|end| length.checked_sub(end))
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

        if !is_complete(&dir)? {
            eprintln!(
                "[nativella] Extracting the application to {}",
                dir.display()
            );
            // Another instance may be extracting too: each one works aside and
            // the first rename wins.
            let staging = Staging(dir.with_extension(format!("tmp-{}", std::process::id())));
            if staging.0.exists() {
                fs::remove_dir_all(&staging.0)?;
            }
            fs::create_dir_all(&staging.0)?;

            self.reader.seek(SeekFrom::Start(self.archive_offset))?;
            let archive = GzDecoder::new((&mut self.reader).take(self.archive_length));
            let mut archive = tar::Archive::new(archive);
            // Fresh dates: cleaners purge the oldest files of a cache first.
            archive.set_preserve_mtime(false);
            archive.unpack(&staging.0)?;
            let mut files = BTreeSet::new();
            list_files(&staging.0, "", &mut files)?;
            fs::write(
                staging.0.join(COMPLETE),
                files.into_iter().collect::<Vec<_>>().join("\0"),
            )?;

            if let Err(error) = fs::rename(&staging.0, &dir)
                && !is_complete(&dir)?
            {
                // What is there is what a cleaner left of this build, not
                // what another instance has just extracted.
                if !dir.exists() {
                    return Err(error);
                }
                eprintln!(
                    "[nativella] The extracted application is incomplete: extracting it again."
                );
                fs::remove_dir_all(&dir)?;
                fs::rename(&staging.0, &dir)?;
            }
        }

        Ok(Project {
            dir,
            manifest: self.description.manifest,
        })
    }
}

/// The file that ends an extraction: the files of the build, relative to its
/// directory, with `/`, separated by a NUL, which no file name holds.
const COMPLETE: &str = ".nativella-complete";

/// Whether `dir` holds a build extracted to its end, with all its files
/// still: a cache directory may be purged in part (Android does it when
/// storage runs low, the deepest files first), and cleaners remove files
/// without their directories.
fn is_complete(dir: &Path) -> io::Result<bool> {
    let extracted = match fs::read_to_string(dir.join(COMPLETE)) {
        Ok(extracted) => extracted,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    // Reading the directories is enough: no file is opened.
    let mut files = BTreeSet::new();
    list_files(dir, "", &mut files)?;

    Ok(extracted
        .split('\0')
        .filter(|file| !file.is_empty())
        .all(|file| files.contains(file)))
}

/// Adds the files of the directory `relative` of `root` to `files`, relative
/// to `root`, with `/`.
fn list_files(root: &Path, relative: &str, files: &mut BTreeSet<String>) -> io::Result<()> {
    for entry in fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let name = entry.file_name();
        let path = if relative.is_empty() {
            name.to_string_lossy().into_owned()
        } else {
            format!("{relative}/{}", name.to_string_lossy())
        };
        if entry.file_type()?.is_dir() {
            list_files(root, &path, files)?;
        } else {
            files.insert(path);
        }
    }

    Ok(())
}

/// Where a build is extracted, until it takes its place: removed if it does
/// not, as when the disk is full, where each start would leave another one.
struct Staging(PathBuf);

impl Drop for Staging {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0)
            && error.kind() != io::ErrorKind::NotFound
        {
            eprintln!(
                "[nativella] Could not remove {}: {error}.",
                self.0.display()
            );
        }
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
            path != current && path.join(COMPLETE).exists()
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
        walked: vec![project.dir.canonicalize()?],
    };

    let mut archive = Digested {
        inner: &mut *target,
        length: 0,
        digest: Sha256::new(),
    };
    let mut builder = tar::Builder::new(GzEncoder::new(&mut archive, Compression::default()));
    // Composer path repositories symlink packages: ship what they point to.
    builder.follow_symlinks(true);
    // Neither the dates nor the owners of the files of this computer: the
    // same files make the same archive. The extraction dates them anew.
    builder.mode(tar::HeaderMode::Deterministic);
    append_dir(&mut builder, &project.dir, Path::new(""), EXCLUDED, &mut walk)?;
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

    // Of the archive, of the manifest, and of the Nativella that runs them.
    let mut digest = archive.digest;
    digest.update(serde_json::to_vec(&project.manifest).expect("A manifest serializes to JSON."));
    digest.update(env!("CARGO_PKG_VERSION"));
    let description = Description {
        build: digest.finalize()[..8]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
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

/// A writer that counts and digests what it writes.
struct Digested<W> {
    inner: W,
    length: u64,
    digest: Sha256,
}

impl<W: Write> Write for Digested<W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buffer)?;
        self.length += written as u64;
        self.digest.update(&buffer[..written]);

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
    /// The project, then the directories behind the symbolic links that lead
    /// to the directory being walked, canonical: a link to one of them, or to
    /// what holds one, would be followed forever.
    walked: Vec<PathBuf>,
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

/// Appends the directory `relative` of `root` to the archive, but its entries
/// named in `excluded`.
fn append_dir<W: Write>(
    archive: &mut tar::Builder<W>,
    root: &Path,
    relative: &Path,
    excluded: &[&str],
    walk: &mut Walk,
) -> io::Result<()> {
    let directory = root.join(relative);
    let mut entries = fs::read_dir(&directory)
        .and_then(Iterator::collect::<Result<Vec<_>, _>>)
        .map_err(|error| at(&directory, error))?;
    // Reproducible order.
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let path = relative.join(entry.file_name());
        let source = entry.path();
        let normalized = path.to_string_lossy().replace('\\', "/");
        if normalized.ends_with(".local") {
            walk.left_out.local_overrides.push(normalized);
            continue;
        }
        if excluded.contains(&entry.file_name().to_string_lossy().as_ref())
            || source
                .canonicalize()
                .is_ok_and(|path| walk.excluded.contains(&path))
        {
            continue;
        }

        let is_link = entry
            .file_type()
            .map_err(|error| at(&source, error))?
            .is_symlink();
        // Follows symlinks, as the archive does.
        let metadata = match fs::metadata(&source) {
            Ok(metadata) => metadata,
            Err(error) if is_link && error.kind() == io::ErrorKind::NotFound => {
                walk.left_out.broken_links.push(normalized);
                continue;
            }
            Err(error) => return Err(at(&source, error)),
        };
        if EMPTIED.contains(&normalized.as_str()) {
            if metadata.is_dir() {
                archive
                    .append_dir(&path, &source)
                    .map_err(|error| at(&source, error))?;
            }
        } else if metadata.is_dir() {
            if is_link {
                append_linked_dir(archive, root, &path, walk)?;
            } else {
                append_dir(archive, root, &path, &[], walk)?;
            }
        } else if is_credential(&normalized, &source, metadata.len())
            .map_err(|error| at(&source, error))?
        {
            walk.left_out.credentials.push(normalized);
        } else if is_database(&normalized) {
            walk.left_out.databases.push(normalized);
        } else {
            walk.icon_ships |= walk.icon.as_ref() == Some(&normalized);
            archive
                .append_path_with_name(&source, &path)
                .map_err(|error| at(&source, error))?;
        }
    }

    Ok(())
}

/// Appends the directory behind the symbolic link `relative` of `root`, as
/// what it leads to ships: not at all, or empty, when it is a directory of
/// the project that does (the `public/storage` of Laravel leads to
/// `storage/app/public`); without its own development when it is a Composer
/// package.
fn append_linked_dir<W: Write>(
    archive: &mut tar::Builder<W>,
    root: &Path,
    relative: &Path,
    walk: &mut Walk,
) -> io::Result<()> {
    let link = root.join(relative);
    let target = link.canonicalize().map_err(|error| at(&link, error))?;
    if walk.walked.iter().any(|walked| walked.starts_with(&target)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "The symbolic link {} leads to {}, which holds it: following it would never end. Remove it, or leave it out of the project.",
                link.display(),
                target.display()
            ),
        ));
    }

    if let Ok(inside) = target.strip_prefix(&walk.walked[0]) {
        let inside = inside.to_string_lossy().replace('\\', "/");
        let is_under = |directories: &[&str]| {
            directories.iter().any(|directory| {
                inside
                    .strip_prefix(directory)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
            })
        };
        if is_under(EXCLUDED) {
            return Ok(());
        }
        if is_under(EMPTIED) {
            return archive
                .append_dir(relative, &link)
                .map_err(|error| at(&link, error));
        }
    }

    let excluded = if target.join("composer.json").is_file() {
        LINKED_PACKAGE_EXCLUDED
    } else {
        &[]
    };
    walk.walked.push(target);
    append_dir(archive, root, relative, excluded, walk)?;
    walk.walked.pop();

    Ok(())
}

/// `error`, with the file or the directory of the project it is about: the
/// errors of the system do not name it.
fn at(path: &Path, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}

/// Whether the file `path`, `normalized` in the project, of `length` bytes,
/// holds credentials.
fn is_credential(normalized: &str, path: &Path, length: u64) -> io::Result<bool> {
    let name = normalized.to_lowercase();
    if name == COMPOSER_AUTH || CREDENTIALS.iter().any(|end| name.ends_with(end)) {
        return Ok(true);
    }
    let file_name = name.rsplit('/').next().unwrap_or(&name);
    let may_hold_a_key = PEM_FILES.iter().any(|end| name.ends_with(end))
        || ((name.ends_with(".json") || !file_name.contains('.'))
            && length <= KEY_FILE_MAX_LENGTH);
    if !may_hold_a_key {
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

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

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
        fs::create_dir_all(project_dir.join("config")).unwrap();
        fs::write(
            project_dir.join("config/firebase.json"),
            r#"{"private_key": "-----BEGIN PRIVATE KEY-----\nMIIE"}"#,
        )
        .unwrap();
        fs::write(project_dir.join("config/app.json"), r#"{"name": "Notes"}"#).unwrap();
        fs::write(
            project_dir.join("id_ed25519"),
            "-----BEGIN OPENSSH PRIVATE KEY-----\n",
        )
        .unwrap();
        fs::write(project_dir.join("deploy.ppk"), "PuTTY-User-Key-File-3").unwrap();
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
                    "config/firebase.json".to_owned(),
                    "deploy.ppk".to_owned(),
                    "id_ed25519".to_owned(),
                    "release.p12".to_owned(),
                    "storage/oauth-private.key".to_owned(),
                ],
                databases: vec![
                    "database/app.sqlite".to_owned(),
                    "database/app.sqlite-wal".to_owned(),
                ],
                broken_links: Vec::new(),
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
        assert!(extracted.dir.join("config/app.json").is_file());
        assert!(
            extracted
                .dir
                .join("vendor/composer/ca-bundle/res/cacert.pem")
                .is_file(),
            "Certificates ship."
        );

        fs::remove_dir_all(root).unwrap();
    }

    /// The bundle of a project of two files, and where to extract it.
    fn small_bundle(name: &str) -> (Vec<u8>, PathBuf) {
        let root = Path::new(env!("OUT_DIR")).join(format!("{name}-test-{}", std::process::id()));
        fs::create_dir_all(root.join("project/public")).unwrap();
        fs::create_dir_all(root.join("project/vendor/acme")).unwrap();
        fs::write(root.join("project/public/index.php"), "<?php").unwrap();
        fs::write(root.join("project/vendor/acme/autoload.php"), "<?php").unwrap();
        let project = Project::load(&root.join("project")).unwrap();
        let mut bytes = Vec::new();
        write_bundle(&project, &mut bytes, &[]).unwrap();

        (bytes, root)
    }

    fn extract(bytes: &[u8], root: &Path) -> io::Result<Project> {
        Bundle::read(Cursor::new(bytes))
            .unwrap()
            .unwrap()
            .extract(&root.join("cache"))
    }

    #[test]
    fn a_build_purged_in_part_is_extracted_again() {
        let (bytes, root) = small_bundle("purged");
        let dir = extract(&bytes, &root).unwrap().dir;
        let autoload = dir.join("vendor/acme/autoload.php");

        // What the application writes there is none of the build's.
        fs::write(dir.join("vendor/written"), "by the application").unwrap();
        extract(&bytes, &root).unwrap();
        assert!(dir.join("vendor/written").exists(), "A whole build stays.");

        // A file is gone, the front controller and the marker are not.
        fs::remove_file(&autoload).unwrap();
        extract(&bytes, &root).unwrap();
        assert!(autoload.is_file());
        assert!(!dir.join("vendor/written").exists());

        // The files are gone, the marker first, the directories are not.
        fs::remove_file(dir.join(COMPLETE)).unwrap();
        fs::remove_file(&autoload).unwrap();
        extract(&bytes, &root).unwrap();
        assert!(autoload.is_file());
        assert!(dir.join(COMPLETE).is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_extraction_that_fails_leaves_nothing() {
        let (mut bytes, root) = small_bundle("failed");
        // No gzip header anymore.
        bytes[..10].fill(0);

        extract(&bytes, &root).unwrap_err();

        let builds = root.join("cache/dev.nativella.project");
        assert_eq!(fs::read_dir(builds).unwrap().count(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn what_a_symbolic_link_leads_to_ships_as_it_does_in_the_project() {
        use std::os::unix::fs::symlink;

        let root = Path::new(env!("OUT_DIR")).join(format!("links-test-{}", std::process::id()));
        let project_dir = root.join("project");
        fs::create_dir_all(project_dir.join("public")).unwrap();
        fs::write(project_dir.join("public/index.php"), "<?php").unwrap();
        // The public disk of Laravel, as `php artisan storage:link` links it.
        fs::create_dir_all(project_dir.join("storage/app/public")).unwrap();
        fs::write(project_dir.join("storage/app/public/upload.jpg"), "JPEG").unwrap();
        symlink("../storage/app/public", project_dir.join("public/storage")).unwrap();
        fs::create_dir_all(project_dir.join("tests/fixtures")).unwrap();
        fs::write(project_dir.join("tests/fixtures/note.md"), "A note").unwrap();
        symlink("tests/fixtures", project_dir.join("fixtures")).unwrap();
        // A package of a Composer path repository, with its own development.
        let package = root.join("packages/lib");
        for (file, content) in [
            ("composer.json", "{}"),
            ("src/Lib.php", "<?php"),
            ("src/vendor/bundled.js", "A directory of its sources"),
            ("tests/LibTest.php", "<?php"),
            ("vendor/phpunit/phpunit", "<?php"),
        ] {
            fs::create_dir_all(package.join(file).parent().unwrap()).unwrap();
            fs::write(package.join(file), content).unwrap();
        }
        fs::create_dir_all(project_dir.join("vendor/acme")).unwrap();
        symlink("../../../packages/lib", project_dir.join("vendor/acme/lib")).unwrap();
        let project = Project::load(&project_dir).unwrap();

        let mut bytes = Vec::new();
        write_bundle(&project, &mut bytes, &[]).unwrap();

        let mut archive = tar::Archive::new(GzDecoder::new(&bytes[..]));
        let paths: Vec<_> = archive
            .entries()
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path().unwrap().to_string_lossy().into_owned();
                (path, entry.header().entry_type().is_dir())
            })
            .collect();
        assert_eq!(
            paths,
            [
                ("public/index.php".to_owned(), false),
                ("public/storage".to_owned(), true),
                ("storage/app".to_owned(), true),
                ("vendor/acme/lib/composer.json".to_owned(), false),
                ("vendor/acme/lib/src/Lib.php".to_owned(), false),
                ("vendor/acme/lib/src/vendor/bundled.js".to_owned(), false),
            ]
        );

        // A link to nothing.
        symlink("missing", project_dir.join("public/broken")).unwrap();
        let left_out = write_bundle(&project, &mut Vec::new(), &[]).unwrap();
        assert_eq!(left_out.broken_links, ["public/broken"]);

        // A link to what holds it.
        symlink("..", project_dir.join("public/project")).unwrap();
        let error = write_bundle(&project, &mut Vec::new(), &[]).unwrap_err();
        assert!(error.to_string().contains("public/project"), "{error}");
        assert!(error.to_string().contains("would never end"), "{error}");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_trailer_with_lengths_beyond_the_file_is_corrupted() {
        for lengths in [[u64::MAX, 0], [0, u64::MAX], [0, u64::MAX - 8], [1 << 40, 2]] {
            let mut bytes = b"What comes before the trailer".to_vec();
            bytes.extend(lengths[0].to_le_bytes());
            bytes.extend(lengths[1].to_le_bytes());
            bytes.extend(MAGIC);

            let error = Bundle::read(Cursor::new(bytes)).err().unwrap();

            assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{lengths:?}");
        }
    }

    #[test]
    fn the_same_project_makes_the_same_bundle() {
        let root =
            Path::new(env!("OUT_DIR")).join(format!("reproducible-test-{}", std::process::id()));
        fs::create_dir_all(root.join("public")).unwrap();
        fs::write(root.join("public/index.php"), "<?php echo 'hi';").unwrap();
        let project = Project::load(&root).unwrap();
        let bundle = || {
            let mut bytes = Vec::new();
            write_bundle(&project, &mut bytes, &[]).unwrap();
            bytes
        };

        let first = bundle();
        // Another date, other permissions: the same files.
        fs::write(root.join("public/index.php"), "<?php echo 'hi';").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(root.join("public"), fs::Permissions::from_mode(0o775)).unwrap();
        }
        assert_eq!(bundle(), first);

        fs::write(root.join("public/index.php"), "<?php echo 'hello';").unwrap();
        let other = Bundle::read(Cursor::new(bundle())).unwrap().unwrap();
        let first = Bundle::read(Cursor::new(first)).unwrap().unwrap();
        assert_ne!(
            other.description.build, first.description.build,
            "Other files are another build."
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

    #[test]
    fn only_the_builds_an_update_replaced_are_removed() {
        let builds = Path::new(env!("OUT_DIR"))
            .join(format!("builds-test-{}", std::process::id()))
            .join("dev.nativella.notes");
        let build = |name: &str, complete: bool| {
            let dir = builds.join(name);
            fs::create_dir_all(&dir).unwrap();
            if complete {
                File::create(dir.join(COMPLETE)).unwrap();
            }
            dir
        };
        let current = build("1300", true);
        let replaced = build("1200", true);
        let unknown = build("notes", false);
        let extracting = build("1400.tmp-123", false);
        let abandoned = build("1100.tmp-456", false);
        File::open(&abandoned)
            .unwrap()
            .set_modified(SystemTime::now() - ABANDONED - Duration::from_secs(60))
            .unwrap();

        remove_other_builds(&current);

        assert!(current.exists());
        assert!(!replaced.exists());
        assert!(unknown.exists());
        assert!(extracting.exists());
        assert!(!abandoned.exists());

        fs::remove_dir_all(builds.parent().unwrap()).unwrap();
    }
}

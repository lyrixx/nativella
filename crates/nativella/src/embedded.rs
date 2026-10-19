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
//! - `nativella bundle` writes it alone, for an iOS build to compile it into
//!   the application library (`NATIVELLA_BUNDLE`, see build.rs).
//!
//! At startup, the application extracts its project once per build, in a
//! cache directory.

use std::env;
use std::fs::{self, File};
#[cfg(any(test, target_os = "ios"))]
use std::io::Cursor;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};

use crate::project::{Manifest, Project};

const MAGIC: &[u8; 8] = b"NTVLLA01";
const TRAILER_LENGTH: u64 = 8 + 8 + 8;

/// The bundle compiled into the library for iOS builds, empty otherwise.
#[cfg(target_os = "ios")]
static COMPILED_BUNDLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/bundle.bin"));

/// The bundle of a phone application, which it reads in place: an asset of
/// its APK on Android, a file of its app on iOS, next to the executable.
pub const PACKAGED_BUNDLE: &str = "nativella.bundle";

/// Paths of the project that never ship: version control, editors, tests.
/// Relative to the project, with `/`. Local overrides (`.env.local`, every
/// path ending with `.local`), which may hold secrets, never ship either:
/// `append_dir` leaves them out on their own, to list them.
const EXCLUDED: &[&str] = &[".git", ".idea", "node_modules", "tests"];

/// Directories that ship empty: caches, logs and sessions of the computer
/// that built the application, which the framework expects to exist.
const EMPTIED: &[&str] = &[
    "storage/framework/cache",
    "storage/framework/sessions",
    "storage/framework/views",
    "storage/logs",
    "var/cache",
    "var/log",
];

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
pub type Packaged = Bundle<Cursor<&'static [u8]>>;

/// The bundle the phone application carries: the asset [`PACKAGED_BUNDLE`] of
/// its APK, or the one compiled into the library on iOS.
#[cfg(target_os = "android")]
pub fn packaged() -> io::Result<Option<Packaged>> {
    match asset::open(PACKAGED_BUNDLE)? {
        Some(asset) => Bundle::read(asset),
        None => Ok(None),
    }
}

/// The bundle the phone application carries: the asset [`PACKAGED_BUNDLE`] of
/// its APK, or the one compiled into the library on iOS.
#[cfg(target_os = "ios")]
pub fn packaged() -> io::Result<Option<Packaged>> {
    Bundle::read(Cursor::new(COMPILED_BUNDLE))
}

/// Writes a standalone executable for `project` to `output`. Returns the
/// local overrides it left out, relative to the project.
pub fn build(project: &Project, output: &Path) -> io::Result<Vec<String>> {
    let mut base = File::open(env::current_exe()?)?;
    if Bundle::read(&mut base)?.is_some() {
        return Err(io::Error::other(
            "This executable is already a standalone application: build with the nativella executable.",
        ));
    }
    base.seek(SeekFrom::Start(0))?;

    let mut target = create(output)?;
    io::copy(&mut base, &mut target)?;
    let local_overrides = write_bundle(project, &mut target, &[output])?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(output, fs::Permissions::from_mode(0o755))?;
    }

    Ok(local_overrides)
}

/// Writes the bundle of `project` alone, for an iOS build. Returns the local
/// overrides it left out, relative to the project.
pub fn bundle(project: &Project, output: &Path) -> io::Result<Vec<String>> {
    write_bundle(project, &mut create(output)?, &[output])
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
/// what the packaging writes, or reads. Returns the local overrides left out.
pub(crate) fn write_bundle(
    project: &Project,
    target: &mut dyn Write,
    excluded: &[&Path],
) -> io::Result<Vec<String>> {
    let excluded = excluded
        .iter()
        .map(|path| path.canonicalize())
        .collect::<io::Result<Vec<_>>>()?;

    let mut archive = Counted {
        inner: &mut *target,
        length: 0,
    };
    let mut builder = tar::Builder::new(GzEncoder::new(&mut archive, Compression::default()));
    // Composer path repositories symlink packages: ship what they point to.
    builder.follow_symlinks(true);
    let mut local_overrides = Vec::new();
    append_dir(
        &mut builder,
        &project.dir,
        Path::new(""),
        &excluded,
        &mut local_overrides,
    )?;
    builder.into_inner()?.finish()?;
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

    Ok(local_overrides)
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

fn append_dir<W: Write>(
    archive: &mut tar::Builder<W>,
    root: &Path,
    relative: &Path,
    excluded: &[PathBuf],
    local_overrides: &mut Vec<String>,
) -> io::Result<()> {
    let mut entries = fs::read_dir(root.join(relative))?.collect::<Result<Vec<_>, _>>()?;
    // Reproducible order.
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let path = relative.join(entry.file_name());
        let normalized = path.to_string_lossy().replace('\\', "/");
        if normalized.ends_with(".local") {
            local_overrides.push(normalized);
            continue;
        }
        if EXCLUDED.contains(&normalized.as_str())
            || entry
                .path()
                .canonicalize()
                .is_ok_and(|path| excluded.contains(&path))
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
            append_dir(archive, root, &path, excluded, local_overrides)?;
        } else {
            archive.append_path_with_name(entry.path(), &path)?;
        }
    }

    Ok(())
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
        fs::write(project_dir.join("public/index.php"), "<?php echo 'hi';").unwrap();
        fs::write(project_dir.join("var/cache/big"), "never shipped").unwrap();
        fs::write(project_dir.join(".env.local"), "SECRET=1").unwrap();
        let project = Project::load(&project_dir).unwrap();

        assert_eq!(
            bundle(&project, &root.join("app.bundle")).unwrap(),
            [".env.local"]
        );
        let bytes = fs::read(root.join("app.bundle")).unwrap();
        let found = Bundle::read(Cursor::new(bytes)).unwrap().unwrap();
        assert_eq!(found.manifest().identifier, "dev.nativella.project");

        let extracted = found.extract(&root.join("cache")).unwrap();
        assert_eq!(
            fs::read_to_string(extracted.dir.join("public/index.php")).unwrap(),
            "<?php echo 'hi';"
        );
        assert!(!extracted.dir.join("var/cache/big").exists());
        assert!(
            extracted.dir.join("var/cache").is_dir(),
            "Caches ship empty."
        );
        assert!(!extracted.dir.join(".env.local").exists());

        fs::remove_dir_all(root).unwrap();
    }
}

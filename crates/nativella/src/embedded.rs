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
//! - `nativella bundle` writes it alone, for a mobile build to compile it into
//!   the application library (`NATIVELLA_BUNDLE`, see build.rs).
//!
//! At startup, the application extracts its project once per build, in a
//! cache directory.

use std::env;
use std::fs::{self, File};
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};

use crate::project::{Manifest, Project};

const MAGIC: &[u8; 8] = b"NTVLLA01";
const TRAILER_LENGTH: u64 = 8 + 8 + 8;

/// The bundle compiled into the library for mobile builds, empty otherwise.
static COMPILED_BUNDLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/bundle.bin"));

/// Paths of the project that never ship: version control, local overrides
/// that may hold secrets, tests. Relative to the project, with `/`.
const EXCLUDED: &[&str] = &[".env.local", ".git", ".idea", "node_modules", "tests"];

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

/// The bundle compiled into the library, for mobile builds.
pub fn compiled() -> io::Result<Option<Bundle<Cursor<&'static [u8]>>>> {
    Bundle::read(Cursor::new(COMPILED_BUNDLE))
}

/// Writes a standalone executable for `project` to `output`.
pub fn build(project: &Project, output: &Path) -> io::Result<()> {
    let mut base = File::open(env::current_exe()?)?;
    if Bundle::read(&mut base)?.is_some() {
        return Err(io::Error::other(
            "This executable is already a standalone application: build with the nativella executable.",
        ));
    }
    base.seek(SeekFrom::Start(0))?;

    let mut target = create(output)?;
    io::copy(&mut base, &mut target)?;
    write_bundle(project, &mut target, output)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(output, fs::Permissions::from_mode(0o755))?;
    }

    Ok(())
}

/// Writes the bundle of `project` alone, for a mobile build.
pub fn bundle(project: &Project, output: &Path) -> io::Result<()> {
    write_bundle(project, &mut create(output)?, output)
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

/// `output` is left out, should it be inside the project.
fn write_bundle(project: &Project, target: &mut File, output: &Path) -> io::Result<()> {
    let output = output.canonicalize()?;
    let archive_offset = target.stream_position()?;

    let mut archive = tar::Builder::new(GzEncoder::new(&mut *target, Compression::default()));
    // Composer path repositories symlink packages: ship what they point to.
    archive.follow_symlinks(true);
    append_dir(&mut archive, &project.dir, Path::new(""), &output)?;
    archive.into_inner()?.finish()?;
    let archive_length = target.stream_position()? - archive_offset;

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
    target.flush()
}

fn append_dir<W: Write>(
    archive: &mut tar::Builder<W>,
    root: &Path,
    relative: &Path,
    output: &Path,
) -> io::Result<()> {
    let mut entries = fs::read_dir(root.join(relative))?.collect::<Result<Vec<_>, _>>()?;
    // Reproducible order.
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let path = relative.join(entry.file_name());
        let normalized = path.to_string_lossy().replace('\\', "/");
        if EXCLUDED.contains(&normalized.as_str())
            || normalized.ends_with(".local")
            || entry.path().canonicalize().is_ok_and(|path| path == output)
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
            append_dir(archive, root, &path, output)?;
        } else {
            archive.append_path_with_name(entry.path(), &path)?;
        }
    }

    Ok(())
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

        bundle(&project, &root.join("app.bundle")).unwrap();
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

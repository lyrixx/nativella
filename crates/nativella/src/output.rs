//! What the packaging writes: a file written next to its path, which takes
//! its place once complete. A packaging that fails leaves no file cut short,
//! and the previous one in place.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

/// A file being written next to its path.
pub(crate) struct Output {
    file: File,
    staging: Staging,
    path: PathBuf,
}

/// Where an output is written, until it takes the place of its path: removed
/// unless it does.
struct Staging(PathBuf);

impl Output {
    /// Starts writing `path`, aside, in a new file of a name no one can
    /// guess: it could be a link to another file otherwise.
    pub fn create(path: &Path) -> io::Result<Self> {
        let name = path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("The output {} is not the path of a file.", path.display()),
            )
        })?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;

        let mut random = [0; 8];
        getrandom::fill(&mut random).map_err(io::Error::other)?;
        let random: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let staging = parent.join(format!(".{}.{random}.tmp", name.to_string_lossy()));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&staging)?;

        Ok(Self {
            file,
            staging: Staging(staging),
            path: path.to_owned(),
        })
    }

    pub fn file(&mut self) -> &mut File {
        &mut self.file
    }

    /// Where the file is written: inside the project, it must not ship.
    pub fn staging(&self) -> &Path {
        &self.staging.0
    }

    /// Closes the file, then puts it in place of its path.
    pub fn finish(self) -> io::Result<()> {
        let Self {
            file,
            mut staging,
            path,
        } = self;
        file.sync_all()?;
        drop(file);

        fs::rename(&staging.0, &path)?;
        staging.0 = PathBuf::new();

        Ok(())
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        if self.0.as_os_str().is_empty() {
            return;
        }
        if let Err(error) = fs::remove_file(&self.0) {
            eprintln!("Warning: cannot remove {}: {error}.", self.0.display());
        }
    }
}

/// Refuses an `output` that is the `input` of the packaging: writing it would
/// destroy it.
#[cfg(desktop)]
pub(crate) fn refuse_input(output: &Path, input: &Path) -> io::Result<()> {
    let same = output
        .canonicalize()
        .and_then(|output| Ok(output == input.canonicalize()?))
        .unwrap_or(false);
    if !same {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "The packaging would write over {}, which it reads: choose another output.",
            input.display()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn the_output_takes_its_place_once_complete() {
        let dir = Path::new(env!("OUT_DIR")).join(format!("output-test-{}", std::process::id()));
        let path = dir.join("app");

        let mut output = Output::create(&path).unwrap();
        output.file().write_all(b"cut").unwrap();
        drop(output);
        assert!(!path.exists(), "A failure leaves nothing.");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);

        fs::write(&path, "previous").unwrap();
        let mut output = Output::create(&path).unwrap();
        output.file().write_all(b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"previous");
        output.finish().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);

        assert!(refuse_input(&path, &dir.join("./app")).is_err());
        assert!(refuse_input(&dir.join("other"), &path).is_ok());

        fs::remove_dir_all(dir).unwrap();
    }
}

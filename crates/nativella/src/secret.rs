//! A secret unique to each installation, for PHP frameworks to sign and
//! encrypt with (Symfony's APP_SECRET, Laravel's APP_KEY): an application
//! ships its code to every user, it cannot ship a secret.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const FILE: &str = ".nativella-secret";

/// The secret of the installation, created on first use: 32 random bytes,
/// hex-encoded.
///
/// A file that holds no such secret is set aside, never overwritten: what
/// was encrypted with it is lost without it. The application starts with a
/// new secret, and the previous one can be put back.
pub(crate) fn load(data_dir: &Path) -> io::Result<String> {
    let path = data_dir.join(FILE);

    match fs::read(&path) {
        Ok(content) => {
            // A copy restored by hand may end with a line break.
            match std::str::from_utf8(&content).map(str::trim) {
                Ok(secret) if is_secret(secret) => return Ok(secret.to_owned()),
                _ => {
                    let seconds = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_or(0, |elapsed| elapsed.as_secs());
                    let aside = path.with_extension(format!("corrupted-{seconds}"));
                    fs::rename(&path, &aside)?;
                    eprintln!(
                        "[nativella] {} holds no secret of Nativella: it is now {}, and a new secret replaces it. What was encrypted with the previous one is unreadable until it is put back.",
                        path.display(),
                        aside.display()
                    );
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(io::Error::other)?;
    let secret: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();

    // Readable by the user only from the start, written aside then renamed.
    let temporary = path.with_extension("tmp");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(secret.as_bytes())?;
    // On the disk before it has its name: after a power cut, an empty file
    // would be set aside for a new secret, and the first one lost.
    file.sync_all()?;
    fs::rename(&temporary, &path)?;

    Ok(secret)
}

fn is_secret(secret: &str) -> bool {
    secret.len() == 64 && secret.chars().all(|char| char.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory(name: &str) -> std::path::PathBuf {
        let dir = Path::new(env!("OUT_DIR")).join(format!("secret-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        dir
    }

    #[test]
    fn the_secret_is_kept() {
        let dir = directory("kept");

        let secret = load(&dir).unwrap();
        assert!(is_secret(&secret));
        assert_eq!(load(&dir).unwrap(), secret);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_secret_restored_with_a_line_break_is_the_same() {
        let dir = directory("restored");
        let secret = "0123456789abcdef".repeat(4);
        fs::write(dir.join(FILE), format!("{secret}\r\n")).unwrap();

        assert_eq!(load(&dir).unwrap(), secret);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_corrupted_secret_is_set_aside() {
        let dir = directory("corrupted");
        fs::write(dir.join(FILE), b"0123\xff").unwrap();

        let secret = load(&dir).unwrap();
        assert!(is_secret(&secret));
        let aside: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.starts_with(".nativella-secret.corrupted-"))
            .collect();
        assert_eq!(aside.len(), 1);
        assert_eq!(fs::read(dir.join(&aside[0])).unwrap(), b"0123\xff");

        fs::remove_dir_all(dir).unwrap();
    }
}

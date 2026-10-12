//! A secret unique to each installation, for PHP frameworks to sign and
//! encrypt with (Symfony's APP_SECRET, Laravel's APP_KEY): an application
//! ships its code to every user, it cannot ship a secret.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

const FILE: &str = ".nativella-secret";

/// The secret of the installation, created on first use: 32 random bytes,
/// hex-encoded.
pub(crate) fn load(data_dir: &Path) -> io::Result<String> {
    let path = data_dir.join(FILE);

    match fs::read_to_string(&path) {
        Ok(secret) if secret.len() == 64 && secret.chars().all(|char| char.is_ascii_hexdigit()) => {
            return Ok(secret);
        }
        Ok(_) => eprintln!(
            "[nativella] {} is corrupted: a new secret replaces it.",
            path.display()
        ),
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
    options.open(&temporary)?.write_all(secret.as_bytes())?;
    fs::rename(&temporary, &path)?;

    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_secret_is_kept() {
        let dir = Path::new(env!("OUT_DIR")).join(format!("secret-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let secret = load(&dir).unwrap();
        assert_eq!(secret.len(), 64);
        assert!(secret.chars().all(|char| char.is_ascii_hexdigit()));
        assert_eq!(load(&dir).unwrap(), secret);

        fs::remove_dir_all(dir).unwrap();
    }
}

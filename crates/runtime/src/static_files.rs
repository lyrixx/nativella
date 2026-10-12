//! Serves the files of the document root that exist on disk, the way a web
//! server would before falling back to the front controller.

use std::fs;
use std::path::{Component, Path, PathBuf};

use http::header::CONTENT_TYPE;
use http::{Response, StatusCode};
use percent_encoding::percent_decode_str;

/// Returns the response for `path` if it names a public file, `None` if the
/// request is for PHP.
pub(crate) fn serve(document_root: &Path, path: &str) -> Option<Response<Vec<u8>>> {
    let file = public_file(document_root, path)?;

    let body = match fs::read(&file) {
        Ok(body) => body,
        Err(error) => {
            eprintln!("[static] Could not read {}: {error}.", file.display());

            return Some(
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Vec::new())
                    .expect("A response with a status and no header is valid."),
            );
        }
    };

    let content_type = mime_guess::from_path(&file).first_or_octet_stream();

    Some(
        Response::builder()
            .header(CONTENT_TYPE, content_type.as_ref())
            .body(body)
            .expect("A guessed MIME type is a valid header value."),
    )
}

/// The file of the document root that `path` names, if any.
///
/// The path may only go down: no `..`, no absolute part. Symlinks are
/// followed wherever they point, as web servers do: the developer made them
/// (Laravel's `storage:link`, for one).
fn public_file(document_root: &Path, path: &str) -> Option<PathBuf> {
    let decoded = percent_decode_str(path).decode_utf8().ok()?;
    let relative = Path::new(decoded.trim_start_matches('/'));

    if relative.as_os_str().is_empty()
        || !relative
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return None;
    }

    let file = document_root.join(relative);
    // What the file system actually opens: `/INDEX.PHP` on a
    // case-insensitive one, `index.php.` on Windows.
    let target = file.canonicalize().ok()?;
    if !target.is_file() || is_php(&target) || is_php(&file) {
        return None;
    }

    Some(file)
}

fn is_php(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("php"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        let root = Path::new(env!("OUT_DIR")).join(format!("static-{}", std::process::id()));
        fs::create_dir_all(root.join("public/css")).unwrap();
        fs::create_dir_all(root.join("storage/app/public")).unwrap();
        fs::write(root.join("public/css/app.css"), "body {}").unwrap();
        fs::write(root.join("public/index.php"), "<?php").unwrap();
        fs::write(root.join("storage/app/public/avatar.png"), "png").unwrap();
        fs::write(root.join("secret.txt"), "secret").unwrap();
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(
                root.join("storage/app/public"),
                root.join("public/storage"),
            );
        }

        root
    }

    #[test]
    fn serves_public_files_only() {
        let root = root();
        let public = root.join("public");

        assert!(serve(&public, "/css/app.css").is_some());
        assert!(serve(&public, "/").is_none());
        assert!(serve(&public, "/index.php").is_none());
        assert!(serve(&public, "/INDEX.PHP").is_none());
        assert!(serve(&public, "/../secret.txt").is_none());
        assert!(serve(&public, "/css/%2e%2e/%2e%2e/secret.txt").is_none());
        assert!(serve(&public, "/missing.css").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn follows_the_symlinks_of_the_developer() {
        let root = root();

        assert!(serve(&root.join("public"), "/storage/avatar.png").is_some());
    }
}

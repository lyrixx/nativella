//! Serves the files of the document root that exist on disk, the way a web
//! server would before falling back to the front controller.

use std::fs;
use std::path::Path;

use http::header::CONTENT_TYPE;
use http::{Response, StatusCode};
use percent_encoding::percent_decode_str;

/// Returns the response for `path` if it names a public file, `None` if the
/// request is for PHP.
pub(crate) fn serve(document_root: &Path, path: &str) -> Option<Response<Vec<u8>>> {
    let decoded = percent_decode_str(path).decode_utf8().ok()?;
    let relative = decoded.trim_start_matches('/');

    if relative.is_empty() || relative.ends_with(".php") {
        return None;
    }

    // Canonicalizing resolves `..` and symlinks: anything that ends up outside
    // the document root is not served.
    let file = document_root.join(relative).canonicalize().ok()?;
    if !file.starts_with(document_root) || !file.is_file() {
        return None;
    }

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

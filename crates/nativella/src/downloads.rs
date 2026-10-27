//! Downloads: the responses a browser would save rather than show, saved
//! where the user chooses.
//!
//! The webview cannot download from the `php` scheme: WebKitGTK drops an
//! attachment, and fails a link with a `download` attribute without asking
//! PHP. So Nativella takes the response itself: an attachment answering a
//! navigation (a link, a form), or any response to a request that downloads.js
//! marks (a link with a `download` attribute), opens a save dialog. The
//! webview gets an empty response of a type it cannot show, and stays on the
//! page: a 204 No Content would do over HTTP only, WebKitGTK shows an empty
//! page for it on a custom scheme.
//!
//! What a page makes itself (`blob:` and `data:` URLs) goes through the
//! webview, to a temporary file, then to the same dialog.
//!
//! Desktop only: phones save through a content URI, which PHP gets no say
//! in yet.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, thread};

use percent_encoding::percent_decode_str;
use serde_json::json;
use tauri::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE, HeaderMap};
use tauri::http::{Request, Response};
use tauri::webview::{DownloadEvent, Url};
use tauri::{AppHandle, Manager, Webview};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::events;

/// Injected in every page: links with a `download` attribute.
pub(crate) const SCRIPT: &str = include_str!("downloads.js");

/// What downloads.js marks its requests with: the `download` attribute of
/// the link, percent-encoded, empty when it names no file.
const HEADER: &str = "x-nativella-download";

/// The name of a file when nothing else gives one.
const FALLBACK_NAME: &str = "download";

/// The files the webview downloads for the pages (`blob:` and `data:`
/// URLs), by URL, until they are finished.
#[derive(Default)]
pub(crate) struct Pending(Mutex<HashMap<String, VecDeque<PathBuf>>>);

/// Wraps `respond`, of a request of the window `window`, so that a response
/// to download is saved rather than handed to the webview.
pub(crate) fn intercept(
    app: &AppHandle,
    window: &str,
    request: &Request<Vec<u8>>,
    respond: impl FnOnce(Response<Vec<u8>>) + Send + 'static,
) -> impl FnOnce(Response<Vec<u8>>) + Send + 'static {
    let app = app.clone();
    let window = window.to_owned();
    let asked = Asked::of(request);
    let url = request.uri().to_string();

    move |response: Response<Vec<u8>>| {
        let Some(name) = asked.download(&url, &response) else {
            respond(response);
            return;
        };

        respond(
            Response::builder()
                .header(CONTENT_TYPE, "application/octet-stream")
                .body(Vec::new())
                .expect("An empty response is valid."),
        );
        let body = response.into_body();
        let spawned = thread::Builder::new()
            .name("download".to_owned())
            .spawn(move || save(&app, &window, &name, |path| fs::write(path, &body)));
        if let Err(error) = spawned {
            eprintln!("[download] {url} is not saved: {error}.");
        }
    }
}

/// The downloads of the webview: `blob:` and `data:` URLs, which a page
/// makes itself. Requests of the application never get there (see
/// [`intercept`]), and the webview cannot download them: they are cancelled.
pub(crate) fn on_download(webview: Webview, event: DownloadEvent<'_>) -> bool {
    let app = webview.app_handle().clone();
    let pending = app.state::<Pending>();

    match event {
        DownloadEvent::Requested { url, destination } => {
            if !matches!(url.scheme(), "blob" | "data") {
                eprintln!("[download] {} cannot be downloaded.", short(&url));
                return false;
            }
            // The webview names the file in the downloads directory.
            let name = destination
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| FALLBACK_NAME.to_owned());
            let temporary = match temporary_file(&app, &name) {
                Ok(path) => path,
                Err(error) => {
                    eprintln!("[download] {name} cannot be downloaded: {error}.");
                    return false;
                }
            };
            *destination = temporary.clone();
            lock(&pending)
                .entry(url.to_string())
                .or_default()
                .push_back(temporary);
        }
        DownloadEvent::Finished { url, success, .. } => {
            let temporary = {
                let mut pending = lock(&pending);
                let temporary = pending.get_mut(url.as_str()).and_then(VecDeque::pop_front);
                pending.retain(|_, paths| !paths.is_empty());
                temporary
            };
            // WebKitGTK reports it to every window: the first one takes it.
            let Some(temporary) = temporary else {
                return true;
            };

            if !success {
                eprintln!("[download] {} failed.", short(&url));
                remove_temporary(&temporary);
                return true;
            }
            let name = temporary
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| FALLBACK_NAME.to_owned());
            let window = webview.label().to_owned();
            let spawned = thread::Builder::new()
                .name("download".to_owned())
                .spawn(move || {
                    save(&app, &window, &name, |path| move_file(&temporary, path));
                    remove_temporary(&temporary);
                });
            if let Err(error) = spawned {
                eprintln!("[download] {} is not saved: {error}.", short(&url));
            }
        }
        _ => {}
    }

    true
}

/// What a request says of a download.
struct Asked {
    /// The `download` attribute of the link downloads.js fetched, decoded.
    marked: Option<String>,
    /// Whether the webview would show the response as a page: a link, a
    /// form.
    navigation: bool,
}

impl Asked {
    fn of(request: &Request<Vec<u8>>) -> Self {
        Self {
            marked: request
                .headers()
                .get(HEADER)
                .and_then(|value| value.to_str().ok())
                .map(|name| percent_decode_str(name).decode_utf8_lossy().into_owned()),
            navigation: nativella_runtime::is_navigation(request),
        }
    }

    /// The name to suggest for `response`, when it is to download.
    fn download(&self, url: &str, response: &Response<Vec<u8>>) -> Option<String> {
        let disposition = disposition(response.headers());

        // A script that fetches an attachment wants its body, as in a browser.
        let attachment = disposition
            .as_ref()
            .is_some_and(|disposition| disposition.attachment);
        if self.marked.is_none() && !(self.navigation && attachment) {
            return None;
        }
        if !response.status().is_success() {
            eprintln!(
                "[download] PHP answered {} to {url}: nothing to save.",
                response.status()
            );
            return None;
        }

        let name = self
            .marked
            .clone()
            .filter(|name| !name.is_empty())
            .or_else(|| disposition.and_then(|disposition| disposition.file_name))
            .or_else(|| last_segment(url));

        Some(name.map_or_else(|| FALLBACK_NAME.to_owned(), |name| safe_name(&name)))
    }
}

/// A `Content-Disposition` header.
#[derive(Debug, PartialEq)]
struct Disposition {
    attachment: bool,
    file_name: Option<String>,
}

/// Reads the `Content-Disposition` of `headers` (RFC 6266): `filename*`, in
/// UTF-8 or Latin-1, wins over `filename`.
fn disposition(headers: &HeaderMap) -> Option<Disposition> {
    let value = headers.get(CONTENT_DISPOSITION)?.as_bytes();
    // Latin-1, as HTTP has it: each byte is a character.
    let value: String = value.iter().map(|&byte| char::from(byte)).collect();
    let mut parts = split_parameters(&value).into_iter();

    let attachment = parts
        .next()
        .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("attachment"));
    let mut plain = None;
    let mut extended = None;
    for part in parts {
        let Some((name, value)) = part.split_once('=') else {
            continue;
        };
        match name.trim().to_ascii_lowercase().as_str() {
            "filename" => plain = Some(unquote(value.trim())),
            "filename*" => extended = extended_value(value.trim()),
            _ => {}
        }
    }

    Some(Disposition {
        attachment,
        file_name: extended.or(plain).filter(|name| !name.is_empty()),
    })
}

/// Splits a header value at the semicolons that are not quoted.
fn split_parameters(value: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut quoted = false;
    let mut escaped = false;

    for character in value.chars() {
        let part = parts.last_mut().expect("There is always a part.");
        match character {
            _ if escaped => {
                escaped = false;
                part.push(character);
            }
            '\\' if quoted => {
                escaped = true;
                part.push(character);
            }
            '"' => {
                quoted = !quoted;
                part.push(character);
            }
            ';' if !quoted => parts.push(String::new()),
            _ => part.push(character),
        }
    }

    parts
}

/// `"a \"quoted\" string"`, or a token as it is.
fn unquote(value: &str) -> String {
    let Some(inner) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return value.to_owned();
    };

    let mut unquoted = String::new();
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        match character {
            '\\' => unquoted.extend(characters.next()),
            _ => unquoted.push(character),
        }
    }

    unquoted
}

/// `UTF-8''r%C3%A9sum%C3%A9.pdf` (RFC 8187): the charsets HTTP clients must
/// know, UTF-8 and Latin-1.
fn extended_value(value: &str) -> Option<String> {
    let mut parts = value.splitn(3, '\'');
    let charset = parts.next()?;
    let _language = parts.next()?;
    let bytes: Vec<u8> = percent_decode_str(parts.next()?).collect();

    if charset.eq_ignore_ascii_case("utf-8") {
        String::from_utf8(bytes).ok()
    } else if charset.eq_ignore_ascii_case("iso-8859-1") {
        Some(bytes.into_iter().map(char::from).collect())
    } else {
        None
    }
}

/// The last segment of the path of `url`, decoded: `report.pdf` for
/// `php://localhost/files/report.pdf?v=2`.
fn last_segment(url: &str) -> Option<String> {
    let url = Url::parse(url).ok()?;
    let segment = url.path_segments()?.next_back()?;

    Some(percent_decode_str(segment).decode_utf8_lossy().into_owned())
        .filter(|name| !name.is_empty())
}

/// A name that cannot point outside the directory the user picks: only
/// what follows the last separator, without control characters.
fn safe_name(name: &str) -> String {
    let name: String = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .chars()
        .filter(|character| !character.is_control())
        .collect();
    let name = name.trim();

    match name {
        "" | "." | ".." => FALLBACK_NAME.to_owned(),
        _ => name.to_owned(),
    }
}

/// Asks the user where to save `name`, then `write`s it there, and tells
/// PHP and the pages, with `window.downloaded`. Blocks: not on the main
/// thread.
fn save(
    app: &AppHandle,
    window: &str,
    name: &str,
    write: impl FnOnce(&Path) -> std::io::Result<()>,
) {
    let mut dialog = app.dialog().file().set_file_name(name);
    if let Ok(directory) = app.path().download_dir() {
        dialog = dialog.set_directory(directory);
    }
    // The window that downloaded, when it is still there.
    let parent = app.get_webview_window(window);
    if let Some(parent) = &parent {
        dialog = dialog.set_parent(parent);
    }

    let Some(picked) = dialog.blocking_save_file() else {
        return;
    };
    let path = match picked.into_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("[download] {name} is not saved: {error}.");
            return;
        }
    };

    if let Err(error) = write(&path) {
        eprintln!("[download] {} is not saved: {error}.", path.display());
        let mut message = app
            .dialog()
            .message(format!("{} could not be saved: {error}.", path.display()))
            .kind(MessageDialogKind::Error);
        if let Some(parent) = &parent {
            message = message.parent(parent);
        }
        message.show(|_| {});
        return;
    }

    match crate::native::dialog::utf8(path) {
        Ok(path) => events::send(
            app,
            "window.downloaded",
            json!({ "window": window, "path": path }),
        ),
        Err(error) => eprintln!("[download] Saved, but PHP is not told: {error}"),
    }
}

/// A file in the cache directory of the application, in a directory of its
/// own, for the webview to download to.
fn temporary_file(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?
        .join("downloads")
        .join(format!("{nanos:x}"));
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;

    Ok(directory.join(safe_name(name)))
}

/// Moves `from` to `to`, across file systems too.
fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }

    fs::copy(from, to).map(|_| ())
}

/// Removes a temporary file and its directory, if they are still there.
fn remove_temporary(path: &Path) {
    if let Some(directory) = path.parent() {
        let _ = fs::remove_dir_all(directory);
    }
}

/// A URL for the logs: `data:` ones can be long.
fn short(url: &Url) -> String {
    let url = url.as_str();
    match url.char_indices().nth(80) {
        Some((end, _)) => format!("{}…", &url[..end]),
        None => url.to_owned(),
    }
}

fn lock(pending: &Pending) -> std::sync::MutexGuard<'_, HashMap<String, VecDeque<PathBuf>>> {
    pending
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(disposition: Option<&str>) -> Response<Vec<u8>> {
        let mut response = Response::builder();
        if let Some(disposition) = disposition {
            response = response.header(CONTENT_DISPOSITION, disposition);
        }

        response.body(b"a,b".to_vec()).unwrap()
    }

    fn navigation() -> Asked {
        Asked {
            marked: None,
            navigation: true,
        }
    }

    #[test]
    fn an_attachment_answering_a_navigation_is_downloaded() {
        let url = "php://localhost/notes/export";

        assert_eq!(
            navigation().download(url, &response(Some("attachment; filename=\"notes.csv\""))),
            Some("notes.csv".to_owned())
        );
        assert_eq!(
            navigation().download(url, &response(Some("attachment"))),
            Some("export".to_owned())
        );
        assert_eq!(
            navigation().download(url, &response(Some("inline; filename=a.pdf"))),
            None
        );
        assert_eq!(navigation().download(url, &response(None)), None);
    }

    #[test]
    fn a_script_gets_the_attachments_it_fetches() {
        let script = Asked {
            marked: None,
            navigation: false,
        };

        assert_eq!(
            script.download("php://localhost/export", &response(Some("attachment"))),
            None
        );
    }

    #[test]
    fn a_marked_request_is_downloaded_with_the_name_of_its_link() {
        let marked = |name: &str| Asked {
            marked: Some(name.to_owned()),
            navigation: false,
        };
        let url = "php://localhost/files/report%201.pdf?v=2";

        assert_eq!(
            marked("Rapport.pdf").download(url, &response(Some("attachment; filename=r.pdf"))),
            Some("Rapport.pdf".to_owned())
        );
        assert_eq!(
            marked("").download(url, &response(Some("attachment; filename=r.pdf"))),
            Some("r.pdf".to_owned())
        );
        assert_eq!(
            marked("").download(url, &response(None)),
            Some("report 1.pdf".to_owned())
        );
        assert_eq!(
            marked("").download("php://localhost/", &response(None)),
            Some(FALLBACK_NAME.to_owned())
        );
    }

    #[test]
    fn an_error_is_not_downloaded() {
        let mut error = response(Some("attachment; filename=a.csv"));
        *error.status_mut() = tauri::http::StatusCode::NOT_FOUND;

        assert_eq!(navigation().download("php://localhost/a.csv", &error), None);
    }

    #[test]
    fn the_extended_file_name_wins() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_DISPOSITION,
            "attachment; filename=\"ete.pdf\"; filename*=UTF-8''%C3%A9t%C3%A9.pdf"
                .parse()
                .unwrap(),
        );

        assert_eq!(
            disposition(&headers),
            Some(Disposition {
                attachment: true,
                file_name: Some("été.pdf".to_owned()),
            })
        );
    }

    #[test]
    fn file_names_are_unquoted() {
        let file_name = |value: &str| {
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_DISPOSITION, value.parse().unwrap());
            disposition(&headers).and_then(|disposition| disposition.file_name)
        };

        assert_eq!(
            file_name("Attachment; FILENAME=\"a; \\\"b\\\".txt\""),
            Some("a; \"b\".txt".to_owned())
        );
        assert_eq!(
            file_name("attachment; filename=plain.txt"),
            Some("plain.txt".to_owned())
        );
        assert_eq!(
            file_name("attachment; filename*=iso-8859-1'fr'%E9t%E9.txt"),
            Some("été.txt".to_owned())
        );
        assert_eq!(file_name("attachment; filename*=koi8-r''x.txt"), None);
        assert_eq!(file_name("attachment; filename=\"\""), None);
    }

    #[test]
    fn names_stay_in_the_directory() {
        assert_eq!(safe_name("../../.bashrc"), ".bashrc");
        assert_eq!(safe_name("C:\\Windows\\evil.dll"), "evil.dll");
        assert_eq!(safe_name("a\nb.txt"), "ab.txt");
        assert_eq!(safe_name(".."), FALLBACK_NAME);
        assert_eq!(safe_name("dir/"), FALLBACK_NAME);
        assert_eq!(safe_name(" notes.md "), "notes.md");
    }
}

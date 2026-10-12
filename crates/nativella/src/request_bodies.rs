//! Android's WebView hands custom schemes requests without their body: a form
//! POST or a `fetch()` POST would reach PHP empty.
//!
//! A script injected in every page captures the body of same-origin
//! requests, encoded by the browser itself, and sends it through Tauri's IPC
//! before the request: keyed by an `X-Nativella-Body` header for `fetch()`
//! and XHR, by a `nativella-body` parameter of the URL for a form, which
//! navigates without custom headers.
//! The scheme handler puts the body back before PHP sees the request.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use tauri::http::header::CONTENT_TYPE;
use tauri::http::{HeaderValue, Request};
use tauri::{Manager, Runtime};

use crate::native::lock;

/// The header a request carries its body key in.
const KEY_HEADER: &str = "x-nativella-body";

/// The parameter the URL of a form carries its body key in.
const QUERY_KEY: &str = "nativella-body=";

/// A body nobody claimed in this time was for a request that never came.
const LIFETIME: Duration = Duration::from_secs(60);

/// Injected in every page, before its own scripts.
pub(crate) const SCRIPT: &str = include_str!("request_bodies.js");

struct Body {
    content_type: String,
    bytes: Vec<u8>,
    stored: Instant,
}

/// The bodies sent by the pages, waiting for their request.
#[derive(Default)]
pub(crate) struct Bodies(Mutex<HashMap<String, Body>>);

/// Called by the page script before its request. Android's IPC carries JSON
/// only: the body comes base64-encoded.
#[tauri::command]
pub(crate) fn nativella_store_body(
    key: String,
    content_type: String,
    body: String,
    bodies: tauri::State<'_, Bodies>,
) -> Result<(), String> {
    let bytes = BASE64_STANDARD
        .decode(body)
        .map_err(|error| format!("The body is not base64: {error}."))?;

    let mut bodies = lock(&bodies.0);
    let now = Instant::now();
    bodies.retain(|_, body| now.duration_since(body.stored) < LIFETIME);
    bodies.insert(
        key,
        Body {
            content_type,
            bytes,
            stored: now,
        },
    );

    Ok(())
}

/// Whether `fetch()` or XHR sent `request`, rather than a form: only they
/// carry the key in a header, which `restore` takes away.
pub(crate) fn is_script(request: &Request<Vec<u8>>) -> bool {
    request.headers().contains_key(KEY_HEADER)
}

/// Gives `request` the body its page stored for it, if any.
pub(crate) fn restore<R: Runtime>(app: &impl Manager<R>, request: &mut Request<Vec<u8>>) {
    let key = match request.headers_mut().remove(KEY_HEADER) {
        Some(key) => key.to_str().unwrap_or_default().to_owned(),
        // A form: the key is in the URL.
        None => match take_query_key(request) {
            Some(key) => key,
            None => return,
        },
    };

    let Some(body) = lock(&app.state::<Bodies>().0).remove(&key) else {
        eprintln!(
            "[android] No body for {} {}.",
            request.method(),
            request.uri()
        );
        return;
    };

    if let Ok(content_type) = HeaderValue::from_str(&body.content_type)
        && !body.content_type.is_empty()
    {
        request.headers_mut().insert(CONTENT_TYPE, content_type);
    }
    *request.body_mut() = body.bytes;
}

/// Removes the `nativella-body` parameter the page added to the URL of a
/// form, and returns its value.
fn take_query_key(request: &mut Request<Vec<u8>>) -> Option<String> {
    let query = request.uri().query()?;
    let mut key = None;
    let rest: Vec<&str> = query
        .split('&')
        .filter(|pair| match pair.strip_prefix(QUERY_KEY) {
            Some(value) => {
                key = Some(value.to_owned());
                false
            }
            None => true,
        })
        .collect();
    let key = key?;

    let path = request.uri().path();
    let path_and_query = if rest.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{}", rest.join("&"))
    };
    let mut parts = request.uri().clone().into_parts();
    parts.path_and_query = path_and_query.parse().ok();
    if let Ok(uri) = tauri::http::Uri::from_parts(parts) {
        *request.uri_mut() = uri;
    }

    Some(key)
}

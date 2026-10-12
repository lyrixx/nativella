//! `clipboard.*`: the system clipboard, as text, HTML or a PNG image.

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::image::Image;
use tauri_plugin_clipboard_manager::ClipboardExt;

use super::{Call, failed};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WriteText {
    text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WriteImage {
    /// Base64, JSON carries no bytes.
    png: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WriteHtml {
    html: String,
    alt_text: Option<String>,
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, String>> {
    let clipboard = call.app.clipboard();

    Some(match method {
        // An empty clipboard, or one holding an image, has no text.
        "readText" => Ok(json!({ "text": clipboard.read_text().ok() })),
        "writeText" => call.payload::<WriteText>().and_then(|payload| {
            clipboard.write_text(payload.text).map_err(failed)?;
            Ok(json!({}))
        }),
        "writeHtml" => call.payload::<WriteHtml>().and_then(|payload| {
            clipboard
                .write_html(payload.html, payload.alt_text)
                .map_err(failed)?;
            Ok(json!({}))
        }),
        // A clipboard without an image is no failure.
        "readImage" => clipboard
            .read_image()
            .ok()
            .map(|image| png(&image))
            .transpose()
            .map(|png| json!({ "png": png })),
        "writeImage" => call.payload::<WriteImage>().and_then(|payload| {
            let bytes = BASE64_STANDARD
                .decode(payload.png)
                .map_err(|error| format!("The image is not base64: {error}."))?;
            let image = Image::from_bytes(&bytes)
                .map_err(|error| format!("The image is not a PNG: {error}."))?;
            clipboard.write_image(&image).map_err(failed)?;
            Ok(json!({}))
        }),
        "clear" => clipboard.clear().map(|()| json!({})).map_err(failed),
        _ => return None,
    })
}

/// The image as a base64-encoded PNG.
fn png(image: &Image<'_>) -> Result<String, String> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, image.width(), image.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(image.rgba()))
        .map_err(|error| format!("The image cannot be encoded as PNG: {error}."))?;

    Ok(BASE64_STANDARD.encode(bytes))
}

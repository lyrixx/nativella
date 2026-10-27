//! The NFC tags tauri-plugin-nfc reads, as PHP gets them: see native/nfc.rs.
//! Apart, so that the tests run on computers too.

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde::Deserialize;
use serde_json::{Value, json};

/// A tag as the classes answer it: bytes as arrays of numbers, signed on
/// Android, from 0 to 255 on iOS.
#[derive(Debug, Deserialize)]
pub(crate) struct Tag {
    #[serde(default)]
    id: Option<Vec<i16>>,
    /// The technologies of the tag: Android's class names, iOS's families.
    #[serde(default)]
    kind: Vec<String>,
    #[serde(default)]
    records: Vec<Record>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Record {
    tnf: u8,
    #[serde(default)]
    kind: Vec<i16>,
    #[serde(default)]
    id: Vec<i16>,
    #[serde(default)]
    payload: Vec<i16>,
}

/// The tag for PHP: its identifier in hexadecimal, its records with their
/// bytes in base64.
pub(crate) fn tag_json(tag: &Tag) -> Value {
    let records: Vec<Value> = tag
        .records
        .iter()
        .map(|record| {
            json!({
                "tnf": record.tnf,
                "type": BASE64_STANDARD.encode(bytes(&record.kind)),
                "id": BASE64_STANDARD.encode(bytes(&record.id)),
                "payload": BASE64_STANDARD.encode(bytes(&record.payload)),
            })
        })
        .collect();

    json!({
        "id": tag.id.as_deref().map(|id| {
            bytes(id).iter().map(|byte| format!("{byte:02X}")).collect::<String>()
        }),
        "technologies": tag.kind,
        "records": records,
    })
}

/// Bytes, from Android's signed ones or iOS's unsigned ones.
fn bytes(numbers: &[i16]) -> Vec<u8> {
    // -1 is 255, as Java's bytes go.
    numbers.iter().map(|&number| number as u8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_of_android() {
        let tag: Tag = serde_json::from_value(json!({
            "id": [4, -95, 2, -1],
            "kind": ["android.nfc.tech.NfcA", "android.nfc.tech.Ndef"],
            "records": [{ "tnf": 1, "kind": [85], "id": [], "payload": [4, 110, 111] }],
        }))
        .unwrap();

        assert_eq!(
            tag_json(&tag),
            json!({
                "id": "04A102FF",
                "technologies": ["android.nfc.tech.NfcA", "android.nfc.tech.Ndef"],
                "records": [{ "tnf": 1, "type": "VQ==", "id": "", "payload": "BG5v" }],
            })
        );
    }

    #[test]
    fn a_tag_of_ios_may_have_no_identifier() {
        let tag: Tag = serde_json::from_value(json!({
            "id": null,
            "kind": ["FeliCa"],
            "records": [{ "tnf": 1, "kind": [84], "id": [], "payload": [2, 101, 110, 255] }],
        }))
        .unwrap();

        assert_eq!(tag_json(&tag)["id"], Value::Null);
        assert_eq!(tag_json(&tag)["records"][0]["payload"], "AmVu/w==");
    }
}

//! The entitlements of a signed app, in its signature: as an XML property
//! list, and as DER, which iOS reads since iOS 15.
//!
//! The DER form, as codesign writes it:
//!
//! ```text
//! [APPLICATION 16] { INTEGER 1, <dictionary> }
//! dictionary = [CONTEXT 16] { SEQUENCE { UTF8String <key>, <value> }… }, sorted by key
//! array      = SEQUENCE { <value>… }
//! value      = BOOLEAN | INTEGER | UTF8String | dictionary | array
//! ```

use std::io;

use plist::{Dictionary, Value};

const BOOLEAN: u8 = 0x01;
const INTEGER: u8 = 0x02;
const UTF8_STRING: u8 = 0x0c;
const SEQUENCE: u8 = 0x30;
/// Context-specific, constructed, 16.
const DICTIONARY: u8 = 0xb0;
/// Application, constructed, 16.
const ENTITLEMENTS: u8 = 0x70;

/// The entitlements as an XML property list.
pub fn xml(entitlements: &Dictionary) -> io::Result<Vec<u8>> {
    let mut xml = Vec::new();
    plist::to_writer_xml(&mut xml, entitlements)
        .map_err(|error| io::Error::other(format!("Cannot write the entitlements: {error}.")))?;

    Ok(xml)
}

/// The entitlements as DER.
pub fn der(entitlements: &Dictionary) -> io::Result<Vec<u8>> {
    let mut content = tlv(INTEGER, &[1]);
    content.extend(dictionary(entitlements)?);

    Ok(tlv(ENTITLEMENTS, &content))
}

fn dictionary(dictionary: &Dictionary) -> io::Result<Vec<u8>> {
    let mut entries: Vec<_> = dictionary.iter().collect();
    entries.sort_by(|(a, _), (b, _)| a.as_bytes().cmp(b.as_bytes()));

    let mut content = Vec::new();
    for (key, value) in entries {
        let mut entry = tlv(UTF8_STRING, key.as_bytes());
        entry.extend(value_of(key, value)?);
        content.extend(tlv(SEQUENCE, &entry));
    }

    Ok(tlv(DICTIONARY, &content))
}

fn value_of(key: &str, value: &Value) -> io::Result<Vec<u8>> {
    Ok(match value {
        Value::Boolean(value) => tlv(BOOLEAN, &[if *value { 0xff } else { 0 }]),
        Value::Integer(integer) => {
            let integer = integer
                .as_signed()
                .ok_or_else(|| unsupported(key, "an integer greater than a signed 64-bit one"))?;
            tlv(INTEGER, &integer_bytes(integer))
        }
        Value::String(string) => tlv(UTF8_STRING, string.as_bytes()),
        Value::Array(array) => {
            let mut content = Vec::new();
            for value in array {
                content.extend(value_of(key, value)?);
            }
            tlv(SEQUENCE, &content)
        }
        Value::Dictionary(nested) => dictionary(nested)?,
        _ => return Err(unsupported(key, "data, a date or a real number")),
    })
}

/// The shortest two's complement of `integer`, big-endian, as DER wants.
fn integer_bytes(integer: i64) -> Vec<u8> {
    let bytes = integer.to_be_bytes();
    let mut start = 0;
    // A leading byte is redundant when it only extends the sign of the next.
    while start < bytes.len() - 1
        && ((bytes[start] == 0 && bytes[start + 1] & 0x80 == 0)
            || (bytes[start] == 0xff && bytes[start + 1] & 0x80 != 0))
    {
        start += 1;
    }

    bytes[start..].to_vec()
}

/// A DER element: its tag, its length, its content.
fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut element = vec![tag];
    let length = content.len();
    if length < 0x80 {
        element.push(length as u8);
    } else {
        let bytes = length.to_be_bytes();
        let bytes = &bytes[bytes
            .iter()
            .position(|&byte| byte != 0)
            .unwrap_or(bytes.len() - 1)..];
        element.push(0x80 | bytes.len() as u8);
        element.extend(bytes);
    }
    element.extend(content);

    element
}

fn unsupported(key: &str, what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("The entitlement {key} of the profile holds {what}, which an app's cannot."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entitlements(entries: &[(&str, Value)]) -> Dictionary {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.clone()))
            .collect()
    }

    /// The DER of codesign, as the signatures of Xcode hold it: keys in
    /// order, whatever the order of the property list.
    #[test]
    fn as_codesign_encodes_them() {
        let der = der(&entitlements(&[
            ("get-task-allow", true.into()),
            ("application-identifier", "TEAM123456.com.acme.notes".into()),
            ("com.apple.developer.team-identifier", "TEAM123456".into()),
        ]))
        .unwrap();

        let mut expected = vec![0x70, 0x81, 0x82, 0x02, 0x01, 0x01, 0xb0, 0x7d];
        expected.extend([0x30, 0x33, 0x0c, 0x16]);
        expected.extend(b"application-identifier");
        expected.extend([0x0c, 0x19]);
        expected.extend(b"TEAM123456.com.acme.notes");
        expected.extend([0x30, 0x31, 0x0c, 0x23]);
        expected.extend(b"com.apple.developer.team-identifier");
        expected.extend([0x0c, 0x0a]);
        expected.extend(b"TEAM123456");
        expected.extend([0x30, 0x13, 0x0c, 0x0e]);
        expected.extend(b"get-task-allow");
        expected.extend([0x01, 0x01, 0xff]);
        assert_eq!(der, expected);
    }

    #[test]
    fn values_of_every_type() {
        let encoded = der(&entitlements(&[
            ("a", vec![Value::from(false), "x".into()].into()),
            ("b", Value::Dictionary(entitlements(&[("c", 128.into())]))),
            ("d", (-1).into()),
        ]))
        .unwrap();

        assert_eq!(
            encoded,
            [
                0x70, 0x2a, 0x02, 0x01, 0x01, 0xb0, 0x25, //
                0x30, 0x0b, 0x0c, 0x01, b'a', 0x30, 0x06, 0x01, 0x01, 0x00, 0x0c, 0x01,
                b'x', //
                0x30, 0x0e, 0x0c, 0x01, b'b', 0xb0, 0x09, 0x30, 0x07, 0x0c, 0x01, b'c', 0x02, 0x02,
                0x00, 0x80, //
                0x30, 0x06, 0x0c, 0x01, b'd', 0x02, 0x01, 0xff,
            ]
        );
        assert!(der(&entitlements(&[("e", Value::Data(vec![1]))])).is_err());
    }

    #[test]
    fn integers_and_lengths() {
        assert_eq!(integer_bytes(0), [0]);
        assert_eq!(integer_bytes(127), [0x7f]);
        assert_eq!(integer_bytes(128), [0, 0x80]);
        assert_eq!(integer_bytes(-128), [0x80]);
        assert_eq!(integer_bytes(-129), [0xff, 0x7f]);
        assert_eq!(tlv(0x04, &[0; 200])[..3], [0x04, 0x81, 200]);
        assert_eq!(tlv(0x04, &[0; 300])[..4], [0x04, 0x82, 0x01, 0x2c]);
    }
}

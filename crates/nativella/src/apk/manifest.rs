//! The AndroidManifest.xml of an APK, in the binary XML aapt2 compiles it to:
//! the manifest of the template takes the identity of the application.
//!
//! ```text
//! [header][string pool][resource map][nodes: namespaces, elements, ends]
//! ```
//!
//! Attributes point into the string pool; the resource map gives the first
//! strings, the attribute names, their ID. The strings of the application
//! are added at the end of the pool: no index moves.

use std::io;

use super::string_pool::{self, STRING_POOL, StringPool};

const XML: u16 = 0x0003;
const RESOURCE_MAP: u16 = 0x0180;
const START_ELEMENT: u16 = 0x0102;
const END_ELEMENT: u16 = 0x0103;

/// The attributes of the android namespace this module changes, by ID.
const LABEL: u32 = 0x0101_0001;
const NAME: u32 = 0x0101_0003;
const AUTHORITIES: u32 = 0x0101_0018;
const VERSION_CODE: u32 = 0x0101_021b;
const VERSION_NAME: u32 = 0x0101_021c;

/// The types of attribute values this module writes.
const TYPE_STRING: u8 = 0x03;
const TYPE_INT_DEC: u8 = 0x10;

/// The index of no string.
const NO_STRING: u32 = u32::MAX;

/// The activity of the template that the launcher opens: Tauri names it so,
/// in the package of the template.
const MAIN_ACTIVITY: &str = "MainActivity";

/// What makes the template an application.
pub struct Identity<'a> {
    /// The application ID: what Android knows the application by, the
    /// package of its manifest.
    pub application_id: &'a str,
    /// The name the launcher shows.
    pub label: &'a str,
    pub version_code: u32,
    pub version_name: &'a str,
    /// The permissions to leave out: those of the phone APIs the application
    /// does not ask for.
    pub removed_permissions: &'a [&'a str],
}

/// The manifest `xml` of the template, for the application `identity`.
pub fn rewrite(xml: &[u8], identity: &Identity) -> io::Result<Vec<u8>> {
    let mut document = Document::parse(xml)?;
    let chunks = std::mem::take(&mut document.chunks);
    let mut kept = Vec::with_capacity(chunks.len());
    // The application ID of the template, the package of its manifest
    // element, the first one.
    let mut template = None;
    // The depth of the element being left out, with its children.
    let mut removing = 0_usize;

    for mut chunk in chunks {
        let kind = u16_at(&chunk, 0);
        if removing > 0 {
            match kind {
                START_ELEMENT => removing += 1,
                END_ELEMENT => removing -= 1,
                _ => {}
            }
            continue;
        }

        if kind == START_ELEMENT {
            let (name, attributes) = element(&chunk)?;
            let element = document.string(name)?.to_owned();

            match element.as_str() {
                "manifest" => {
                    for attribute in &attributes {
                        match document.attribute_id(attribute.name) {
                            Some(VERSION_CODE) => {
                                set_integer(&mut chunk, attribute, identity.version_code)
                            }
                            Some(VERSION_NAME) => {
                                let value = document.add_string(identity.version_name)?;
                                set_string(&mut chunk, attribute, value);
                            }
                            // The one attribute of no namespace: the package.
                            None if attribute.namespace == NO_STRING
                                && document.string(attribute.name).ok() == Some("package") =>
                            {
                                template = Some(document.string(attribute.raw_value)?.to_owned());
                                let value = document.add_string(identity.application_id)?;
                                set_string(&mut chunk, attribute, value);
                            }
                            _ => {}
                        }
                    }
                }
                "uses-permission" | "permission" => {
                    let template = template.as_deref().ok_or_else(no_package)?;
                    if let Some(name) = attributes
                        .iter()
                        .find(|attribute| document.attribute_id(attribute.name) == Some(NAME))
                    {
                        let permission = document.string(name.raw_value)?.to_owned();
                        if element == "uses-permission"
                            && identity.removed_permissions.contains(&permission.as_str())
                        {
                            removing = 1;
                            continue;
                        }
                        // ${applicationId}.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION:
                        // two applications cannot both define one.
                        if let Some(renamed) =
                            renamed(&permission, template, identity.application_id)
                        {
                            let value = document.add_string(&renamed)?;
                            set_string(&mut chunk, name, value);
                        }
                    }
                }
                "provider" => {
                    let template = template.as_deref().ok_or_else(no_package)?;
                    // ${applicationId}.fileprovider and the like: two
                    // applications cannot both have one.
                    if let Some(authorities) = attributes.iter().find(|attribute| {
                        document.attribute_id(attribute.name) == Some(AUTHORITIES)
                    }) {
                        let value = document
                            .string(authorities.raw_value)?
                            .split(';')
                            .map(|authority| {
                                renamed(authority, template, identity.application_id)
                                    .unwrap_or_else(|| authority.to_owned())
                            })
                            .collect::<Vec<_>>()
                            .join(";");
                        let value = document.add_string(&value)?;
                        set_string(&mut chunk, authorities, value);
                    }
                }
                "application" | "activity" => {
                    let template = template.as_deref().ok_or_else(no_package)?;
                    let main_activity = format!("{template}.{MAIN_ACTIVITY}");
                    let is_main = attributes.iter().any(|attribute| {
                        document.attribute_id(attribute.name) == Some(NAME)
                            && document.string(attribute.raw_value).ok()
                                == Some(main_activity.as_str())
                    });
                    if element == "application" || is_main {
                        for attribute in &attributes {
                            if document.attribute_id(attribute.name) == Some(LABEL) {
                                let value = document.add_string(identity.label)?;
                                set_string(&mut chunk, attribute, value);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        kept.push(chunk);
    }

    if template.is_none() {
        return Err(no_package());
    }
    document.chunks = kept;

    document.write()
}

/// `value` for the application ID `application_id` instead of `template`:
/// the application ID itself, or a name under it.
fn renamed(value: &str, template: &str, application_id: &str) -> Option<String> {
    if value == template {
        return Some(application_id.to_owned());
    }

    value
        .strip_prefix(template)
        .filter(|rest| rest.starts_with('.'))
        .map(|rest| format!("{application_id}{rest}"))
}

/// The version name of the manifest `xml`: for the template, the version of
/// Nativella it was built with.
pub fn version_name(xml: &[u8]) -> io::Result<Option<String>> {
    let document = Document::parse(xml)?;
    let Some(manifest) = document
        .chunks
        .iter()
        .find(|chunk| u16_at(chunk, 0) == START_ELEMENT)
    else {
        return Ok(None);
    };
    let (_, attributes) = element(manifest)?;

    attributes
        .iter()
        .find(|attribute| document.attribute_id(attribute.name) == Some(VERSION_NAME))
        .map(|attribute| document.string(attribute.raw_value).map(str::to_owned))
        .transpose()
}

struct Document {
    strings: Vec<String>,
    utf8: bool,
    /// The attribute ID of the first strings.
    resource_ids: Vec<u32>,
    /// The chunks after the string pool, as they are: the resource map, then
    /// the nodes.
    chunks: Vec<Vec<u8>>,
}

impl Document {
    fn parse(xml: &[u8]) -> io::Result<Self> {
        if xml.len() < 8 || u16_at(xml, 0) != XML || u32_at(xml, 4) as usize != xml.len() {
            return Err(invalid("The manifest of the template is not binary XML."));
        }

        let mut strings = None;
        let mut resource_ids = Vec::new();
        let mut chunks = Vec::new();
        let mut offset = usize::from(u16_at(xml, 2));
        while offset < xml.len() {
            let size = xml
                .get(offset + 4..offset + 8)
                .map(|size| u32::from_le_bytes(size.try_into().expect("4 bytes")) as usize)
                .filter(|&size| size >= 8)
                .ok_or_else(corrupted)?;
            let chunk = xml.get(offset..offset + size).ok_or_else(corrupted)?;

            match u16_at(chunk, 0) {
                STRING_POOL if strings.is_none() => strings = Some(parse_strings(chunk)?),
                kind => {
                    if kind == RESOURCE_MAP {
                        resource_ids = chunk[8..]
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .map(|id| u32::from_le_bytes(*id))
                            .collect();
                    }
                    chunks.push(chunk.to_vec());
                }
            }
            offset += size;
        }

        let (strings, utf8) = strings.ok_or_else(corrupted)?;

        Ok(Self {
            strings,
            utf8,
            resource_ids,
            chunks,
        })
    }

    fn string(&self, index: u32) -> io::Result<&str> {
        self.strings
            .get(index as usize)
            .map(String::as_str)
            .ok_or_else(corrupted)
    }

    fn attribute_id(&self, name: u32) -> Option<u32> {
        self.resource_ids.get(name as usize).copied()
    }

    /// The index of `value`, added at the end of the pool if need be.
    fn add_string(&mut self, value: &str) -> io::Result<u32> {
        if value.encode_utf16().count() > 0x7fff {
            return Err(invalid(&format!("\"{value}\" is too long for a manifest.")));
        }

        // Not among the attribute names, which the resource map gives an ID.
        let index = match self.strings[self.resource_ids.len()..]
            .iter()
            .position(|string| string == value)
        {
            Some(position) => self.resource_ids.len() + position,
            None => {
                self.strings.push(value.to_owned());
                self.strings.len() - 1
            }
        };

        u32::try_from(index).map_err(|_| corrupted())
    }

    fn write(&self) -> io::Result<Vec<u8>> {
        let pool = string_pool::write(&self.strings, self.utf8)?;

        let length = 8 + pool.len() + self.chunks.iter().map(Vec::len).sum::<usize>();
        let mut xml = Vec::with_capacity(length);
        xml.extend(XML.to_le_bytes());
        xml.extend(8_u16.to_le_bytes());
        xml.extend(chunk_size(length)?.to_le_bytes());
        xml.extend(pool);
        for chunk in &self.chunks {
            xml.extend(chunk);
        }

        Ok(xml)
    }
}

/// The strings of a pool, and whether they are in UTF-8 rather than UTF-16.
fn parse_strings(chunk: &[u8]) -> io::Result<(Vec<String>, bool)> {
    let pool = StringPool::parse(chunk).ok_or_else(corrupted)?;
    if pool.styled {
        return Err(invalid("The manifest of the template has styled strings."));
    }
    let strings = (0..pool.count)
        .map(|index| pool.get(index).ok_or_else(corrupted))
        .collect::<io::Result<_>>()?;

    Ok((strings, pool.utf8))
}

/// An attribute of a start element: where it is in the chunk, and what it
/// points to.
struct Attribute {
    offset: usize,
    namespace: u32,
    name: u32,
    raw_value: u32,
}

/// The name of a start element, and its attributes.
fn element(chunk: &[u8]) -> io::Result<(u32, Vec<Attribute>)> {
    // The node header, then the element: namespace, name, then where its
    // attributes start (from the element), their size and count.
    let header_length = usize::from(u16_at(chunk, 2));
    let element = chunk
        .get(header_length..header_length + 20)
        .ok_or_else(corrupted)?;
    let name = u32_at(element, 4);
    let start = header_length + usize::from(u16_at(element, 8));
    let size = usize::from(u16_at(element, 10));
    let count = usize::from(u16_at(element, 12));
    if size < 20 || chunk.len() < start + size * count {
        return Err(corrupted());
    }

    let attributes = (0..count)
        .map(|index| {
            let offset = start + index * size;
            Attribute {
                offset,
                namespace: u32_at(chunk, offset),
                name: u32_at(chunk, offset + 4),
                raw_value: u32_at(chunk, offset + 8),
            }
        })
        .collect();

    Ok((name, attributes))
}

/// The raw value, then the typed one: its size, a zero, its type, its data.
fn set_value(chunk: &mut [u8], attribute: &Attribute, raw_value: u32, kind: u8, data: u32) {
    let offset = attribute.offset;
    chunk[offset + 8..offset + 12].copy_from_slice(&raw_value.to_le_bytes());
    chunk[offset + 12..offset + 14].copy_from_slice(&8_u16.to_le_bytes());
    chunk[offset + 14] = 0;
    chunk[offset + 15] = kind;
    chunk[offset + 16..offset + 20].copy_from_slice(&data.to_le_bytes());
}

fn set_string(chunk: &mut [u8], attribute: &Attribute, string: u32) {
    set_value(chunk, attribute, string, TYPE_STRING, string);
}

fn set_integer(chunk: &mut [u8], attribute: &Attribute, value: u32) {
    set_value(chunk, attribute, NO_STRING, TYPE_INT_DEC, value);
}

fn chunk_size(length: usize) -> io::Result<u32> {
    u32::try_from(length).map_err(|_| invalid("The manifest is too big."))
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("A 4-byte slice."),
    )
}

fn no_package() -> io::Error {
    invalid("The manifest of the template has no package.")
}

fn corrupted() -> io::Error {
    invalid("The manifest of the template is corrupted.")
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    const ANDROID: u32 = 5;

    /// A start element: its name, then its attributes (namespace, name, raw
    /// value, type, data).
    fn start(name: u32, attributes: &[(u32, u32, u32, u8, u32)]) -> Vec<u8> {
        let mut chunk = Vec::new();
        chunk.extend(START_ELEMENT.to_le_bytes());
        chunk.extend(16_u16.to_le_bytes());
        chunk.extend((36 + 20 * attributes.len() as u32).to_le_bytes());
        chunk.extend(1_u32.to_le_bytes());
        chunk.extend(NO_STRING.to_le_bytes());
        chunk.extend(NO_STRING.to_le_bytes());
        chunk.extend(name.to_le_bytes());
        chunk.extend(20_u16.to_le_bytes());
        chunk.extend(20_u16.to_le_bytes());
        chunk.extend((attributes.len() as u16).to_le_bytes());
        chunk.extend([0; 6]);
        for &(namespace, name, raw_value, kind, data) in attributes {
            chunk.extend(namespace.to_le_bytes());
            chunk.extend(name.to_le_bytes());
            chunk.extend(raw_value.to_le_bytes());
            chunk.extend(8_u16.to_le_bytes());
            chunk.extend([0, kind]);
            chunk.extend(data.to_le_bytes());
        }
        chunk
    }

    fn end(name: u32) -> Vec<u8> {
        let mut chunk = Vec::new();
        chunk.extend(END_ELEMENT.to_le_bytes());
        chunk.extend(16_u16.to_le_bytes());
        chunk.extend(24_u32.to_le_bytes());
        chunk.extend(1_u32.to_le_bytes());
        chunk.extend(NO_STRING.to_le_bytes());
        chunk.extend(NO_STRING.to_le_bytes());
        chunk.extend(name.to_le_bytes());
        chunk
    }

    /// What aapt2 writes for the manifest of the template, in short.
    pub(in crate::apk) fn template(utf8: bool) -> Vec<u8> {
        let strings = [
            "versionCode",
            "versionName",
            "name",
            "label",
            "authorities",
            "http://schemas.android.com/apk/res/android",
            "package",
            "manifest",
            "dev.nativella.runtime",
            "0.1.1",
            "uses-permission",
            "android.permission.INTERNET",
            "android.permission.VIBRATE",
            "permission",
            "dev.nativella.runtime.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION",
            "application",
            "activity",
            "dev.nativella.runtime.MainActivity",
            "dev.nativella.runtime.OtherActivity",
            "provider",
            "dev.nativella.runtime.fileprovider;dev.nativella.runtimes",
        ];
        let ids = [VERSION_CODE, VERSION_NAME, NAME, LABEL, AUTHORITIES];
        let mut resource_map = Vec::new();
        resource_map.extend(RESOURCE_MAP.to_le_bytes());
        resource_map.extend(8_u16.to_le_bytes());
        resource_map.extend((8 + 4 * ids.len() as u32).to_le_bytes());
        for id in ids {
            resource_map.extend(id.to_le_bytes());
        }

        let string = |index: u32| (ANDROID, 2, index, TYPE_STRING, index);
        let reference = (ANDROID, 3, NO_STRING, 0x01, 0x7f0f_001c);
        let chunks = vec![
            resource_map,
            start(
                7,
                &[
                    (ANDROID, 0, NO_STRING, TYPE_INT_DEC, 1001),
                    (ANDROID, 1, 9, TYPE_STRING, 9),
                    (NO_STRING, 6, 8, TYPE_STRING, 8),
                ],
            ),
            start(10, &[string(11)]),
            end(10),
            start(10, &[string(12)]),
            end(10),
            start(13, &[string(14)]),
            end(13),
            start(10, &[string(14)]),
            end(10),
            start(15, &[reference]),
            start(16, &[reference, string(17)]),
            end(16),
            start(16, &[reference, string(18)]),
            end(16),
            start(19, &[(ANDROID, 4, 20, TYPE_STRING, 20)]),
            end(19),
            end(15),
            end(7),
        ];

        Document {
            strings: strings.map(str::to_owned).to_vec(),
            utf8,
            resource_ids: ids.to_vec(),
            chunks,
        }
        .write()
        .unwrap()
    }

    /// Each start element: its name, and its attributes by name, as their
    /// string, or their data.
    pub(in crate::apk) fn elements(xml: &[u8]) -> Vec<(String, Vec<(String, String)>)> {
        let document = Document::parse(xml).unwrap();

        document
            .chunks
            .iter()
            .filter(|chunk| u16_at(chunk, 0) == START_ELEMENT)
            .map(|chunk| {
                let (name, attributes) = element(chunk).unwrap();
                let attributes = attributes
                    .iter()
                    .map(|attribute| {
                        let value = match chunk[attribute.offset + 15] {
                            TYPE_STRING => document
                                .string(u32_at(chunk, attribute.offset + 16))
                                .unwrap()
                                .to_owned(),
                            _ => format!("@{}", u32_at(chunk, attribute.offset + 16)),
                        };
                        (document.string(attribute.name).unwrap().to_owned(), value)
                    })
                    .collect();
                (document.string(name).unwrap().to_owned(), attributes)
            })
            .collect()
    }

    #[test]
    fn the_template_takes_the_identity_of_the_application() {
        for utf8 in [false, true] {
            let identity = Identity {
                application_id: "com.acme.notes",
                label: "Acme Notes",
                version_code: 2_003_004,
                version_name: "2.3.4",
                removed_permissions: &["android.permission.VIBRATE"],
            };
            let xml = rewrite(&template(utf8), &identity).unwrap();

            let pairs = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
                pairs
                    .iter()
                    .map(|&(name, value)| (name.to_owned(), value.to_owned()))
                    .collect()
            };
            assert_eq!(
                elements(&xml),
                [
                    (
                        "manifest".to_owned(),
                        pairs(&[
                            ("versionCode", "@2003004"),
                            ("versionName", "2.3.4"),
                            ("package", "com.acme.notes")
                        ])
                    ),
                    (
                        "uses-permission".to_owned(),
                        pairs(&[("name", "android.permission.INTERNET")])
                    ),
                    (
                        "permission".to_owned(),
                        pairs(&[(
                            "name",
                            "com.acme.notes.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION"
                        )])
                    ),
                    (
                        "uses-permission".to_owned(),
                        pairs(&[(
                            "name",
                            "com.acme.notes.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION"
                        )])
                    ),
                    ("application".to_owned(), pairs(&[("label", "Acme Notes")])),
                    (
                        "activity".to_owned(),
                        pairs(&[
                            ("label", "Acme Notes"),
                            ("name", "dev.nativella.runtime.MainActivity")
                        ])
                    ),
                    (
                        "activity".to_owned(),
                        pairs(&[
                            ("label", "@2131689500"),
                            ("name", "dev.nativella.runtime.OtherActivity")
                        ])
                    ),
                    (
                        "provider".to_owned(),
                        pairs(&[(
                            "authorities",
                            "com.acme.notes.fileprovider;dev.nativella.runtimes"
                        )])
                    ),
                ],
                "UTF-8: {utf8}"
            );
        }
    }

    #[test]
    fn a_document_round_trips() {
        let xml = template(false);

        assert_eq!(Document::parse(&xml).unwrap().write().unwrap(), xml);
    }

    #[test]
    fn rejects_what_is_not_binary_xml() {
        let identity = Identity {
            application_id: "com.acme.notes",
            label: "Acme Notes",
            version_code: 1,
            version_name: "0.0.1",
            removed_permissions: &[],
        };

        for xml in [&b"<manifest/>"[..], &[], &template(false)[..100]] {
            assert_eq!(
                rewrite(xml, &identity).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
        }
    }
}

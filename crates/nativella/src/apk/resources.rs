//! The resource table of an APK, `resources.arsc`: where the file of a
//! resource is, for each configuration, such as each screen density.
//!
//! ```text
//! [header][string pool of the values][package: [header][type names][entry names][types…]]
//! ```
//!
//! A resource ID is 0xPPTTEEEE: its package, its type, its entry. Each type
//! chunk holds the entries of one type for one configuration; the value of a
//! file entry is its path in the APK, in the string pool. The path is no
//! guess: a release build shortens them, `res/9w.png` for
//! `res/mipmap-mdpi-v4/ic_launcher.png`.

use std::io;

use super::string_pool::{STRING_POOL, StringPool};

const TABLE: u16 = 0x0002;
const PACKAGE: u16 = 0x0200;
const TYPE: u16 = 0x0201;

/// The flags of a type chunk: its offsets are pairs of an entry index and an
/// offset, for the entries it has only; or 16-bit, in units of 4 bytes.
const SPARSE: u8 = 0x01;
const OFFSET16: u8 = 0x02;

/// The flags of an entry: a bag of values (a style…), not one value; or a
/// compact entry, its value in its own 8 bytes.
const COMPLEX: u16 = 0x0001;
const COMPACT: u16 = 0x0008;

/// The types of values: none, and a string, which a file is, its path.
const TYPE_NULL: u8 = 0x00;
const TYPE_STRING: u8 = 0x03;

/// Where the type chunk describes its configuration, and where the screen
/// density is in it, after its size, its mobile network, its locale, its
/// orientation and its touchscreen.
const CONFIG: usize = 20;
const CONFIG_DENSITY: usize = 14;

/// A file of a resource.
#[derive(Debug, PartialEq, Eq)]
pub struct File {
    /// The path of the file in the APK.
    pub path: String,
    /// The screen density of its configuration, in dots per inch: 160 for
    /// mdpi, 640 for xxxhdpi; 0 when its configuration has none.
    pub density: u16,
}

/// The files of the resource `id` in the resource table `table`, one per
/// configuration that has it, in the order of the table.
pub fn files(table: &[u8], id: u32) -> io::Result<Vec<File>> {
    if table.len() < 12 || u16_at(table, 0) != Some(TABLE) {
        return Err(corrupted());
    }
    let package_id = id >> 24;
    let type_id = (id >> 16) & 0xff;
    let entry = (id & 0xffff) as usize;

    let mut strings = None;
    let mut files = Vec::new();
    for chunk in chunks(table)? {
        let chunk = chunk?;
        match u16_at(chunk, 0) {
            Some(STRING_POOL) if strings.is_none() => {
                strings = Some(StringPool::parse(chunk).ok_or_else(corrupted)?);
            }
            Some(PACKAGE) if u32_at(chunk, 8) == Some(package_id) => {
                let strings = strings.as_ref().ok_or_else(corrupted)?;
                for chunk in chunks(chunk)? {
                    let chunk = chunk?;
                    if u16_at(chunk, 0) != Some(TYPE)
                        || chunk.get(8).map(|&kind| u32::from(kind)) != Some(type_id)
                    {
                        continue;
                    }
                    let Some(value) = value(chunk, entry)? else {
                        continue;
                    };
                    let path = match value {
                        (TYPE_STRING, index) => strings.get(index as usize),
                        _ => None,
                    }
                    .ok_or_else(|| {
                        invalid(&format!(
                            "The resource 0x{id:08x} of the template is not a file."
                        ))
                    })?;
                    let density = u16_at(chunk, CONFIG + CONFIG_DENSITY).ok_or_else(corrupted)?;

                    files.push(File { path, density });
                }
            }
            _ => {}
        }
    }

    Ok(files)
}

/// The chunks inside `parent`, after its header.
fn chunks(parent: &[u8]) -> io::Result<impl Iterator<Item = io::Result<&[u8]>>> {
    let mut offset = usize::from(u16_at(parent, 2).ok_or_else(corrupted)?);
    let parent = u32_at(parent, 4)
        .and_then(|length| parent.get(..length as usize))
        .ok_or_else(corrupted)?;

    Ok(std::iter::from_fn(move || {
        if offset >= parent.len() {
            return None;
        }
        let chunk = u32_at(parent, offset + 4)
            .map(|size| size as usize)
            .filter(|&size| size >= 8)
            .and_then(|size| parent.get(offset..offset + size))
            .ok_or_else(corrupted);
        // A corrupted chunk ends the iteration.
        offset = chunk
            .as_ref()
            .map_or(parent.len(), |chunk| offset + chunk.len());

        Some(chunk)
    }))
}

/// The value of the entry `entry` in the type chunk `chunk`, its type and its
/// data, if the configuration of the chunk has it.
fn value(chunk: &[u8], entry: usize) -> io::Result<Option<(u8, u32)>> {
    let header_length = usize::from(u16_at(chunk, 2).ok_or_else(corrupted)?);
    let flags = *chunk.get(9).ok_or_else(corrupted)?;
    let count = u32_at(chunk, 12).ok_or_else(corrupted)? as usize;
    let entries_start = u32_at(chunk, 16).ok_or_else(corrupted)? as usize;

    let offset = if flags & SPARSE != 0 {
        chunk
            .get(header_length..header_length + count * 4)
            .ok_or_else(corrupted)?
            .as_chunks::<4>()
            .0
            .iter()
            .find(|pair| usize::from(u16::from_le_bytes([pair[0], pair[1]])) == entry)
            .map(|pair| usize::from(u16::from_le_bytes([pair[2], pair[3]])) * 4)
    } else if entry >= count {
        None
    } else if flags & OFFSET16 != 0 {
        match u16_at(chunk, header_length + entry * 2).ok_or_else(corrupted)? {
            u16::MAX => None,
            offset => Some(usize::from(offset) * 4),
        }
    } else {
        match u32_at(chunk, header_length + entry * 4).ok_or_else(corrupted)? {
            u32::MAX => None,
            offset => Some(offset as usize),
        }
    };
    let Some(offset) = offset else {
        return Ok(None);
    };

    // An entry is its size, its flags and its name, then its value: the size
    // of the value, a zero, its type, its data.
    let entry = entries_start + offset;
    let entry_flags = u16_at(chunk, entry + 2).ok_or_else(corrupted)?;
    if entry_flags & COMPACT != 0 {
        let data = u32_at(chunk, entry + 4).ok_or_else(corrupted)?;
        return Ok(Some(((entry_flags >> 8) as u8, data)));
    }
    // A bag has no value of its own, and is no file.
    if entry_flags & COMPLEX != 0 {
        return Ok(Some((TYPE_NULL, 0)));
    }
    let value = entry + usize::from(u16_at(chunk, entry).ok_or_else(corrupted)?);
    let kind = *chunk.get(value + 3).ok_or_else(corrupted)?;
    let data = u32_at(chunk, value + 4).ok_or_else(corrupted)?;

    Ok(Some((kind, data)))
}

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    bytes
        .get(offset..offset + 2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|bytes| u32::from_le_bytes(bytes.try_into().expect("A 4-byte slice.")))
}

fn corrupted() -> io::Error {
    invalid("The resource table of the template is corrupted.")
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::apk::string_pool;

    const TYPE_INT_DEC: u8 = 0x10;

    /// A chunk: its type, the fields of its header after its sizes, then its
    /// body.
    fn chunk(kind: u16, header: &[u8], body: &[u8]) -> Vec<u8> {
        let header_length = 8 + header.len();
        let mut chunk = Vec::new();
        chunk.extend(kind.to_le_bytes());
        chunk.extend((header_length as u16).to_le_bytes());
        chunk.extend(((header_length + body.len()) as u32).to_le_bytes());
        chunk.extend(header);
        chunk.extend(body);
        chunk
    }

    /// How a type chunk gives the offsets of its entries.
    #[derive(Clone, Copy)]
    enum Offsets {
        Wide,
        Narrow,
        Sparse,
    }

    /// A type chunk of the type `kind` for the screen density `density`,
    /// whose entry `entry` is the value `value`, with its offsets written in
    /// one way or another, and the entry compact or not.
    fn type_chunk(
        kind: u8,
        density: u16,
        entry: u16,
        value: (u8, u32),
        offsets: Offsets,
        compact: bool,
    ) -> Vec<u8> {
        let mut entries = Vec::new();
        if compact {
            entries.extend(0_u16.to_le_bytes());
            entries.extend((COMPACT | (u16::from(value.0) << 8)).to_le_bytes());
            entries.extend(value.1.to_le_bytes());
        } else {
            entries.extend(8_u16.to_le_bytes());
            entries.extend(0_u16.to_le_bytes());
            entries.extend(0_u32.to_le_bytes());
            entries.extend(8_u16.to_le_bytes());
            entries.extend([0, value.0]);
            entries.extend(value.1.to_le_bytes());
        }

        // The entries before it are missing.
        let (flags, count, table) = match offsets {
            Offsets::Wide => {
                let mut table: Vec<u8> = (0..entry).flat_map(|_| u32::MAX.to_le_bytes()).collect();
                table.extend(0_u32.to_le_bytes());
                (0, u32::from(entry) + 1, table)
            }
            Offsets::Narrow => {
                let mut table: Vec<u8> = (0..entry).flat_map(|_| u16::MAX.to_le_bytes()).collect();
                table.extend(0_u16.to_le_bytes());
                (OFFSET16, u32::from(entry) + 1, table)
            }
            Offsets::Sparse => {
                // This entry, 8 bytes after the one that follows it.
                let mut table = Vec::new();
                table.extend(entry.to_le_bytes());
                table.extend(2_u16.to_le_bytes());
                table.extend((entry + 1).to_le_bytes());
                table.extend(0_u16.to_le_bytes());
                entries.splice(0..0, [0; 8]);
                (SPARSE, 2, table)
            }
        };

        let mut config = vec![0; 64];
        config[..4].copy_from_slice(&64_u32.to_le_bytes());
        config[CONFIG_DENSITY..CONFIG_DENSITY + 2].copy_from_slice(&density.to_le_bytes());
        let mut header = vec![kind, flags, 0, 0];
        header.extend(count.to_le_bytes());
        header.extend(((CONFIG + config.len() + table.len()) as u32).to_le_bytes());
        header.extend(config);

        chunk(TYPE, &header, &[table, entries].concat())
    }

    /// A resource table whose resource `id` is the file of each path, for
    /// its density, each in its own type chunk, written in each of the ways,
    /// among other resources.
    pub(in crate::apk) fn table(id: u32, files: &[(&str, u16)]) -> Vec<u8> {
        let strings: Vec<String> = ["res/other.png"]
            .into_iter()
            .chain(files.iter().map(|&(path, _)| path))
            .map(str::to_owned)
            .collect();
        let kind = (id >> 16) as u8;
        let entry = id as u16;
        let ways = [
            (Offsets::Wide, false),
            (Offsets::Narrow, false),
            (Offsets::Sparse, false),
            (Offsets::Wide, true),
        ];
        let other = (TYPE_STRING, 0);

        let mut types = string_pool::write(&["mipmap".to_owned()], false).unwrap();
        // Another type, with the same entry.
        types.extend(type_chunk(
            kind + 1,
            160,
            entry,
            other,
            Offsets::Wide,
            false,
        ));
        for (index, &(_, density)) in files.iter().enumerate() {
            let (offsets, compact) = ways[index % ways.len()];
            let value = (TYPE_STRING, index as u32 + 1);
            types.extend(type_chunk(kind, density, entry, value, offsets, compact));
        }
        let package = |id: u32, types: &[u8]| {
            let mut header = id.to_le_bytes().to_vec();
            header.resize(288 - 8, 0);
            chunk(PACKAGE, &header, types)
        };

        let mut body = string_pool::write(&strings, true).unwrap();
        // Another package first, with the same type and entry.
        body.extend(package(
            0x01,
            &type_chunk(kind, 160, entry, other, Offsets::Wide, false),
        ));
        body.extend(package(id >> 24, &types));

        chunk(TABLE, &2_u32.to_le_bytes(), &body)
    }

    #[test]
    fn the_files_of_a_resource() {
        let files = [
            ("res/mipmap-mdpi-v4/ic_launcher.png", 160),
            ("res/9w.png", 240),
            ("res/mipmap-xhdpi-v4/ic_launcher.png", 320),
            ("res/o-.png", 640),
            ("res/mipmap-anydpi-v26/ic_launcher.xml", 0xfffe),
        ];
        let table = table(0x7f0e_0003, &files);

        assert_eq!(
            super::files(&table, 0x7f0e_0003).unwrap(),
            files.map(|(path, density)| File {
                path: path.to_owned(),
                density
            })
        );
        assert!(super::files(&table, 0x7f0e_0002).unwrap().is_empty());
    }

    #[test]
    fn a_value_is_no_file() {
        let mut table = table(0x7f0e_0000, &[("res/a.png", 160)]);
        // The type of the value of the last entry, before its data.
        let position = table.len() - 5;
        table[position] = TYPE_INT_DEC;

        let error = files(&table, 0x7f0e_0000).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("0x7f0e0000"), "{error}");
    }

    #[test]
    fn a_corrupted_table_is_an_error_not_a_panic() {
        let table = table(0x7f0e_0000, &[("res/a.png", 160), ("res/b.png", 640)]);

        for length in 0..table.len() {
            files(&table[..length], 0x7f0e_0000).unwrap_err();
        }
        for position in 0..table.len() {
            let mut corrupted = table.clone();
            corrupted[position] ^= 0xff;
            let _ = files(&corrupted, 0x7f0e_0000);
        }
    }
}

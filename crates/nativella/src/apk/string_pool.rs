//! The string pool of the binary formats of Android, binary XML and the
//! resource table: a chunk of strings, in UTF-8 or in UTF-16, which the rest
//! of the file points into by index.
//!
//! ```text
//! [header][offsets of the strings][offsets of the styles][strings][styles]
//! ```

use std::io;

pub const STRING_POOL: u16 = 0x0001;
const HEADER_LENGTH: usize = 28;
const UTF8_FLAG: u32 = 1 << 8;

/// A string pool, read in place.
pub struct StringPool<'a> {
    chunk: &'a [u8],
    header_length: usize,
    strings_start: usize,
    pub count: usize,
    pub utf8: bool,
    /// Whether some strings have styles, spans of markup, which the pool
    /// keeps apart from them.
    pub styled: bool,
}

impl<'a> StringPool<'a> {
    /// The string pool `chunk`, or None if it is none.
    pub fn parse(chunk: &'a [u8]) -> Option<Self> {
        if chunk.len() < HEADER_LENGTH || u16_at(chunk, 0) != STRING_POOL {
            return None;
        }

        Some(Self {
            chunk,
            header_length: usize::from(u16_at(chunk, 2)),
            strings_start: u32_at(chunk, 20) as usize,
            count: u32_at(chunk, 8) as usize,
            utf8: u32_at(chunk, 16) & UTF8_FLAG != 0,
            styled: u32_at(chunk, 12) != 0,
        })
    }

    /// The string `index`, or None if the pool has none, or a corrupted one.
    pub fn get(&self, index: usize) -> Option<String> {
        if index >= self.count {
            return None;
        }
        let offset = self
            .chunk
            .get(self.header_length + index * 4..self.header_length + index * 4 + 4)
            .map(|offset| u32_at(offset, 0) as usize)?;
        let data = self.chunk.get(self.strings_start + offset..)?;

        if self.utf8 {
            // The length in UTF-16 units, then in bytes.
            let (_, skipped) = read_length8(data)?;
            let (length, start) = read_length8(&data[skipped..])?;
            let bytes = data.get(skipped + start..skipped + start + length)?;

            String::from_utf8(bytes.to_vec()).ok()
        } else {
            let (length, start) = read_length16(data)?;
            let units: Vec<u16> = data
                .get(start..start + length * 2)?
                .as_chunks::<2>()
                .0
                .iter()
                .map(|unit| u16::from_le_bytes(*unit))
                .collect();

            String::from_utf16(&units).ok()
        }
    }
}

/// The string pool of `strings`, without styles.
pub fn write(strings: &[String], utf8: bool) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut offsets = Vec::with_capacity(strings.len() * 4);
    for string in strings {
        offsets.extend(size(data.len())?.to_le_bytes());
        if utf8 {
            write_length8(&mut data, string.encode_utf16().count());
            write_length8(&mut data, string.len());
            data.extend(string.as_bytes());
            data.push(0);
        } else {
            let units: Vec<u16> = string.encode_utf16().collect();
            write_length16(&mut data, units.len());
            data.extend(units.iter().flat_map(|unit| unit.to_le_bytes()));
            data.extend([0, 0]);
        }
    }
    data.resize(data.len().next_multiple_of(4), 0);

    let strings_start = HEADER_LENGTH + offsets.len();
    let mut pool = Vec::with_capacity(strings_start + data.len());
    pool.extend(STRING_POOL.to_le_bytes());
    pool.extend((HEADER_LENGTH as u16).to_le_bytes());
    pool.extend(size(strings_start + data.len())?.to_le_bytes());
    pool.extend(size(strings.len())?.to_le_bytes());
    // No style.
    pool.extend(0_u32.to_le_bytes());
    pool.extend((if utf8 { UTF8_FLAG } else { 0 }).to_le_bytes());
    pool.extend(size(strings_start)?.to_le_bytes());
    pool.extend(0_u32.to_le_bytes());
    pool.extend(offsets);
    pool.extend(data);

    Ok(pool)
}

/// A length of a UTF-8 pool, and how many bytes it takes: one byte, or two
/// with the high bit set.
fn read_length8(data: &[u8]) -> Option<(usize, usize)> {
    match *data {
        [first, second, ..] if first & 0x80 != 0 => {
            Some(((usize::from(first & 0x7f) << 8) | usize::from(second), 2))
        }
        [first, ..] => Some((usize::from(first), 1)),
        [] => None,
    }
}

fn write_length8(data: &mut Vec<u8>, length: usize) {
    if length > 0x7f {
        data.extend([0x80 | (length >> 8) as u8, length as u8]);
    } else {
        data.push(length as u8);
    }
}

/// A length of a UTF-16 pool, and how many bytes it takes: one unit, or two
/// with the high bit set.
fn read_length16(data: &[u8]) -> Option<(usize, usize)> {
    let unit = |index: usize| {
        data.get(index * 2..index * 2 + 2)
            .map(|unit| usize::from(u16::from_le_bytes([unit[0], unit[1]])))
    };

    let first = unit(0)?;
    if first & 0x8000 != 0 {
        Some((((first & 0x7fff) << 16) | unit(1)?, 4))
    } else {
        Some((first, 2))
    }
}

fn write_length16(data: &mut Vec<u8>, length: usize) {
    if length > 0x7fff {
        data.extend((0x8000 | (length >> 16) as u16).to_le_bytes());
    }
    data.extend((length as u16).to_le_bytes());
}

fn size(length: usize) -> io::Result<u32> {
    u32::try_from(length).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "The strings are too long for a string pool.",
        )
    })
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

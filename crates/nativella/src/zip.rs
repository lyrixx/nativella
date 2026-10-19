//! ZIP archives: the APK of `nativella android`, its entries read from the
//! template and written again, aligned as zipalign does; the template and the
//! IPA of `nativella ios`, whose entries carry their Unix permissions.
//!
//! Only what Android and iOS write and read: no ZIP64 (an application stays
//! under 4 GB), no encryption, no comment.

use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};

use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;

const LOCAL_HEADER: u32 = 0x0403_4b50;
const CENTRAL_HEADER: u32 = 0x0201_4b50;
const END_OF_CENTRAL_DIRECTORY: u32 = 0x0605_4b50;
const END_OF_CENTRAL_DIRECTORY_LENGTH: usize = 22;
const LOCAL_HEADER_LENGTH: u64 = 30;

const STORED: u16 = 0;
const DEFLATED: u16 = 8;

/// The sizes and CRC follow the data, in a data descriptor: the entries this
/// module writes have them in their header instead.
const DATA_DESCRIPTOR_FLAG: u16 = 1 << 3;

/// The extra field that pads a stored entry to its alignment, as apksigner
/// writes it: the alignment, then zeros.
const ALIGNMENT_FIELD: u16 = 0xd935;

/// 1981-01-01 00:00, in MS-DOS format: the date Android's build tools give
/// their entries, for reproducible archives.
const DOS_DATE: u16 = (1 << 5) | 1;

/// The version of the specification an entry is made by: 2.0, on Unix, whose
/// file type and permissions are in the high half of its external attributes.
const MADE_BY_UNIX: u16 = (3 << 8) | 20;
const UNIX_TYPE: u32 = 0o170_000;
const UNIX_FILE: u32 = 0o100_000;
const UNIX_DIRECTORY: u32 = 0o040_000;
/// The attribute of directories, in MS-DOS format, in the low half.
const DOS_DIRECTORY: u32 = 0x10;

/// An entry, as the central directory of the template describes it.
pub struct Entry {
    pub name: String,
    version_made_by: u16,
    version_needed: u16,
    flags: u16,
    method: u16,
    time: u16,
    date: u16,
    crc32: u32,
    compressed_size: u32,
    size: u32,
    internal_attributes: u16,
    external_attributes: u32,
    local_header_offset: u32,
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        self.name.ends_with('/')
    }

    /// Its Unix permissions, when an archiver on Unix made it.
    pub fn unix_permissions(&self) -> Option<u32> {
        self.made_on_unix()
            .then_some((self.external_attributes >> 16) & 0o777)
    }

    /// Whether it is a symbolic link, or another file neither regular nor a
    /// directory, which only an archiver on Unix records.
    pub fn is_special(&self) -> bool {
        self.made_on_unix()
            && !matches!(
                (self.external_attributes >> 16) & UNIX_TYPE,
                0 | UNIX_FILE | UNIX_DIRECTORY
            )
    }

    fn made_on_unix(&self) -> bool {
        self.version_made_by >> 8 == MADE_BY_UNIX >> 8
    }
}

/// The entries of the archive `file`, in the order of its central directory.
pub fn entries(file: &mut File) -> io::Result<Vec<Entry>> {
    let length = file.seek(SeekFrom::End(0))?;
    if length < END_OF_CENTRAL_DIRECTORY_LENGTH as u64 {
        return Err(not_an_archive());
    }
    // The end of central directory record closes the archive, after a comment
    // of up to 64 KiB.
    let tail_length = length.min(END_OF_CENTRAL_DIRECTORY_LENGTH as u64 + 0xffff);
    file.seek(SeekFrom::Start(length - tail_length))?;
    let mut tail = vec![0; tail_length as usize];
    file.read_exact(&mut tail)?;

    let end = (0..=tail.len() - END_OF_CENTRAL_DIRECTORY_LENGTH)
        .rev()
        .find(|&offset| u32_at(&tail, offset) == END_OF_CENTRAL_DIRECTORY)
        .ok_or_else(not_an_archive)?;
    let end = &tail[end..];
    let count = u16_at(end, 10);
    let central_directory_size = u32_at(end, 12);
    let central_directory_offset = u32_at(end, 16);
    if count == u16::MAX || central_directory_offset == u32::MAX {
        return Err(invalid(
            "The template is a ZIP64 archive, which an application is not.",
        ));
    }

    file.seek(SeekFrom::Start(central_directory_offset.into()))?;
    let mut central_directory = vec![0; central_directory_size as usize];
    file.read_exact(&mut central_directory)?;

    let mut entries = Vec::with_capacity(count.into());
    let mut offset = 0;
    for _ in 0..count {
        let header = central_directory
            .get(offset..offset + 46)
            .filter(|header| u32_at(header, 0) == CENTRAL_HEADER)
            .ok_or_else(|| invalid("The central directory of the template is corrupted."))?;
        let name_length = usize::from(u16_at(header, 28));
        let extra_length = usize::from(u16_at(header, 30));
        let comment_length = usize::from(u16_at(header, 32));
        let name = central_directory
            .get(offset + 46..offset + 46 + name_length)
            .ok_or_else(|| invalid("The central directory of the template is corrupted."))?;

        entries.push(Entry {
            name: String::from_utf8(name.to_vec())
                .map_err(|_| invalid("An entry of the template is not named in UTF-8."))?,
            version_made_by: u16_at(header, 4),
            version_needed: u16_at(header, 6),
            flags: u16_at(header, 8),
            method: u16_at(header, 10),
            time: u16_at(header, 12),
            date: u16_at(header, 14),
            crc32: u32_at(header, 16),
            compressed_size: u32_at(header, 20),
            size: u32_at(header, 24),
            internal_attributes: u16_at(header, 36),
            external_attributes: u32_at(header, 38),
            local_header_offset: u32_at(header, 42),
        });
        offset += 46 + name_length + extra_length + comment_length;
    }

    Ok(entries)
}

/// The content of `entry`, uncompressed.
pub fn read(file: &mut File, entry: &Entry) -> io::Result<Vec<u8>> {
    let mut data = Vec::with_capacity(entry.size as usize);
    let compressed = data_of(file, entry)?;

    match entry.method {
        STORED => compressed.take(entry.size.into()).read_to_end(&mut data)?,
        DEFLATED => DeflateDecoder::new(compressed).read_to_end(&mut data)?,
        method => return Err(unknown_method(entry, method)),
    };

    Ok(data)
}

/// Writes the content of `entry`, uncompressed, to `out`, as it goes.
pub fn extract(file: &mut File, entry: &Entry, out: &mut dyn Write) -> io::Result<()> {
    let compressed = data_of(file, entry)?;

    let extracted = match entry.method {
        STORED => io::copy(&mut compressed.take(entry.size.into()), out)?,
        DEFLATED => io::copy(&mut DeflateDecoder::new(compressed), out)?,
        method => return Err(unknown_method(entry, method)),
    };
    if extracted != u64::from(entry.size) {
        return Err(invalid(&format!(
            "The data of {} is truncated.",
            entry.name
        )));
    }

    Ok(())
}

/// The compressed data of `entry`, after its local header.
fn data_of<'a>(file: &'a mut File, entry: &Entry) -> io::Result<io::Take<&'a mut File>> {
    let offset = u64::from(entry.local_header_offset);
    file.seek(SeekFrom::Start(offset))?;
    let mut header = [0; LOCAL_HEADER_LENGTH as usize];
    file.read_exact(&mut header)?;
    if u32_at(&header, 0) != LOCAL_HEADER {
        return Err(invalid(&format!(
            "The local header of {} is corrupted.",
            entry.name
        )));
    }
    let variable = u64::from(u16_at(&header, 26)) + u64::from(u16_at(&header, 28));
    file.seek(SeekFrom::Start(offset + LOCAL_HEADER_LENGTH + variable))?;

    Ok(file.take(entry.compressed_size.into()))
}

/// Writes the entries of an archive, then leaves its central directory to
/// the signature: the APK Signing Block goes between the two.
pub struct Writer<'a> {
    out: BufWriter<&'a mut File>,
    offset: u64,
    central_directory: Vec<u8>,
    count: u16,
}

/// An archive whose entries are written, not its central directory yet.
pub struct Unfinished {
    /// Where the entries end: the central directory comes next.
    pub entries_length: u64,
    pub central_directory: Vec<u8>,
    pub count: u16,
}

impl<'a> Writer<'a> {
    pub fn new(out: &'a mut File) -> Self {
        Self {
            out: BufWriter::new(out),
            offset: 0,
            central_directory: Vec::new(),
            count: 0,
        }
    }

    /// Copies `entry` from `template` as it is, compressed or not.
    pub fn copy(&mut self, entry: &Entry, template: &mut File) -> io::Result<()> {
        let offset = self.write_header(entry)?;
        let copied = io::copy(&mut data_of(template, entry)?, &mut self.out)?;
        if copied != u64::from(entry.compressed_size) {
            return Err(invalid(&format!(
                "The data of {} is truncated.",
                entry.name
            )));
        }
        self.offset += copied;

        self.add_central(entry, offset)
    }

    /// Adds an entry, uncompressed, what Android reads in place, as `write`
    /// writes it: its CRC and size go into its header once written.
    pub fn add_stored_with(
        &mut self,
        name: &str,
        write: impl FnOnce(&mut dyn Write) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut entry = new_entry(name, STORED);
        let offset = self.write_header(&entry)?;

        let mut data = Hashed {
            inner: &mut self.out,
            crc: flate2::Crc::new(),
            length: 0,
        };
        write(&mut data)?;
        entry.crc32 = data.crc.sum();
        entry.size = size(data.length)?;
        entry.compressed_size = entry.size;
        self.offset += data.length;
        self.patch_header(&entry, offset)?;

        self.add_central(&entry, offset)
    }

    /// Adds a file, compressed, with the Unix `permissions`, as `write` writes
    /// it: its CRC and sizes go into its header once written.
    pub fn add_deflated_with(
        &mut self,
        name: &str,
        permissions: u32,
        write: impl FnOnce(&mut dyn Write) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut entry = new_entry(name, DEFLATED);
        entry.version_made_by = MADE_BY_UNIX;
        entry.external_attributes = (UNIX_FILE | permissions) << 16;
        let offset = self.write_header(&entry)?;

        let mut compressed = Hashed {
            inner: &mut self.out,
            crc: flate2::Crc::new(),
            length: 0,
        };
        let mut encoder = DeflateEncoder::new(&mut compressed, Compression::default());
        let mut data = Hashed {
            inner: &mut encoder,
            crc: flate2::Crc::new(),
            length: 0,
        };
        write(&mut data)?;
        entry.crc32 = data.crc.sum();
        entry.size = size(data.length)?;
        encoder.finish()?;
        entry.compressed_size = size(compressed.length)?;
        self.offset += compressed.length;
        self.patch_header(&entry, offset)?;

        self.add_central(&entry, offset)
    }

    /// Adds a directory, with Unix permissions.
    pub fn add_directory(&mut self, name: &str) -> io::Result<()> {
        let mut entry = new_entry(&format!("{}/", name.trim_end_matches('/')), STORED);
        entry.version_made_by = MADE_BY_UNIX;
        entry.external_attributes = ((UNIX_DIRECTORY | 0o755) << 16) | DOS_DIRECTORY;
        let offset = self.write_header(&entry)?;

        self.add_central(&entry, offset)
    }

    /// Writes the CRC and the sizes of `entry`, known once its data is
    /// written, into its local header, which starts at `offset`.
    fn patch_header(&mut self, entry: &Entry, offset: u32) -> io::Result<()> {
        // The CRC, then the compressed and uncompressed sizes, from the 14th
        // byte of the header.
        self.out.seek(SeekFrom::Start(u64::from(offset) + 14))?;
        self.out.write_all(&entry.crc32.to_le_bytes())?;
        self.out.write_all(&entry.compressed_size.to_le_bytes())?;
        self.out.write_all(&entry.size.to_le_bytes())?;
        self.out.seek(SeekFrom::Start(self.offset))?;

        Ok(())
    }

    /// Adds an entry, compressed.
    pub fn add_deflated(&mut self, name: &str, data: &[u8]) -> io::Result<()> {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let mut entry = new_entry(name, DEFLATED);
        entry.crc32 = crc.sum();
        entry.compressed_size = size(compressed.len() as u64)?;
        entry.size = size(data.len() as u64)?;

        let offset = self.write_header(&entry)?;
        self.out.write_all(&compressed)?;
        self.offset += compressed.len() as u64;

        self.add_central(&entry, offset)
    }

    /// Writes the local header of `entry`, padded so that the data of a
    /// stored file is aligned: on 4 bytes, on 16 KiB for native libraries,
    /// which Android maps from the APK, whatever the size of the memory
    /// pages. A directory has no data to align. Returns where it starts.
    fn write_header(&mut self, entry: &Entry) -> io::Result<u32> {
        let offset = size(self.offset)?;
        let name = entry.name.as_bytes();
        let flags = entry.flags & !DATA_DESCRIPTOR_FLAG;

        let extra = if entry.method == STORED && !entry.is_dir() {
            let alignment: u64 = if entry.name.ends_with(".so") {
                16384
            } else {
                4
            };
            // The header, the name, then the field itself: 2 bytes of ID, 2
            // of size and 2 of alignment.
            let unpadded = self.offset + LOCAL_HEADER_LENGTH + name.len() as u64 + 6;
            let padding = (alignment - unpadded % alignment) % alignment;
            let mut extra = Vec::with_capacity(6 + padding as usize);
            extra.extend(ALIGNMENT_FIELD.to_le_bytes());
            extra.extend((2 + padding as u16).to_le_bytes());
            extra.extend((alignment as u16).to_le_bytes());
            extra.resize(6 + padding as usize, 0);
            extra
        } else {
            Vec::new()
        };

        let mut header =
            Vec::with_capacity(LOCAL_HEADER_LENGTH as usize + name.len() + extra.len());
        header.extend(LOCAL_HEADER.to_le_bytes());
        header.extend(entry.version_needed.to_le_bytes());
        header.extend(flags.to_le_bytes());
        header.extend(entry.method.to_le_bytes());
        header.extend(entry.time.to_le_bytes());
        header.extend(entry.date.to_le_bytes());
        header.extend(entry.crc32.to_le_bytes());
        header.extend(entry.compressed_size.to_le_bytes());
        header.extend(entry.size.to_le_bytes());
        header.extend((name.len() as u16).to_le_bytes());
        header.extend((extra.len() as u16).to_le_bytes());
        header.extend(name);
        header.extend(&extra);
        self.out.write_all(&header)?;
        self.offset += header.len() as u64;

        Ok(offset)
    }

    /// Adds `entry`, whose local header starts at `offset`, to the central
    /// directory.
    fn add_central(&mut self, entry: &Entry, offset: u32) -> io::Result<()> {
        let name = entry.name.as_bytes();
        let flags = entry.flags & !DATA_DESCRIPTOR_FLAG;
        let central = &mut self.central_directory;
        central.extend(CENTRAL_HEADER.to_le_bytes());
        central.extend(entry.version_made_by.to_le_bytes());
        central.extend(entry.version_needed.to_le_bytes());
        central.extend(flags.to_le_bytes());
        central.extend(entry.method.to_le_bytes());
        central.extend(entry.time.to_le_bytes());
        central.extend(entry.date.to_le_bytes());
        central.extend(entry.crc32.to_le_bytes());
        central.extend(entry.compressed_size.to_le_bytes());
        central.extend(entry.size.to_le_bytes());
        central.extend((name.len() as u16).to_le_bytes());
        // No extra field, no comment, on the first disk.
        central.extend([0; 6]);
        central.extend(entry.internal_attributes.to_le_bytes());
        central.extend(entry.external_attributes.to_le_bytes());
        central.extend(offset.to_le_bytes());
        central.extend(name);

        self.count = self
            .count
            .checked_add(1)
            .ok_or_else(|| invalid("A ZIP archive without ZIP64 holds 65535 entries at most."))?;

        Ok(())
    }

    pub fn finish(mut self) -> io::Result<Unfinished> {
        self.out.flush()?;

        Ok(Unfinished {
            entries_length: self.offset,
            central_directory: self.central_directory,
            count: self.count,
        })
    }
}

/// Ends the archive whose entries are written in `file`: its central
/// directory, then its end record, right after the entries.
pub fn close(file: &mut File, unfinished: &Unfinished) -> io::Result<()> {
    let end = end_of_central_directory(unfinished, unfinished.entries_length)?;
    file.seek(SeekFrom::Start(unfinished.entries_length))?;
    file.write_all(&unfinished.central_directory)?;
    file.write_all(&end)?;
    let length = file.stream_position()?;

    file.set_len(length)
}

/// The end of central directory record of an archive whose central
/// directory starts at `offset`.
pub fn end_of_central_directory(unfinished: &Unfinished, offset: u64) -> io::Result<Vec<u8>> {
    let offset = size(offset)?;
    let mut end = Vec::with_capacity(END_OF_CENTRAL_DIRECTORY_LENGTH);
    end.extend(END_OF_CENTRAL_DIRECTORY.to_le_bytes());
    // On the first and only disk.
    end.extend([0; 4]);
    end.extend(unfinished.count.to_le_bytes());
    end.extend(unfinished.count.to_le_bytes());
    end.extend(size(unfinished.central_directory.len() as u64)?.to_le_bytes());
    end.extend(offset.to_le_bytes());
    // No comment.
    end.extend([0; 2]);

    Ok(end)
}

/// A new entry, its CRC and sizes to come.
fn new_entry(name: &str, method: u16) -> Entry {
    Entry {
        name: name.to_owned(),
        version_made_by: 20,
        version_needed: if method == STORED { 10 } else { 20 },
        flags: 0,
        method,
        time: 0,
        date: DOS_DATE,
        crc32: 0,
        compressed_size: 0,
        size: 0,
        internal_attributes: 0,
        external_attributes: 0,
        local_header_offset: 0,
    }
}

/// A writer that computes the CRC and the length of what it writes.
struct Hashed<W> {
    inner: W,
    crc: flate2::Crc,
    length: u64,
}

impl<W: Write> Write for Hashed<W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buffer)?;
        self.crc.update(&buffer[..written]);
        self.length += written as u64;

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn size(length: u64) -> io::Result<u32> {
    u32::try_from(length).map_err(|_| {
        invalid("The application would exceed 4 GB, which a ZIP archive without ZIP64 cannot.")
    })
}

fn not_an_archive() -> io::Error {
    invalid("The template is not a ZIP archive, as an APK or an IPA is.")
}

fn unknown_method(entry: &Entry, method: u16) -> io::Error {
    invalid(&format!(
        "{} is compressed with the method {method}, which applications do not use.",
        entry.name
    ))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
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

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::path::Path;

    use super::*;

    #[test]
    fn entries_round_trip_aligned() {
        let path = Path::new(env!("OUT_DIR")).join(format!("zip-test-{}.zip", std::process::id()));
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .unwrap();

        let mut writer = Writer::new(&mut file);
        writer
            .add_deflated("AndroidManifest.xml", &[1; 1000])
            .unwrap();
        writer
            .add_stored_with("lib/arm64-v8a/libnativella_lib.so", |out| {
                out.write_all(b"ELF")
            })
            .unwrap();
        writer
            .add_stored_with("assets/odd", |out| out.write_all(b"x"))
            .unwrap();
        let unfinished = writer.finish().unwrap();
        let end = end_of_central_directory(&unfinished, unfinished.entries_length).unwrap();
        file.write_all(&unfinished.central_directory).unwrap();
        file.write_all(&end).unwrap();

        let entries = entries(&mut file).unwrap();
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "AndroidManifest.xml",
                "lib/arm64-v8a/libnativella_lib.so",
                "assets/odd"
            ]
        );
        assert_eq!(read(&mut file, &entries[0]).unwrap(), [1; 1000]);
        assert_eq!(read(&mut file, &entries[1]).unwrap(), b"ELF");

        let mut data = data_of(&mut file, &entries[1]).unwrap();
        let position = data.get_mut().stream_position().unwrap();
        assert_eq!(position % 16384, 0, "Libraries are page-aligned.");
        let mut data = data_of(&mut file, &entries[2]).unwrap();
        assert_eq!(data.get_mut().stream_position().unwrap() % 4, 0);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn unix_files_round_trip_streamed() {
        let path = Path::new(env!("OUT_DIR")).join(format!("zip-test-{}.ipa", std::process::id()));
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .unwrap();
        // More than a buffer of the encoder, and compressible.
        let executable: Vec<u8> = (0..100_000_u32).map(|byte| (byte % 7) as u8).collect();

        let mut writer = Writer::new(&mut file);
        writer.add_directory("Payload/Notes.app").unwrap();
        writer
            .add_deflated_with("Payload/Notes.app/Notes", 0o755, |out| {
                out.write_all(&executable)
            })
            .unwrap();
        writer
            .add_deflated_with("Payload/Notes.app/Info.plist", 0o644, |out| {
                out.write_all(b"<plist/>")
            })
            .unwrap();
        let unfinished = writer.finish().unwrap();
        close(&mut file, &unfinished).unwrap();

        let entries = entries(&mut file).unwrap();
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Payload/Notes.app/",
                "Payload/Notes.app/Notes",
                "Payload/Notes.app/Info.plist"
            ]
        );
        assert!(entries[0].is_dir());
        assert_eq!(entries[0].unix_permissions(), Some(0o755));
        assert_eq!(entries[1].unix_permissions(), Some(0o755));
        assert_eq!(entries[2].unix_permissions(), Some(0o644));
        assert!(entries.iter().all(|entry| !entry.is_special()));
        assert!(u64::from(entries[1].compressed_size) < executable.len() as u64);
        let mut extracted = Vec::new();
        extract(&mut file, &entries[1], &mut extracted).unwrap();
        assert_eq!(extracted, executable);
        assert_eq!(read(&mut file, &entries[2]).unwrap(), b"<plist/>");

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn what_is_too_short_is_not_an_archive() {
        let path =
            Path::new(env!("OUT_DIR")).join(format!("zip-test-{}.empty", std::process::id()));
        for content in [&b""[..], b"PK\x05\x06"] {
            std::fs::write(&path, content).unwrap();

            let error = entries(&mut File::open(&path).unwrap()).err().unwrap();

            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        }
        std::fs::remove_file(path).unwrap();
    }
}

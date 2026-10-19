//! The executable of an iOS app: a Mach-O file for arm64, thin, whose code
//! signature signature.rs replaces, in place.
//!
//! The signature is the last data of the file: the end of its last segment,
//! `__LINKEDIT`, where the load command `LC_CODE_SIGNATURE` points. The linker
//! signs the executables of the simulator ad hoc already; those of devices
//! may have no signature, and no such load command yet.

use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

use rsa::sha2::{Digest, Sha256};

const MH_MAGIC_64: u32 = 0xfeed_facf;
/// The magic numbers of universal binaries, as read in little-endian order.
const FAT_MAGICS: [u32; 2] = [0xbeba_feca, 0xbfba_feca];
const CPU_TYPE_ARM64: u32 = 0x0100_000c;
const MH_EXECUTE: u32 = 2;
const LC_SEGMENT_64: u32 = 0x19;
const LC_CODE_SIGNATURE: u32 = 0x1d;

const HEADER_LENGTH: usize = 32;
const SEGMENT_COMMAND_LENGTH: usize = 72;
const SECTION_LENGTH: usize = 80;
const LINKEDIT_DATA_COMMAND_LENGTH: usize = 16;

/// What the linker rounds the virtual size of segments to, the pages of
/// arm64: as codesign does for `__LINKEDIT`.
const SEGMENT_ALIGNMENT: u64 = 16384;
/// The pages of the code directory, whose hashes it holds.
pub const PAGE_SIZE: u64 = 4096;

/// An executable opened to be signed.
pub struct Executable {
    file: File,
    /// The header and the load commands, as the file has them.
    commands: Vec<u8>,
    /// Where `__TEXT` starts and ends in the file: the executable segment.
    pub text: (u64, u64),
    /// Where the load command of `__LINKEDIT` is in `commands`, and where the
    /// segment starts in the file.
    linkedit: (usize, u64),
    /// Where `LC_CODE_SIGNATURE` is in `commands`, if there is one.
    code_signature: Option<usize>,
    /// Where the signature starts: what comes before is the signed code.
    pub signature_offset: u64,
}

impl Executable {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        let length = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;

        let mut header = [0; HEADER_LENGTH];
        file.read_exact(&mut header)
            .map_err(|_| invalid("The executable of the template is not a Mach-O file."))?;
        let magic = u32_at(&header, 0);
        if FAT_MAGICS.contains(&magic) {
            return Err(invalid(
                "The executable of the template is a universal binary: a template is built for arm64 only.",
            ));
        }
        if magic != MH_MAGIC_64 {
            return Err(invalid(
                "The executable of the template is not a 64-bit Mach-O file.",
            ));
        }
        if u32_at(&header, 4) != CPU_TYPE_ARM64 || u32_at(&header, 12) != MH_EXECUTE {
            return Err(invalid(
                "The executable of the template is not an executable for arm64.",
            ));
        }
        let count = u32_at(&header, 16);
        let commands_length = u32_at(&header, 20) as usize;

        let mut commands = header.to_vec();
        commands.resize(HEADER_LENGTH + commands_length, 0);
        file.read_exact(&mut commands[HEADER_LENGTH..])
            .map_err(|_| corrupted())?;

        let mut text = None;
        let mut linkedit = None;
        let mut code_signature = None;
        // Where the data of the segments before __LINKEDIT ends, and where the
        // first section starts: the load commands may grow until there.
        let mut other_segments_end = 0;
        let mut first_section = u64::MAX;
        let mut offset = HEADER_LENGTH;
        for _ in 0..count {
            let command = commands.get(offset..offset + 8).ok_or_else(corrupted)?;
            let kind = u32_at(command, 0);
            let size = u32_at(command, 4) as usize;
            let command = commands
                .get(offset..offset + size)
                .filter(|_| size >= 8)
                .ok_or_else(corrupted)?;

            match kind {
                LC_SEGMENT_64 if size >= SEGMENT_COMMAND_LENGTH => {
                    let name = &command[8..24];
                    let file_offset = u64_at(command, 40);
                    let file_size = u64_at(command, 48);
                    let sections = u32_at(command, 64) as usize;
                    for section in 0..sections {
                        let start = SEGMENT_COMMAND_LENGTH + section * SECTION_LENGTH;
                        let section = command
                            .get(start..start + SECTION_LENGTH)
                            .ok_or_else(corrupted)?;
                        // Zero-filled sections have no data in the file.
                        let section_offset = u32_at(section, 48);
                        if section_offset != 0 {
                            first_section = first_section.min(section_offset.into());
                        }
                    }

                    if name == b"__LINKEDIT\0\0\0\0\0\0" {
                        linkedit = Some((offset, file_offset, file_size));
                    } else {
                        if name == b"__TEXT\0\0\0\0\0\0\0\0\0\0" {
                            text = Some((file_offset, file_offset + file_size));
                        }
                        other_segments_end = other_segments_end.max(file_offset + file_size);
                    }
                }
                LC_CODE_SIGNATURE if size >= LINKEDIT_DATA_COMMAND_LENGTH => {
                    code_signature = Some(offset);
                }
                _ => {}
            }
            offset += size;
        }
        if offset != commands.len() {
            return Err(corrupted());
        }

        let text = text.ok_or_else(|| invalid("The executable of the template has no __TEXT."))?;
        let (linkedit_command, linkedit_offset, linkedit_size) =
            linkedit.ok_or_else(|| invalid("The executable of the template has no __LINKEDIT."))?;
        // The signature grows __LINKEDIT, at the end of the file.
        let linkedit_end = linkedit_offset + linkedit_size;
        if other_segments_end > linkedit_offset || linkedit_end != length {
            return Err(invalid(
                "__LINKEDIT is not the last data of the executable of the template: it cannot be signed.",
            ));
        }

        let signature_start = match code_signature {
            Some(command) => {
                let start = u64::from(u32_at(&commands, command + 8));
                let size = u64::from(u32_at(&commands, command + 12));
                if start < linkedit_offset || start + size != linkedit_end {
                    return Err(invalid(
                        "The signature of the executable of the template is not at its end: it cannot be signed again.",
                    ));
                }
                start
            }
            None => {
                if commands.len() + LINKEDIT_DATA_COMMAND_LENGTH
                    > first_section.min(text.1) as usize
                {
                    return Err(invalid(
                        "The executable of the template has no room for a signature in its load commands.",
                    ));
                }
                linkedit_end
            }
        };

        Ok(Self {
            file,
            commands,
            text,
            linkedit: (linkedit_command, linkedit_offset),
            code_signature,
            // On 16 bytes, as codesign aligns it.
            signature_offset: signature_start.next_multiple_of(16),
        })
    }

    /// Writes the load commands for a signature of `size` bytes, and cuts the
    /// file where it starts: what the code directory hashes.
    pub fn reserve(&mut self, size: u32) -> io::Result<()> {
        let (linkedit_command, linkedit_offset) = self.linkedit;
        let linkedit_size = self.signature_offset + u64::from(size) - linkedit_offset;
        let dataoff = u32::try_from(self.signature_offset).map_err(|_| {
            invalid("The executable of the template exceeds 4 GB: it cannot be signed.")
        })?;

        // vmsize, then filesize.
        put_u64(
            &mut self.commands,
            linkedit_command + 32,
            linkedit_size.next_multiple_of(SEGMENT_ALIGNMENT),
        );
        put_u64(&mut self.commands, linkedit_command + 48, linkedit_size);

        match self.code_signature {
            Some(command) => {
                put_u32(&mut self.commands, command + 8, dataoff);
                put_u32(&mut self.commands, command + 12, size);
            }
            None => {
                let command = self.commands.len();
                self.commands.extend(LC_CODE_SIGNATURE.to_le_bytes());
                self.commands
                    .extend((LINKEDIT_DATA_COMMAND_LENGTH as u32).to_le_bytes());
                self.commands.extend(dataoff.to_le_bytes());
                self.commands.extend(size.to_le_bytes());
                let count = u32_at(&self.commands, 16) + 1;
                let commands_length = (self.commands.len() - HEADER_LENGTH) as u32;
                put_u32(&mut self.commands, 16, count);
                put_u32(&mut self.commands, 20, commands_length);
                self.code_signature = Some(command);
            }
        }

        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&self.commands)?;
        self.file.set_len(self.signature_offset)
    }

    /// The SHA-256 hashes of the pages of the signed code, the last one
    /// shorter.
    pub fn page_hashes(&mut self) -> io::Result<Vec<[u8; 32]>> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut reader = BufReader::new((&self.file).take(self.signature_offset));
        let mut hashes = Vec::with_capacity(self.signature_offset.div_ceil(PAGE_SIZE) as usize);
        let mut page = vec![0; PAGE_SIZE as usize];

        loop {
            let mut filled = 0;
            while filled < page.len() {
                match reader.read(&mut page[filled..])? {
                    0 => break,
                    read => filled += read,
                }
            }
            if filled == 0 {
                break;
            }
            hashes.push(Sha256::digest(&page[..filled]).into());
        }

        Ok(hashes)
    }

    /// Writes `signature` where the signature starts, padded with zeros to
    /// the `size` reserved for it.
    pub fn write_signature(&mut self, signature: &[u8], size: u32) -> io::Result<()> {
        let padding = (size as usize)
            .checked_sub(signature.len())
            .ok_or_else(|| io::Error::other("The signature exceeds the room reserved for it."))?;

        self.file.seek(SeekFrom::Start(self.signature_offset))?;
        self.file.write_all(signature)?;
        self.file.write_all(&vec![0; padding])?;
        self.file.sync_all()
    }
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("A 4-byte slice."),
    )
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("An 8-byte slice."),
    )
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn corrupted() -> io::Error {
    invalid("The load commands of the executable of the template are corrupted.")
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A minimal executable for arm64: `__TEXT`, with one section, then
    /// `__LINKEDIT`, signed or not, as the linker writes them, with room
    /// for more load commands before the code.
    pub fn executable(signed: bool) -> Vec<u8> {
        const CODE: u64 = 0x4000;
        let linkedit_offset = 2 * CODE;
        let linkedit_size: u64 = if signed { 0x100 + 0x40 } else { 0x100 };

        let mut commands = Vec::new();
        let segment = |name: &[u8], fileoff: u64, filesize: u64, sections: &[u32]| {
            let mut command = Vec::new();
            command.extend(LC_SEGMENT_64.to_le_bytes());
            command.extend(
                ((SEGMENT_COMMAND_LENGTH + sections.len() * SECTION_LENGTH) as u32).to_le_bytes(),
            );
            let mut segname = [0; 16];
            segname[..name.len()].copy_from_slice(name);
            command.extend(segname);
            command.extend((0x1_0000_0000 + fileoff).to_le_bytes());
            command.extend(filesize.next_multiple_of(SEGMENT_ALIGNMENT).to_le_bytes());
            command.extend(fileoff.to_le_bytes());
            command.extend(filesize.to_le_bytes());
            command.extend([5, 0, 0, 0, 5, 0, 0, 0]);
            command.extend((sections.len() as u32).to_le_bytes());
            command.extend([0; 4]);
            for &offset in sections {
                let mut section = vec![0; SECTION_LENGTH];
                section[..6].copy_from_slice(b"__text");
                section[16..22].copy_from_slice(b"__TEXT");
                section[48..52].copy_from_slice(&offset.to_le_bytes());
                command.extend(section);
            }
            command
        };
        commands.extend(segment(b"__TEXT", 0, 2 * CODE, &[CODE as u32]));
        commands.extend(segment(b"__LINKEDIT", linkedit_offset, linkedit_size, &[]));
        if signed {
            commands.extend(LC_CODE_SIGNATURE.to_le_bytes());
            commands.extend(16_u32.to_le_bytes());
            commands.extend(((linkedit_offset + 0x100) as u32).to_le_bytes());
            commands.extend(0x40_u32.to_le_bytes());
        }

        let mut file = Vec::new();
        file.extend(MH_MAGIC_64.to_le_bytes());
        file.extend(CPU_TYPE_ARM64.to_le_bytes());
        file.extend(0_u32.to_le_bytes());
        file.extend(MH_EXECUTE.to_le_bytes());
        file.extend((if signed { 3_u32 } else { 2 }).to_le_bytes());
        file.extend((commands.len() as u32).to_le_bytes());
        file.extend(0x0020_0085_u32.to_le_bytes());
        file.extend(0_u32.to_le_bytes());
        file.extend(commands);
        file.resize(CODE as usize, 0);
        // The code, then what the linker writes to __LINKEDIT: symbols, then
        // its signature.
        file.extend((0..CODE).map(|byte| (byte % 251) as u8));
        file.extend(vec![0x5a; 0x100]);
        if signed {
            file.extend(vec![0xfa; 0x40]);
        }
        file
    }

    fn open(bytes: &[u8], name: &str) -> (Executable, std::path::PathBuf) {
        let path =
            Path::new(env!("OUT_DIR")).join(format!("macho-test-{name}-{}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();

        (Executable::open(&path).unwrap(), path)
    }

    #[test]
    fn a_signature_replaces_the_one_of_the_linker() {
        let (mut executable, path) = open(&executable(true), "signed");
        assert_eq!(executable.text, (0, 0x8000));
        assert_eq!(executable.signature_offset, 0x8100);

        executable.reserve(0x1000).unwrap();
        let hashes = executable.page_hashes().unwrap();
        executable.write_signature(b"signature", 0x1000).unwrap();
        drop(executable);

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 0x8100 + 0x1000);
        assert_eq!(&bytes[0x8100..0x8109], b"signature");
        assert_eq!(hashes.len(), 9, "8 pages, and the 256 bytes of a ninth.");
        assert_eq!(
            hashes[8],
            <[u8; 32]>::from(Sha256::digest(&bytes[0x8000..0x8100]))
        );
        // __LINKEDIT, then LC_CODE_SIGNATURE, rewritten.
        let reopened = Executable::open(&path).unwrap();
        assert_eq!(reopened.signature_offset, 0x8100);
        let (linkedit, _) = reopened.linkedit;
        assert_eq!(u64_at(&reopened.commands, linkedit + 48), 0x1100);
        assert_eq!(u64_at(&reopened.commands, linkedit + 32), 0x4000);
        let signature = reopened.code_signature.unwrap();
        assert_eq!(u32_at(&reopened.commands, signature + 12), 0x1000);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn an_unsigned_executable_gets_its_load_command() {
        let original = executable(false);
        let (mut executable, path) = open(&original, "unsigned");
        assert_eq!(executable.signature_offset, 0x8100);

        executable.reserve(0x200).unwrap();
        executable.write_signature(b"", 0x200).unwrap();
        drop(executable);

        let reopened = Executable::open(&path).unwrap();
        assert_eq!(u32_at(&reopened.commands, 16), 3);
        let signature = reopened.code_signature.unwrap();
        assert_eq!(u32_at(&reopened.commands, signature + 8), 0x8100);
        assert_eq!(u32_at(&reopened.commands, signature + 12), 0x200);
        // The code itself is untouched.
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes[0x4000..0x8100], original[0x4000..0x8100]);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn only_thin_executables_for_arm64() {
        let mut universal = executable(true);
        universal[..4].copy_from_slice(&[0xca, 0xfe, 0xba, 0xbe]);
        let mut library = executable(true);
        library[12] = 6;
        let mut trailing = executable(true);
        trailing.push(0);

        for (bytes, name) in [
            (universal, "universal"),
            (library, "library"),
            (trailing, "trailing"),
            (b"#!/bin/sh".to_vec(), "script"),
        ] {
            let path = Path::new(env!("OUT_DIR"))
                .join(format!("macho-test-{name}-{}", std::process::id()));
            std::fs::write(&path, bytes).unwrap();

            let error = Executable::open(&path).err().unwrap();

            assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{name}: {error}");
            std::fs::remove_file(path).unwrap();
        }
    }
}

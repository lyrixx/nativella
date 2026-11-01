//! The binary formats of applications: the little-endian integers of ZIP
//! archives, of the binary XML and the resource table of Android, and of
//! Mach-O executables; the DER values of X.509 certificates and of
//! entitlements.

/// The little-endian `u16` at `offset` of `bytes`, which hold it.
pub(crate) fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        bytes[offset..offset + 2]
            .try_into()
            .expect("A 2-byte slice."),
    )
}

/// The little-endian `u32` at `offset` of `bytes`, which hold it.
pub(crate) fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("A 4-byte slice."),
    )
}

/// The little-endian `u64` at `offset` of `bytes`, which hold it.
pub(crate) fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("An 8-byte slice."),
    )
}

/// A DER value: its tag, its length, its content.
pub(crate) fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut value = vec![tag];
    let length = content.len();
    if length < 0x80 {
        value.push(length as u8);
    } else {
        let bytes = length.to_be_bytes();
        let significant = &bytes[bytes
            .iter()
            .position(|&byte| byte != 0)
            .unwrap_or(bytes.len() - 1)..];
        value.push(0x80 | significant.len() as u8);
        value.extend(significant);
    }
    value.extend(content);

    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers() {
        let bytes = [1, 2, 3, 4, 5, 6, 7, 8, 9];

        assert_eq!(u16_at(&bytes, 1), 0x0302);
        assert_eq!(u32_at(&bytes, 1), 0x0504_0302);
        assert_eq!(u64_at(&bytes, 1), 0x0908_0706_0504_0302);
    }

    #[test]
    fn der_lengths() {
        assert_eq!(der(0x04, &[1; 3])[..2], [0x04, 3]);
        assert_eq!(der(0x04, &[1; 200])[..3], [0x04, 0x81, 200]);
        assert_eq!(der(0x04, &[1; 300])[..4], [0x04, 0x82, 0x01, 0x2c]);
    }
}

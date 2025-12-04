//! Utility functions for PNG/ZIP polyglot operations

use crc32fast::Hasher;

/// Calculate CRC32 checksum for given data
pub fn calculate_crc32(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Read a big-endian u32 from byte slice
pub fn read_u32_be(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("slice too short"))
}

/// Write a big-endian u32 to byte slice
pub fn write_u32_be(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

/// Read a little-endian u32 from byte slice
pub fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("slice too short"))
}

/// Write a little-endian u32 to byte slice
pub fn write_u32_le(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// Read a little-endian u64 from byte slice
pub fn read_u64_le(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("slice too short"))
}

/// Write a little-endian u64 to byte slice
pub fn write_u64_le(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

/// Calculate offset adjustment for ZIP data embedded in PNG
pub fn calculate_offset_adjustment(idat_start_offset: u64, original_idat_length: u64) -> u64 {
    idat_start_offset + original_idat_length
}

/// Validate PNG signature
pub fn is_png_signature(data: &[u8]) -> bool {
    data.len() >= 8 && &data[0..8] == &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_calculation() {
        let data = b"Hello, World!";
        let crc = calculate_crc32(data);
        assert_eq!(crc, 0x4AC2B0C9);
    }

    #[test]
    fn test_u32_be_operations() {
        let mut buf = vec![0u8; 4];
        write_u32_be(&mut buf, 0, 0xDEADBEEF);
        assert_eq!(read_u32_be(&buf, 0), 0xDEADBEEF);
    }

    #[test]
    fn test_u32_le_operations() {
        let mut buf = vec![0u8; 4];
        write_u32_le(&mut buf, 0, 0xDEADBEEF);
        assert_eq!(read_u32_le(&buf, 0), 0xDEADBEEF);
    }

    #[test]
    fn test_png_signature_validation() {
        let valid_sig = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(is_png_signature(&valid_sig));

        let invalid_sig = [0x00, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(!is_png_signature(&invalid_sig));
    }
}

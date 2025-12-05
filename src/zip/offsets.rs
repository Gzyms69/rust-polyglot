//! ZIP central directory offset calculation and updating

use crate::utils::{read_u32_le, write_u32_le};
use crate::{PolyglotError, PolyglotResult};

/// ZIP End of Central Directory record
#[derive(Debug)]
pub struct EocdRecord {
    pub signature: u32,        // 0x06054B50
    pub disk_num: u16,         // Number of this disk
    pub cd_disk_num: u16,      // Disk where central directory starts
    pub num_entries_disk: u16, // Number of central directory entries on this disk
    pub num_entries_total: u16, // Total number of central directory entries
    pub cd_size: u32,          // Size of central directory
    pub cd_offset: u32,        // Offset of central directory from start of archive
    pub comment_length: u16,   // Comment length
}

/// ZIP64 End of Central Directory Locator
#[derive(Debug)]
pub struct Zip64EocdLocator {
    pub signature: u32,       // 0x07064B50
    pub disk_num: u32,        // Disk number where ZIP64 EOCD record starts
    pub zip64_eocd_offset: u64, // Offset of ZIP64 EOCD record
    pub total_disks: u32,     // Total number of disks
}

/// ZIP64 End of Central Directory Record
#[derive(Debug)]
pub struct Zip64EocdRecord {
    pub signature: u32,       // 0x06064B50
    pub eocd_size: u64,       // Size of this record (56)
    pub version_made: u16,    // Version made by
    pub version_needed: u16,  // Version needed to extract
    pub disk_num: u32,        // Number of this disk
    pub cd_disk_num: u32,     // Disk where central directory starts
    pub num_entries_disk: u64,  // Number of central directory entries on this disk
    pub num_entries_total: u64, // Total number of central directory entries
    pub cd_size: u64,         // Size of central directory
    pub cd_offset: u64,       // Offset of central directory from start of archive
}

/// Locate the End of Central Directory record in ZIP data
pub fn find_eocd(data: &[u8]) -> PolyglotResult<EocdRecord> {
    if data.len() < 22 {
        return Err(PolyglotError::ZipParse("ZIP data too short for EOCD".to_string()));
    }

    // Start from the end and search backwards for EOCD signature
    let mut offset = data.len() - 22; // EOCD is at least 22 bytes

    while offset > 0 {
        if read_u32_le(data, offset) == 0x06054B50 {
            // Found EOCD
            let record = EocdRecord {
                signature: read_u32_le(data, offset),
                disk_num: read_u16_le(data, offset + 4),
                cd_disk_num: read_u16_le(data, offset + 6),
                num_entries_disk: read_u16_le(data, offset + 8),
                num_entries_total: read_u16_le(data, offset + 10),
                cd_size: read_u32_le(data, offset + 12),
                cd_offset: read_u32_le(data, offset + 16),
                comment_length: read_u16_le(data, offset + 20),
            };

            // Validate comment length doesn't exceed remaining data
            if (record.comment_length as usize) <= data.len() - offset - 22 {
                return Ok(record);
            }
        }
        offset -= 1;
    }

    Err(PolyglotError::ZipParse("EOCD record not found".to_string()))
}

/// Check if ZIP uses ZIP64 format
pub fn uses_zip64(data: &[u8], eocd: &EocdRecord) -> bool {
    // ZIP64 is used if any field contains the reserved value 0xFFFFFFFF
    eocd.num_entries_disk == 0xFFFF ||
    eocd.num_entries_total == 0xFFFF ||
    eocd.cd_size == 0xFFFFFFFF ||
    eocd.cd_offset == 0xFFFFFFFF
}

/// Read little-endian u16
fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().expect("slice too short"))
}

/// Update all central directory entry offsets in ZIP data
pub fn update_central_directory_offsets(
    data: &mut [u8],
    original_cd_offset: u32,
    offset_adjustment: u64
) -> PolyglotResult<()> {
    if offset_adjustment == 0 {
        return Ok(()); // No adjustment needed
    }

    let adjustment = if offset_adjustment <= u32::MAX as u64 {
        offset_adjustment as u32
    } else {
        return Err(PolyglotError::ZipParse("Offset adjustment too large for ZIP format".to_string()));
    };

    let mut offset = original_cd_offset as usize;

    while offset + 46 <= data.len() { // Central directory header is at least 46 bytes
        // Check if this is a central directory entry (signature: 0x02014B50)
        if read_u32_le(data, offset) == 0x02014B50 {
            // Local file header offset is at offset + 42 in central directory entry
            let local_offset_offset = offset + 42;

            if local_offset_offset + 4 <= data.len() {
                let current_offset = read_u32_le(data, local_offset_offset);

                if current_offset >= original_cd_offset {
                    // This file is after the central directory, need to adjust
                    let new_offset = current_offset + adjustment;
                    write_u32_le(data, local_offset_offset, new_offset);
                }
            }

            // Move to next central directory entry
            // File name length is at offset + 28, extra field length at offset + 30, comment length at offset + 32
            let name_len = read_u16_le(data, offset + 28) as usize;
            let extra_len = read_u16_le(data, offset + 30) as usize;
            let comment_len = read_u16_le(data, offset + 32) as usize;

            offset += 46 + name_len + extra_len + comment_len;
        } else {
            break; // Not a central directory entry
        }
    }

    Ok(())
}

/// Update the central directory offset in the EOCD record
pub fn update_eocd_cd_offset(data: &mut [u8], eocd_offset: usize, new_cd_offset: u32) -> PolyglotResult<()> {
    // EOCD central directory offset is at position 16 from EOCD start
    let cd_offset_pos = eocd_offset + 16;

    if cd_offset_pos + 4 > data.len() {
        return Err(PolyglotError::ZipParse("Invalid EOCD offset".to_string()));
    }

    write_u32_le(data, cd_offset_pos, new_cd_offset);
    Ok(())
}

/// Validate that the data looks like a valid ZIP file
pub fn validate_zip_signature(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    // Check for local file header signature (PK\x03\x04)
    read_u32_le(data, 0) == 0x04034B50
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_signature_validation() {
        let mut data = vec![0x50, 0x4B, 0x03, 0x04]; // PK\x03\x04
        assert!(validate_zip_signature(&data));

        data[3] = 0x00; // Invalid
        assert!(!validate_zip_signature(&data));
    }

    #[test]
    fn test_eocd_locate() {
        // Create minimal ZIP with EOCD
        let mut zip_data = vec![0x50, 0x4B, 0x03, 0x04, 0x00]; // Local file header

        // Add minimal local file header data (30 bytes of zeros plus filename length, etc.)
        zip_data.extend_from_slice(&vec![0u8; 26]);

        // Add EOCD (PK\x05\x06)
        zip_data.extend_from_slice(&vec![0x50, 0x4B, 0x05, 0x06]);
        // Add 18 bytes of EOCD data (disk num, cd disk num, entries, etc. - all zeros)
        zip_data.extend_from_slice(&vec![0u8; 18]);

        let eocd = find_eocd(&zip_data).unwrap();
        assert_eq!(eocd.signature, 0x06054B50);
    }
}

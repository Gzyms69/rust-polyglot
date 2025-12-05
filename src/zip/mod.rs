//! ZIP archive manipulation module

pub mod offsets;

use std::path::Path;
use std::fs;
use crate::utils::read_u32_le;
use crate::{PolyglotError, PolyglotResult};

/// ZIP archive representation with offset tracking
#[derive(Debug)]
pub struct ZipArchive {
    pub data: Vec<u8>,
    pub eocd_offset: usize,
    pub eocd: offsets::EocdRecord,
}

impl ZipArchive {
    /// Read ZIP file from path
    pub fn read_zip(path: &Path) -> PolyglotResult<Self> {
        let data = fs::read(path)?;

        if !offsets::validate_zip_signature(&data) {
            return Err(PolyglotError::ZipParse("Invalid ZIP signature".to_string()));
        }

        let eocd = offsets::find_eocd(&data)?;

        // Find EOCD offset in data
        let mut eocd_offset = data.len() - 22; // Start search from end
        while eocd_offset > 0 {
            if read_u32_le(&data, eocd_offset) == 0x06054B50 {
                break;
            }
            eocd_offset -= 1;
        }

        Ok(Self {
            data,
            eocd_offset,
            eocd,
        })
    }

    /// Create from raw data
    pub fn from_data(data: Vec<u8>) -> PolyglotResult<Self> {
        if !offsets::validate_zip_signature(&data) {
            return Err(PolyglotError::ZipParse("Invalid ZIP signature".to_string()));
        }

        let eocd = offsets::find_eocd(&data)?;

        // Find EOCD offset in data
        let mut eocd_offset = data.len() - 22; // Start search from end
        while eocd_offset > 0 {
            if read_u32_le(&data, eocd_offset) == 0x06054B50 {
                break;
            }
            eocd_offset -= 1;
        }

        Ok(Self {
            data,
            eocd_offset,
            eocd,
        })
    }

    /// Calculate required offset adjustments for embedding at the given position
    pub fn calculate_offset_adjustment(&self, embed_position: u64) -> Result<u64, PolyglotError> {
        // For ZIP embedding, the adjustment depends on where we place the ZIP data
        // relative to the original ZIP structure. Since we append ZIP data to PNG IDAT,
        // the offset of the embedded ZIP data within the new file will be higher.

        // Currently we assume the ZIP is embedded after the original IDAT start position
        Ok(embed_position)
    }

    /// Update central directory offsets for new embedding position
    pub fn update_central_directory_offsets(&mut self, offset_adjustment: u64) -> PolyglotResult<()> {
        // ZIP64 is not supported in this basic implementation
        if offsets::uses_zip64(&self.data, &self.eocd) {
            return Err(PolyglotError::ZipParse("ZIP64 format not supported".to_string()));
        }

        offsets::update_central_directory_offsets(&mut self.data, self.eocd.cd_offset, offset_adjustment)?;

        // Update the EOCD central directory offset
        let new_cd_offset = self.eocd.cd_offset + offset_adjustment as u32;
        offsets::update_eocd_cd_offset(&mut self.data, self.eocd_offset, new_cd_offset)?;

        // Update our cached copy
        self.eocd.cd_offset = new_cd_offset;

        Ok(())
    }

    /// Get the ZIP data as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get mutable reference to ZIP data
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Write the modified ZIP to a file
    pub fn write_to_file(&self, path: &Path) -> PolyglotResult<()> {
        fs::write(path, &self.data)?;
        Ok(())
    }

    /// Size of the ZIP archive
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Create a ZIP archive from a directory
pub fn create_zip_from_directory(dir_path: &Path) -> PolyglotResult<ZipArchive> {
    use std::process::Command;

    // Use the system's zip utility to create the archive
    let temp_dir = tempfile::tempdir()?;
    let temp_zip = temp_dir.path().join("temp.zip");

    let status = Command::new("zip")
        .args(["-r", temp_zip.to_str().unwrap(), "."])
        .current_dir(dir_path)
        .status()
        .map_err(|e| PolyglotError::CreationFailed(format!("Failed to run zip command: {}", e)))?;

    if !status.success() {
        return Err(PolyglotError::CreationFailed("zip command failed".to_string()));
    }

    ZipArchive::read_zip(&temp_zip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_zip() -> Vec<u8> {
        // Minimal ZIP file with one empty file

        // Local file header
        let mut zip = vec![0x50, 0x4B, 0x03, 0x04]; // LFHS
        zip.extend_from_slice(&vec![0x0A, 0x00]); // Version needed
        zip.extend_from_slice(&vec![0x00, 0x00]); // GPB flag
        zip.extend_from_slice(&vec![0x00, 0x00]); // Compression method
        zip.extend_from_slice(&vec![0x00, 0x00, 0x00, 0x00]); // Last mod time/date
        zip.extend_from_slice(&vec![0x00, 0x00, 0x00, 0x00]); // CRC32
        zip.extend_from_slice(&vec![0x00, 0x00, 0x00, 0x00]); // Compressed size
        zip.extend_from_slice(&vec![0x00, 0x00, 0x00, 0x00]); // Uncompressed size
        zip.extend_from_slice(&vec![0x04, 0x00]); // Filename length
        zip.extend_from_slice(&vec![0x00, 0x00]); // Extra field length
        zip.extend_from_slice(b"test"); // Filename
        // Data (empty)

        // Central directory header
        zip.extend_from_slice(&vec![0x50, 0x4B, 0x01, 0x02]); // CDHS
        zip.extend_from_slice(&vec![0x0A, 0x00]); // Version made by
        zip.extend_from_slice(&vec![0x0A, 0x00]); // Version needed
        zip.extend_from_slice(&vec![0x00, 0x00]); // GPB flag
        zip.extend_from_slice(&vec![0x00, 0x00]); // Compression method
        zip.extend_from_slice(&vec![0x00, 0x00, 0x00, 0x00]); // Last mod time/date
        zip.extend_from_slice(&vec![0x00, 0x00, 0x00, 0x00]); // CRC32
        zip.extend_from_slice(&vec![0x00, 0x00, 0x00, 0x00]); // Compressed size
        zip.extend_from_slice(&vec![0x00, 0x00, 0x00, 0x00]); // Uncompressed size
        zip.extend_from_slice(&vec![0x04, 0x00]); // Filename length
        zip.extend_from_slice(&vec![0x00, 0x00]); // Extra field length
        zip.extend_from_slice(&vec![0x00, 0x00]); // File comment length
        zip.extend_from_slice(&vec![0x00, 0x00]); // Disk number
        zip.extend_from_slice(&vec![0x00, 0x00]); // Internal attributes
        zip.extend_from_slice(&vec![0x00, 0x00, 0x00, 0x00]); // External attributes
        zip.extend_from_slice(&vec![0x00, 0x00, 0x00, 0x00]); // Local header offset
        zip.extend_from_slice(b"test"); // Filename

        // End of central directory
        zip.extend_from_slice(&vec![0x50, 0x4B, 0x05, 0x06]); // EOCDS
        zip.extend_from_slice(&vec![0x00, 0x00]); // Disk number
        zip.extend_from_slice(&vec![0x00, 0x00]); // CD disk number
        zip.extend_from_slice(&vec![0x01, 0x00]); // Entries on this disk
        zip.extend_from_slice(&vec![0x01, 0x00]); // Total entries
        zip.extend_from_slice(&vec![0x16, 0x00, 0x00, 0x00]); // CD size (0x16 = 22 bytes)
        zip.extend_from_slice(&vec![0x1A, 0x00, 0x00, 0x00]); // CD offset (0x1A = 26 bytes from start)
        zip.extend_from_slice(&vec![0x00, 0x00]); // Comment length

        zip
    }

    #[test]
    fn test_zip_archive_from_data() {
        let zip_data = create_test_zip();
        let archive = ZipArchive::from_data(zip_data).unwrap();

        assert_eq!(archive.eocd.num_entries_total, 1);
        assert!(archive.eocd.cd_offset > 0);
    }

    #[test]
    fn test_offset_adjustment() {
        let zip_data = create_test_zip();
        let mut archive = ZipArchive::from_data(zip_data).unwrap();

        let original_cd_offset = archive.eocd.cd_offset;
        let adjustment = 100;

        archive.update_central_directory_offsets(adjustment as u64).unwrap();

        // CD offset in EOCD should be updated
        assert_eq!(archive.eocd.cd_offset, original_cd_offset + adjustment);
    }
}

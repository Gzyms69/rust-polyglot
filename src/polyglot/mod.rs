//! Core polyglot creation logic

use std::path::Path;
use crate::png::PngFile;
use crate::zip::ZipArchive;
use crate::flac::FlacFile;
use crate::utils::calculate_offset_adjustment;
use crate::{PolyglotError, PolyglotResult};

/// Core orchestrator for creating PNG/ZIP polyglots
pub struct PolyglotCreator {
    png: PngFile,
    zip: ZipArchive,
}

/// Create PNG+FLAC parasitic polyglot by embedding PNG in FLAC PADDING blocks
pub fn create_png_flac_polyglot(png_path: &Path, flac_path: &Path, output_path: &Path) -> PolyglotResult<()> {
    let png = PngFile::from_file(png_path)?;
    let mut flac = FlacFile::from_file(flac_path)?;

    // Inject PNG data into FLAC's PADDING metadata blocks (parasitic)
    flac.inject_png_to_padding(png.as_bytes())?;
    flac.write_to_file(output_path)?;

    println!("PNG+FLAC parasitic polyglot created: {} bytes", flac.as_bytes().len());
    Ok(())
}

impl PolyglotCreator {
    /// Create a new polyglot creator with PNG and ZIP files
    pub fn new(png_path: &Path, zip_path: &Path) -> PolyglotResult<Self> {
        let png = PngFile::from_file(png_path)?;
        let zip = ZipArchive::read_zip(zip_path)?;

        Ok(Self { png, zip })
    }

    /// Create polyglot from raw data
    pub fn from_data(png_data: Vec<u8>, zip_data: Vec<u8>) -> PolyglotResult<Self> {
        let png = PngFile::from_data(png_data)?;
        let zip = ZipArchive::from_data(zip_data)?;

        Ok(Self { png, zip })
    }

    /// Execute the complete polyglot creation workflow with specified embedding method
    pub fn create_polyglot(&mut self, output_path: &Path) -> PolyglotResult<()> {
        self.create_polyglot_with_method(output_path, "idat")
    }

    /// Execute the complete polyglot creation workflow
    pub fn create_polyglot_with_method(&mut self, output_path: &Path, method: &str) -> PolyglotResult<()> {
        match method {
            "zip" => {
                println!("Creating ZIP-dominant polyglot (PNG embedded in ZIP)...");
                self.create_zip_dominant_polyglot(output_path)
            }
            "idat" => {
                println!("Creating PNG-dominant polyglot (ZIP embedded in IDAT - parasitic)...");
                self.create_png_dominant_polyglot_idat(output_path)
            }
            "text" => {
                println!("Creating PNG-dominant polyglot (ZIP embedded in text chunk - parasitic)...");
                self.create_png_dominant_polyglot_text(output_path)
            }
            _ => {
                return Err(PolyglotError::InvalidInput(format!("Unknown embedding method: {}", method)));
            }
        }
    }

    /// Create ZIP-dominant polyglot (traditional method)
    fn create_zip_dominant_polyglot(&mut self, output_path: &Path) -> PolyglotResult<()> {
        // Step 1: Create new ZIP structure
        let original_png_data = self.png.as_bytes();
        let mut new_zip_data = Vec::new();

        // Add local file header for the PNG file within the ZIP
        let png_filename = b"image.png";
        let png_data = original_png_data;

        // Local File Header
        new_zip_data.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]); // Signature
        new_zip_data.extend_from_slice(&[0x0A, 0x00]); // Version needed
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // GPB flag
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // Compression method
        new_zip_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Last mod time/date
        new_zip_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // CRC32
        new_zip_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Compressed size
        new_zip_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Uncompressed size
        new_zip_data.extend_from_slice(&(png_filename.len() as u16).to_le_bytes()); // Filename length
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // Extra field length
        new_zip_data.extend_from_slice(png_filename); // Filename

        // Store PNG data (no compression for polyglot purposes)
        let png_offset = new_zip_data.len();
        new_zip_data.extend_from_slice(png_data);

        // Update the file header with correct sizes
        let file_header_pos = 14;
        let crc = crc32fast::hash(png_data);
        let compressed_size = png_data.len() as u32; // No compression, so same as uncompressed
        let uncompressed_size = png_data.len() as u32;

        // Copy CRC (4 bytes)
        new_zip_data[file_header_pos..file_header_pos+4].copy_from_slice(&crc.to_le_bytes());
        // Copy compressed size (4 bytes)
        new_zip_data[file_header_pos+4..file_header_pos+8].copy_from_slice(&compressed_size.to_le_bytes());
        // Copy uncompressed size (4 bytes)
        new_zip_data[file_header_pos+8..file_header_pos+12].copy_from_slice(&uncompressed_size.to_le_bytes());

        // Central Directory Header
        let cd_offset = new_zip_data.len();
        new_zip_data.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]); // Signature
        new_zip_data.extend_from_slice(&[0x0A, 0x03]); // Version made by
        new_zip_data.extend_from_slice(&[0x0A, 0x00]); // Version needed
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // GPB flag
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // Compression method
        new_zip_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Last mod time/date
        new_zip_data.extend_from_slice(&crc.to_le_bytes()); // CRC
        new_zip_data.extend_from_slice(&compressed_size.to_le_bytes()); // Compressed size
        new_zip_data.extend_from_slice(&uncompressed_size.to_le_bytes()); // Uncompressed size
        new_zip_data.extend_from_slice(&(png_filename.len() as u16).to_le_bytes()); // Filename length
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // Extra field length
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // File comment length
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // Disk number
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // Internal attributes
        new_zip_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // External attributes
        new_zip_data.extend_from_slice(&(png_offset as u32).to_le_bytes()); // Local header offset
        new_zip_data.extend_from_slice(png_filename); // Filename

        // End of Central Directory
        let eocd_pos = new_zip_data.len();
        new_zip_data.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06]); // Signature
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // Disk number
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // CD disk number
        new_zip_data.extend_from_slice(&[0x01, 0x00]); // Entries on this disk
        new_zip_data.extend_from_slice(&[0x01, 0x00]); // Total entries
        new_zip_data.extend_from_slice(&((new_zip_data.len() - cd_offset) as u32).to_le_bytes()); // CD size
        new_zip_data.extend_from_slice(&(cd_offset as u32).to_le_bytes()); // CD offset
        new_zip_data.extend_from_slice(&[0x00, 0x00]); // Comment length

        // Write the ZIP-based polyglot
        std::fs::write(output_path, &new_zip_data)?;

        println!("ZIP-dominant polyglot created: {} bytes", new_zip_data.len());
        Ok(())
    }

    /// Create PNG-dominant polyglot with ZIP in IDAT chunk
    fn create_png_dominant_polyglot_idat(&mut self, output_path: &Path) -> PolyglotResult<()> {
        let (idat_offset, idat_length) = self.png.find_first_idat()?;
        let embed_position = idat_offset as u64 + idat_length as u64 + 8;

        self.zip.update_central_directory_offsets(embed_position)?;
        self.png.append_to_idat(self.zip.as_bytes())?;

        self.png.write_to_file(output_path)?;
        println!("PNG-dominant polyglot (IDAT method) created: {} bytes", self.png.as_bytes().len());
        Ok(())
    }

    /// Create PNG-dominant polyglot with ZIP in text chunk
    fn create_png_dominant_polyglot_text(&mut self, output_path: &Path) -> PolyglotResult<()> {
        self.png.add_zip_text_chunk(self.zip.as_bytes())?;

        self.png.write_to_file(output_path)?;
        println!("PNG-dominant polyglot (text method) created: {} bytes", self.png.as_bytes().len());
        Ok(())
    }

    /// Get final polyglot data without writing to file
    pub fn create_polyglot_in_memory(&mut self) -> PolyglotResult<Vec<u8>> {
        // Same steps as create_polyglot but return data instead of writing
        let (idat_offset, idat_length) = self.png.find_first_idat()?;
        let embed_position = idat_offset as u64 + idat_length as u64 + 8;

        self.zip.update_central_directory_offsets(embed_position)?;
        self.png.append_to_idat(self.zip.as_bytes())?;

        Ok(self.png.raw_data.clone())
    }

    /// Get PNG component
    pub fn png(&self) -> &PngFile {
        &self.png
    }

    /// Get ZIP component
    pub fn zip(&self) -> &ZipArchive {
        &self.zip
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Helper functions from PNG and ZIP tests
    fn create_test_png() -> Vec<u8> {
        let mut png = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        ];

        // IHDR chunk
        let ihdr_data = [
            0x00, 0x00, 0x00, 0x01, // width = 1
            0x00, 0x00, 0x00, 0x01, // height = 1
            0x08, // bit depth = 8
            0x02, // color type = 2 (RGB)
            0x00, // compression = 0
            0x00, // filter = 0
            0x00, // interlace = 0
        ];

        let ihdr_length = ihdr_data.len() as u32;
        png.extend_from_slice(&ihdr_length.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&ihdr_data);
        let ihdr_crc_data = [b"IHDR".as_slice(), &ihdr_data].concat();
        let ihdr_crc = crate::utils::calculate_crc32(&ihdr_crc_data);
        png.extend_from_slice(&ihdr_crc.to_be_bytes());

        // IDAT chunk with minimal compressed data
        let idat_data = [
            0x78, 0x9C, 0xED, 0xC1, 0x01, 0x01, 0x00, 0x00, 0x00, 0x80, 0x90, 0xFE, 0x37, 0x10
        ];
        let idat_length = idat_data.len() as u32;
        png.extend_from_slice(&idat_length.to_be_bytes());
        png.extend_from_slice(b"IDAT");
        png.extend_from_slice(&idat_data);
        let idat_crc_data = [b"IDAT".as_slice(), &idat_data].concat();
        let idat_crc = crate::utils::calculate_crc32(&idat_crc_data);
        png.extend_from_slice(&idat_crc.to_be_bytes());

        // IEND chunk
        png.extend_from_slice(&0u32.to_be_bytes());
        png.extend_from_slice(b"IEND");
        let iend_crc = crate::utils::calculate_crc32(b"IEND");
        png.extend_from_slice(&iend_crc.to_be_bytes());

        png
    }

    fn create_test_zip() -> Vec<u8> {
        // Minimal ZIP file with one empty file
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
        zip.extend_from_slice(&vec![0x16, 0x00, 0x00, 0x00]); // CD size
        zip.extend_from_slice(&vec![0x1A, 0x00, 0x00, 0x00]); // CD offset
        zip.extend_from_slice(&vec![0x00, 0x00]); // Comment length

        zip
    }

    #[test]
    fn test_polyglot_creation() {
        let png_data = create_test_png();
        let zip_data = create_test_zip();

        let mut creator = PolyglotCreator::from_data(png_data, zip_data).unwrap();

        // Test in-memory creation
        let polyglot_data = creator.create_polyglot_in_memory().unwrap();

        // Should be larger than original PNG
        assert!(polyglot_data.len() > create_test_png().len());

        // Should still start with PNG signature
        assert_eq!(&polyglot_data[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

        // Should contain ZIP signature somewhere
        let zip_sig_pos = polyglot_data.windows(4).position(|w| w == [0x50, 0x4B, 0x03, 0x04]);
        assert!(zip_sig_pos.is_some());
    }
}

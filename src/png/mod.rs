//! PNG chunk manipulation module

pub mod parser;

use std::path::Path;
use std::fs;
use crate::utils::write_u32_be;
use crate::{PolyglotError, PolyglotResult};
pub use parser::{Chunk, ParsedPng};

/// PNG file representation with manipulation capabilities
#[derive(Debug, Clone)]
pub struct PngFile {
    pub raw_data: Vec<u8>,
    pub parsed: ParsedPng,
}

impl PngFile {
    /// Load PNG file from path
    pub fn from_file(path: &Path) -> PolyglotResult<Self> {
        let raw_data = fs::read(path)?;
        let parsed = parser::parse_png_chunks(&raw_data)?;

        Ok(Self { raw_data, parsed })
    }

    /// Create from raw data
    pub fn from_data(data: Vec<u8>) -> PolyglotResult<Self> {
        let parsed = parser::parse_png_chunks(&data)?;
        Ok(Self { raw_data: data, parsed })
    }

    /// Find the first IDAT chunk and return its offset and length
    pub fn find_first_idat(&self) -> Result<(usize, usize), PolyglotError> {
        let chunk = parser::find_first_idat(&self.parsed)?;
        Ok((chunk.data_offset, chunk.data.len()))
    }

    /// Embed ZIP data in a new tEXt chunk (parasitic - embeds in metadata)
    pub fn add_zip_text_chunk(&mut self, zip_data: &[u8]) -> PolyglotResult<()> {
        // Find IEND position for insertion
        let iend_pos = self.raw_data.windows(4).position(|w| w == b"IEND").unwrap() - 4;

        // Create new chunk data
        let keyword = b"ZIP Archive";
        let mut chunk_data = Vec::new();
        chunk_data.extend_from_slice(keyword);
        chunk_data.push(0); // Null terminator
        chunk_data.extend_from_slice(zip_data);

        let chunk_length = chunk_data.len() as u32;
        let mut new_chunk = Vec::new();
        new_chunk.extend_from_slice(&chunk_length.to_be_bytes());
        new_chunk.extend_from_slice(b"tEXt");
        new_chunk.extend_from_slice(&chunk_data);
        let crc_data = [b"tEXt".as_slice(), &chunk_data].concat();
        let crc = crate::utils::calculate_crc32(&crc_data);
        new_chunk.extend_from_slice(&crc.to_be_bytes());

        // Insert before IEND
        let mut new_data = self.raw_data[0..iend_pos].to_vec();
        new_data.extend_from_slice(&new_chunk);
        new_data.extend_from_slice(&self.raw_data[iend_pos..]);

        self.raw_data = new_data;
        self.parsed = parser::parse_png_chunks(&self.raw_data)?;

        Ok(())
    }

    /// Append WAV data to the first IDAT chunk (parasitic - embeds in image data)
    pub fn append_wav_to_idat(&mut self, wav_data: &[u8]) -> PolyglotResult<()> {
        self.append_to_idat(wav_data)
    }

    /// Append data to the first IDAT chunk (parasitic - embeds in image data)
    pub fn append_to_idat(&mut self, additional_data: &[u8]) -> PolyglotResult<()> {
        let idat_chunk = parser::find_first_idat(&self.parsed)?
            .clone();

        // Build new PNG data with modified IDAT
        let mut new_data = Vec::with_capacity(self.raw_data.len() + additional_data.len());

        // Copy PNG signature
        new_data.extend_from_slice(&self.raw_data[0..8]);

        // Process chunks
        let mut found_idat = false;
        for chunk in &self.parsed.chunks {
            if chunk.chunk_type == *b"IDAT" && !found_idat {
                // Modify IDAT chunk
                found_idat = true;

                // New IDAT data = original + additional
                let new_idat_data = [chunk.data.as_slice(), additional_data].concat();
                let new_length = new_idat_data.len() as u32;

                // Write length
                new_data.extend_from_slice(&new_length.to_be_bytes());

                // Write type
                new_data.extend_from_slice(b"IDAT");

                // Write data
                new_data.extend_from_slice(&new_idat_data);

                // Calculate and write CRC
                let mut crc_data = b"IDAT".to_vec();
                crc_data.extend_from_slice(&new_idat_data);
                let crc = crate::utils::calculate_crc32(&crc_data);
                new_data.extend_from_slice(&crc.to_be_bytes());
            } else {
                // Copy chunk as-is
                let length_bytes = chunk.length.to_be_bytes();
                new_data.extend_from_slice(&length_bytes);
                new_data.extend_from_slice(&chunk.chunk_type);
                new_data.extend_from_slice(&chunk.data);
                new_data.extend_from_slice(&chunk.crc.to_be_bytes());
            }
        }

        // Replace raw data
        self.raw_data = new_data;

        // Re-parse after modification to ensure consistency
        self.parsed = parser::parse_png_chunks(&self.raw_data)?;

        Ok(())
    }

    /// Recalculate CRC for all chunks
    pub fn recalculate_crcs(&mut self) -> PolyglotResult<()> {
        let mut offset = 8; // Skip PNG signature

        for chunk in &self.parsed.chunks {
            offset += 4; // Skip length
            let type_offset = offset;
            offset += 4; // Skip type

            let data_start = offset;
            let data_end = offset + chunk.length as usize;
            offset = data_end;

            let crc_offset = offset;
            offset += 4; // Skip CRC

            // Recalculate CRC
            let mut crc_data = Vec::with_capacity(4 + chunk.length as usize);
            crc_data.extend_from_slice(&chunk.chunk_type);
            crc_data.extend_from_slice(&self.raw_data[data_start..data_end]);
            let new_crc = crate::utils::calculate_crc32(&crc_data);

            write_u32_be(&mut self.raw_data, crc_offset, new_crc);
        }

        Ok(())
    }

    /// Write the modified PNG to a file
    pub fn write_to_file(&self, path: &Path) -> PolyglotResult<()> {
        fs::write(path, &self.raw_data)?;
        Ok(())
    }

    /// Get the raw data
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Helper to create a minimal PNG for testing
    fn create_test_png() -> Vec<u8> {
        // Minimal PNG header
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

    #[test]
    fn test_png_file_load() {
        let png_data = create_test_png();
        let file = PngFile::from_data(png_data).unwrap();
        assert_eq!(file.parsed.chunks.len(), 3); // IHDR, IDAT, IEND
    }

    #[test]
    fn test_idat_finding() {
        let png_data = create_test_png();
        let file = PngFile::from_data(png_data).unwrap();
        let (offset, length) = file.find_first_idat().unwrap();
        assert!(offset > 0);
        assert!(length > 0);
    }

    #[test]
    fn test_append_to_idat() {
        let png_data = create_test_png();
        let mut file = PngFile::from_data(png_data.clone()).unwrap();

        // Alternative test with real PNG file
        let mut file = PngFile::from_file(std::path::Path::new("test_files/input/test_image.png")).unwrap();

        println!("Original PNG data length: {}", png_data.len());
        println!("Original chunks: {}", file.parsed.chunks.len());

        let original_size = file.raw_data.len();
        let additional_data = b"extra data";

        println!("Adding {} bytes to IDAT", additional_data.len());

        let result = file.append_to_idat(additional_data);
        if let Err(e) = &result {
            println!("Error: {:?}", e);
            // Print first 200 bytes of modified data
            println!("First 200 bytes after modification:");
            for (i, &byte) in file.raw_data.iter().take(200).enumerate() {
                if i % 16 == 0 { print!("{:04x}: ", i); }
                print!("{:02x} ", byte);
                if i % 16 == 15 { println!(); }
            }
            println!();
            panic!("Append failed: {:?}", e);
        }

        result.unwrap();

        // File should be larger
        assert!(file.raw_data.len() > original_size);

        // IDAT chunk should have been modified
        let (offset, length) = file.find_first_idat().unwrap();
        assert!(length > additional_data.len()); // Original length + additional
    }
}

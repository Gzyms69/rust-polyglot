//! Low-level PNG chunk parsing using manual byte slicing

use std::io::{self, Read};
use crate::utils::{read_u32_be};
use crate::PolyglotError;

/// PNG chunk structure
#[derive(Debug, Clone)]
pub struct Chunk {
    pub length: u32,
    pub chunk_type: [u8; 4],
    pub data: Vec<u8>,
    pub crc: u32,
    pub data_offset: usize, // Offset of chunk data in file
}

/// PNG file representation
#[derive(Debug, Clone)]
pub struct ParsedPng {
    pub chunks: Vec<Chunk>,
}

/// Parse PNG chunks from byte data
pub fn parse_png_chunks(data: &[u8]) -> Result<ParsedPng, PolyglotError> {
    if !crate::utils::is_png_signature(data) {
        return Err(PolyglotError::PngParse("Invalid PNG signature".to_string()));
    }

    let mut offset = 8; // Skip PNG signature
    let mut chunks = Vec::new();

    while offset + 12 <= data.len() {
        let length = read_u32_be(data, offset);
        offset += 4;

        if offset + 4 > data.len() {
            return Err(PolyglotError::PngParse("Insufficient data for chunk type".to_string()));
        }
        let chunk_type = [data[offset], data[offset + 1], data[offset + 2], data[offset + 3]];
        offset += 4;

        let data_end = offset + length as usize;
        if data_end > data.len() {
            return Err(PolyglotError::PngParse("Chunk data extends beyond file".to_string()));
        }

        let chunk_data = data[offset..data_end].to_vec();
        offset = data_end;

        let crc = read_u32_be(data, offset);
        offset += 4;

        // Verify CRC
        let mut crc_data = Vec::with_capacity(4 + length as usize);
        crc_data.extend_from_slice(&chunk_type);
        crc_data.extend_from_slice(&chunk_data);
        let calculated_crc = crate::utils::calculate_crc32(&crc_data);

        if crc != calculated_crc {
            return Err(PolyglotError::CrcMismatch(
                String::from_utf8_lossy(&chunk_type).to_string()
            ));
        }

        chunks.push(Chunk {
            length,
            chunk_type,
            data: chunk_data,
            crc,
            data_offset: offset - length as usize - 8, // Start of data relative to chunk start
        });

        // IEND indicates end of PNG chunks
        if &chunk_type == b"IEND" {
            break;
        }
    }

    if chunks.is_empty() {
        return Err(PolyglotError::PngParse("No chunks found".to_string()));
    }

    Ok(ParsedPng { chunks })
}

/// Find the first IDAT chunk in parsed PNG
pub fn find_first_idat(png: &ParsedPng) -> Result<&Chunk, PolyglotError> {
    for chunk in &png.chunks {
        if &chunk.chunk_type == b"IDAT" {
            return Ok(chunk);
        }
    }
    Err(PolyglotError::NoIdatChunk)
}

/// Get all IDAT chunks
pub fn find_all_idat(png: &ParsedPng) -> Vec<&Chunk> {
    png.chunks.iter().filter(|c| &c.chunk_type == b"IDAT").collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_png() {
        let result = parse_png_chunks(&[0, 1, 2]);
        assert!(matches!(result, Err(PolyglotError::PngParse(_))));
    }

    #[test]
    fn test_invalid_signature() {
        let invalid_png = [0x00, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // Invalid first byte
        let result = parse_png_chunks(&invalid_png);
        assert!(matches!(result, Err(PolyglotError::PngParse(_))));
    }
}

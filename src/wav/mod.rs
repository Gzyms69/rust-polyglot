//! WAV file format support for PNG+WAV parasitic polyglots (PNG embedded in RIFF chunks)

use std::path::Path;
use std::fs;
use crate::{PolyglotError, PolyglotResult};

/// RIFF file signature
const RIFF_SIGNATURE: &[u8; 4] = b"RIFF";
const WAVE_SIGNATURE: &[u8; 4] = b"WAVE";

/// FOURCC for PNG embedding chunk (PNG with trailing space for uniqueness)
const PNG_CHUNK_FOURCC: [u8; 4] = *b"pnG ";

/// RIFF chunk header (all chunks have this format)
#[derive(Debug, Clone)]
pub struct RiffChunkHeader {
    pub fourcc: [u8; 4],
    pub data_size: u32, // Little-endian
}

/// Generic RIFF chunk
#[derive(Debug, Clone)]
pub struct RiffChunk {
    pub header: RiffChunkHeader,
    pub data: Vec<u8>,
}

/// RIFF header (first 12 bytes)
#[derive(Debug, Clone)]
pub struct RiffHeader {
    pub file_size: u32, // Little-endian, total size after this field
}

/// fmt chunk (mandatory for WAV)
#[derive(Debug, Clone)]
pub struct FmtChunk {
    pub header: RiffChunkHeader,
    pub data: Vec<u8>, // Raw fmt data
}

/// data chunk (mandatory for WAV, contains audio samples)
#[derive(Debug, Clone)]
pub struct DataChunk {
    pub header: RiffChunkHeader,
    pub data: Vec<u8>, // Raw audio data
}

/// Parsed RIFF/WAV structure (minimal for our needs)
#[derive(Debug, Clone)]
pub struct RiffStructure {
    pub header: RiffHeader,
    pub fmt_chunk: FmtChunk,
    pub data_chunk: DataChunk,
    pub additional_chunks: Vec<RiffChunk>, // Chunks after data
}

/// WAV file handler for parasitic polyglots
#[derive(Debug, Clone)]
pub struct WavFile {
    pub raw_data: Vec<u8>,
    pub structure: RiffStructure,
}

impl WavFile {
    /// Load WAV file from raw data
    pub fn from_data(raw_data: Vec<u8>) -> PolyglotResult<Self> {
        if raw_data.len() < 12 {
            return Err(PolyglotError::WavParse("File too short for RIFF/WAV".to_string()));
        }

        // Validate RIFF signature
        if &raw_data[0..4] != RIFF_SIGNATURE {
            return Err(PolyglotError::InvalidRiffHeader);
        }

        // Validate WAVE format
        if &raw_data[8..12] != WAVE_SIGNATURE {
            return Err(PolyglotError::WavParse("Not a WAVE file".to_string()));
        }

        let structure = RiffStructure::parse(&raw_data)?;

        Ok(Self { raw_data, structure })
    }

    /// Load WAV file from path
    pub fn from_file(path: &Path) -> PolyglotResult<Self> {
        let raw_data = fs::read(path)?;

        if raw_data.len() < 12 {
            return Err(PolyglotError::WavParse("File too short for RIFF/WAV".to_string()));
        }

        // Validate RIFF signature
        if &raw_data[0..4] != RIFF_SIGNATURE {
            return Err(PolyglotError::InvalidRiffHeader);
        }

        // Validate WAVE format
        if &raw_data[8..12] != WAVE_SIGNATURE {
            return Err(PolyglotError::WavParse("Not a WAVE file".to_string()));
        }

        let structure = RiffStructure::parse(&raw_data)?;

        Ok(Self { raw_data, structure })
    }

    /// Get raw data
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw_data
    }

    /// Write modified WAV to file
    pub fn write_to_file(&self, path: &Path) -> PolyglotResult<()> {
        fs::write(path, &self.raw_data)?;
        Ok(())
    }

    /// Embed PNG data as custom RIFF chunk (WAV-dominant polyglot)
    pub fn embed_png_data(&mut self, png_data: &[u8]) -> PolyglotResult<()> {
        self.structure.insert_png_chunk(png_data)?;
        // Rebuild raw data with updated structure
        self.raw_data = self.structure.to_bytes()?;
        Ok(())
    }

    /// Load WAV-dominant polyglot and extract PNG data if present
    pub fn extract_png_from_wav_polyglot(wav_data: &[u8]) -> Option<Vec<u8>> {
        // First check if it starts with PNG (PNG-dominant)
        if wav_data.len() >= 8 && &wav_data[0..8] == b"\x89PNG\r\n\x1a\n" {
            // This is PNG-dominant, let extract module handle it
            return None;
        }

        // Check if this is WAV and parse it
        if wav_data.len() >= 12 && &wav_data[0..4] == b"RIFF"
            && let Ok(structure) = RiffStructure::parse(wav_data) {
                return structure.extract_png_data();
            }

        None
    }

    /// Extract embedded PNG data if present
    pub fn extract_png_data(&self) -> Option<Vec<u8>> {
        self.structure.extract_png_data()
    }
}

impl RiffStructure {
    /// Insert PNG data as custom RIFF chunk after data chunk
    pub fn insert_png_chunk(&mut self, png_data: &[u8]) -> PolyglotResult<()> {
        // Check for size overflow
        let png_size = png_data.len() as u64;
        let chunk_data_size = 8 + png_size; // 4-byte FOURCC + 4-byte size + data
        let padding_size = if png_size % 2 == 1 { 1 } else { 0 }; // RIFF padding
        let additional_size = chunk_data_size + padding_size;

        if additional_size > u32::MAX as u64 {
            return Err(PolyglotError::SizeOverflow);
        }

        // Update RIFF file size in header
        let new_total_size = self.header.file_size as u64 + additional_size;
        if new_total_size > u32::MAX as u64 {
            return Err(PolyglotError::SizeOverflow);
        }
        self.header.file_size = new_total_size as u32;

        // Create PNG chunk
        let png_chunk = RiffChunk {
            header: RiffChunkHeader {
                fourcc: PNG_CHUNK_FOURCC,
                data_size: png_size as u32,
            },
            data: png_data.to_vec(),
        };

        // Insert after data chunk (preserves audio playback compatibility)
        self.additional_chunks.push(png_chunk);

        Ok(())
    }

    /// Get PNG data from embedded chunk if present
    pub fn extract_png_data(&self) -> Option<Vec<u8>> {
        self.additional_chunks
            .iter()
            .find(|chunk| chunk.header.fourcc == PNG_CHUNK_FOURCC)
            .map(|chunk| chunk.data.clone())
    }

    /// Parse RIFF structure from raw bytes
    pub fn parse(data: &[u8]) -> PolyglotResult<Self> {
        if data.len() < 12 {
            return Err(PolyglotError::WavParse("Data too short for RIFF header".to_string()));
        }

        // Parse RIFF header
        let file_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let header = RiffHeader { file_size };

        let mut offset = 12; // After RIFF header + WAVE signature
        let mut fmt_chunk: Option<FmtChunk> = None;
        let mut data_chunk: Option<DataChunk> = None;
        let mut additional_chunks = Vec::new();

        // Parse chunks until we have the mandatory fmt and data chunks
        while offset + 8 <= data.len() {
            let chunk_header = Self::parse_chunk_header(&data[offset..])?;
            let chunk_data_start = offset + 8;
            let chunk_data_end = chunk_data_start + chunk_header.data_size as usize;

            if chunk_data_end > data.len() {
                return Err(PolyglotError::WavParse("Chunk data extends beyond file".to_string()));
            }

            let chunk_data = data[chunk_data_start..chunk_data_end].to_vec();

            match &chunk_header.fourcc {
                b"fmt " => {
                    fmt_chunk = Some(FmtChunk {
                        header: chunk_header.clone(),
                        data: chunk_data,
                    });
                }
                b"data" => {
                    data_chunk = Some(DataChunk {
                        header: chunk_header.clone(),
                        data: chunk_data,
                    });
                    // Data chunk should be last in valid WAV files, but we'll continue parsing anyway
                }
                _ => {
                    additional_chunks.push(RiffChunk {
                        header: chunk_header.clone(),
                        data: chunk_data,
                    });
                }
            }

            // Move to next chunk (chunk size is padded to even bytes)
            offset = chunk_data_end + ((chunk_header.data_size % 2) as usize);
        }

        let fmt_chunk = fmt_chunk.ok_or_else(|| PolyglotError::ChunkNotFound("fmt ".to_string()))?;
        let data_chunk = data_chunk.ok_or_else(|| PolyglotError::ChunkNotFound("data".to_string()))?;

        Ok(RiffStructure {
            header,
            fmt_chunk,
            data_chunk,
            additional_chunks,
        })
    }

    /// Parse a chunk header from data
    fn parse_chunk_header(data: &[u8]) -> PolyglotResult<RiffChunkHeader> {
        if data.len() < 8 {
            return Err(PolyglotError::WavParse("Insufficient data for chunk header".to_string()));
        }

        let mut fourcc = [0u8; 4];
        fourcc.copy_from_slice(&data[0..4]);
        let data_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        Ok(RiffChunkHeader { fourcc, data_size })
    }

    /// Rebuild raw bytes from structure
    pub fn to_bytes(&self) -> PolyglotResult<Vec<u8>> {
        let mut result = Vec::new();

        // RIFF header
        result.extend_from_slice(RIFF_SIGNATURE);
        result.extend_from_slice(&self.header.file_size.to_le_bytes());
        result.extend_from_slice(WAVE_SIGNATURE);

        // fmt chunk
        Self::write_chunk(&mut result, &self.fmt_chunk.header, &self.fmt_chunk.data);

        // data chunk
        Self::write_chunk(&mut result, &self.data_chunk.header, &self.data_chunk.data);

        // Additional chunks
        for chunk in &self.additional_chunks {
            Self::write_chunk(&mut result, &chunk.header, &chunk.data);
        }

        Ok(result)
    }

    /// Write a chunk to the output buffer
    fn write_chunk(output: &mut Vec<u8>, header: &RiffChunkHeader, data: &[u8]) {
        output.extend_from_slice(&header.fourcc);
        output.extend_from_slice(&header.data_size.to_le_bytes());
        output.extend_from_slice(data);

        // RIFF chunks are padded to even byte boundaries
        if header.data_size % 2 == 1 {
            output.push(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_wav() -> Vec<u8> {
        let mut wav = vec![];

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(40u32).to_le_bytes()); // File size = total - 8 bytes for RIFF header/WAVE signature (48 - 8 = 40)
        wav.extend_from_slice(b"WAVE");

        // fmt chunk (16 bytes)
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&(16u32).to_le_bytes()); // Chunk size
        wav.extend_from_slice(&(1u16).to_le_bytes()); // Audio format (PCM)
        wav.extend_from_slice(&(1u16).to_le_bytes()); // Channels
        wav.extend_from_slice(&(44100u32).to_le_bytes()); // Sample rate
        wav.extend_from_slice(&(88200u32).to_le_bytes()); // Byte rate
        wav.extend_from_slice(&(2u16).to_le_bytes()); // Block align
        wav.extend_from_slice(&(16u16).to_le_bytes()); // Bits per sample

        // data chunk (minimal audio data)
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(4u32).to_le_bytes()); // Data size
        wav.extend_from_slice(&(0u16).to_le_bytes()); // Minimal audio data
        wav.extend_from_slice(&(0u16).to_le_bytes());

        wav
    }

    fn create_test_png() -> Vec<u8> {
        let mut png = vec![];

        // PNG signature
        png.extend_from_slice(b"\x89PNG\r\n\x1A\n");

        // IHDR chunk
        png.extend_from_slice(&(13u32).to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&(1u32).to_be_bytes()); // width = 1
        png.extend_from_slice(&(1u32).to_be_bytes()); // height = 1
        png.push(8); // bit depth = 8
        png.push(2); // color type = RGB
        png.push(0); // compression = 0
        png.push(0); // filter = 0
        png.push(0); // interlace = 0
        let ihdr_data = &png[12..29]; // From IHDR to end of data
        let ihdr_crc = crate::utils::calculate_crc32(&[b"IHDR".as_slice(), ihdr_data].concat());
        png.extend_from_slice(&ihdr_crc.to_be_bytes());

        // IDAT chunk (minimal compressed data)
        let idat_data = [0x78, 0x9C, 0x62, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01]; // zlib compressed 1x1 RGB
        png.extend_from_slice(&(idat_data.len() as u32).to_be_bytes());
        png.extend_from_slice(b"IDAT");
        png.extend_from_slice(&idat_data);
        let idat_crc = crate::utils::calculate_crc32(&[b"IDAT".as_slice(), &idat_data].concat());
        png.extend_from_slice(&idat_crc.to_be_bytes());

        // IEND chunk
        png.extend_from_slice(&(0u32).to_be_bytes());
        png.extend_from_slice(b"IEND");
        let iend_crc = crate::utils::calculate_crc32(b"IEND");
        png.extend_from_slice(&iend_crc.to_be_bytes());

        png
    }

    #[test]
    fn test_riff_signature_validation() {
        // Test invalid file path
        let result = WavFile::from_file(Path::new("nonexistent.wav"));
        assert!(matches!(result, Err(PolyglotError::InputFile(_))));

        // Valid RIFF header but no chunks
        let short_data = b"RIFF....WAVE".to_vec();
        let result = RiffStructure::parse(&short_data);
        assert!(matches!(result, Err(PolyglotError::ChunkNotFound(_))));

        // Invalid RIFF signature
        let invalid_riff = b"XXXX....WAVE....".to_vec();
        let result = WavFile::from_data(invalid_riff);
        assert!(matches!(result, Err(PolyglotError::InvalidRiffHeader)));
    }

    #[test]
    fn test_png_embedding_and_extraction() {
        let wav_data = create_test_wav();
        let png_data = create_test_png();

        // Load WAV
        let mut wav_file = WavFile::from_data(wav_data.clone()).unwrap();

        // Verify initial state - no PNG chunk
        assert!(wav_file.extract_png_data().is_none());
        assert_eq!(wav_file.structure.additional_chunks.len(), 0);

        // Embed PNG data
        wav_file.embed_png_data(&png_data).unwrap();

        // Verify PNG chunk was added
        assert_eq!(wav_file.structure.additional_chunks.len(), 1);
        assert_eq!(wav_file.structure.additional_chunks[0].header.fourcc, PNG_CHUNK_FOURCC);

        // Extract PNG data
        let extracted_png = wav_file.extract_png_data().unwrap();
        assert_eq!(extracted_png, png_data);

        // Verify WAV structure is still valid by reparsing
        let reparsed = WavFile::from_data(wav_file.raw_data.clone()).unwrap();
        let extracted_again = reparsed.extract_png_data().unwrap();
        assert_eq!(extracted_again, png_data);
    }

    #[test]
    fn test_polyglot_file_size() {
        let wav_data = create_test_wav();
        let png_data = create_test_png();

        let original_size = wav_data.len();
        let mut wav_file = WavFile::from_data(wav_data).unwrap();
        wav_file.embed_png_data(&png_data).unwrap();

        // File should be larger: png_data + 8 bytes for chunk header + 1 byte padding if data length is odd
        let mut expected_additional_size = png_data.len() + 8; // 8 = 4-byte FOURCC + 4-byte size
        if png_data.len() % 2 == 1 {
            expected_additional_size += 1; // RIFF padding byte
        }

        // Check total size
        assert_eq!(wav_file.raw_data.len(), original_size + expected_additional_size);

        // RIFF header file size should be updated (total_size - 8 for header)
        let reported_size = u32::from_le_bytes([wav_file.raw_data[4], wav_file.raw_data[5], wav_file.raw_data[6], wav_file.raw_data[7]]);
        assert_eq!(reported_size as usize, original_size + expected_additional_size - 8);
    }

    #[test]
    fn test_wav_still_valid_after_embedding() {
        use hound::WavReader;

        let wav_data = create_test_wav();
        let png_data = create_test_png();

        let mut wav_file = WavFile::from_data(wav_data).unwrap();
        wav_file.embed_png_data(&png_data).unwrap();

        // Should still be readable as WAV
        let cursor = std::io::Cursor::new(&wav_file.raw_data);
        let reader = WavReader::new(cursor).unwrap();
        let spec = reader.spec();

        // Basic format validation
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 44100);
        assert_eq!(spec.bits_per_sample, 16);
    }

    #[test]
    fn test_size_overflow_prevention() {
        let wav_data = create_test_wav();
        let large_png = vec![0u8; (u32::MAX as usize) - 7]; // Would cause overflow

        let mut wav_file = WavFile::from_data(wav_data).unwrap();
        let result = wav_file.embed_png_data(&large_png);
        assert!(matches!(result, Err(PolyglotError::SizeOverflow)));
    }
}

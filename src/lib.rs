//! # PNG/ZIP Polyglot Tool
//!
//! This library provides functionality to create and manipulate PNG/ZIP polyglots -
//! files that are valid in both PNG and ZIP formats simultaneously.
//!
//! The core concept is embedding ZIP archive data into the PNG's IDAT chunk
//! while maintaining valid checksums and offsets in both formats.

// Public API exports
pub mod cli;
pub mod png;
pub mod gif;
pub mod flac;
pub mod wav;
pub mod zip;
pub mod polyglot;
pub mod utils;
pub mod extract;

pub use polyglot::{PolyglotCreator, create_png_wav_polyglot, create_png_flac_polyglot};
pub use extract::{validate_polyglot, extract_zip_from_png, extract_wav_from_png};

/// Result type alias for polyglot operations
pub type PolyglotResult<T> = Result<T, PolyglotError>;

/// Comprehensive error type for the polyglot tool
#[derive(Debug, thiserror::Error)]
pub enum PolyglotError {
    #[error("PNG parse error: {0}")]
    PngParse(String),

    #[error("ZIP parse error: {0}")]
    ZipParse(String),

    #[error("WAV parse error: {0}")]
    WavParse(String),

    #[error("CRC mismatch in chunk {0}")]
    CrcMismatch(String),

    #[error("No IDAT chunk found")]
    NoIdatChunk,

    #[error("Invalid RIFF header")]
    InvalidRiffHeader,

    #[error("Chunk not found: {0}")]
    ChunkNotFound(String),

    #[error("Size overflow in RIFF file")]
    SizeOverflow,

    #[error("Input file error: {0}")]
    InputFile(#[from] std::io::Error),

    #[error("Polyglot creation failed: {0}")]
    CreationFailed(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl PolyglotError {
    /// Validate that a PNG chunk's CRC matches expected value
    pub fn validate_png_chunk(chunk_type: &[u8; 4], expected_crc: u32, actual_crc: u32)
        -> PolyglotResult<()> {
        if expected_crc != actual_crc {
            let chunk_str = String::from_utf8_lossy(chunk_type);
            Err(PolyglotError::CrcMismatch(chunk_str.to_string()))
        } else {
            Ok(())
        }
    }
}

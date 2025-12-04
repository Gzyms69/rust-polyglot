//! GIF file format support for parasitic polyglots

use std::path::Path;
use std::fs;
use crate::{PolyglotError, PolyglotResult};

/// GIF file representation
#[derive(Debug, Clone)]
pub struct GifFile {
    pub raw_data: Vec<u8>,
    pub parsed: GifStructure,
}

/// Parsed GIF structure
#[derive(Debug, Clone)]
pub struct GifStructure {
    pub header: GifHeader,
    pub global_color_table: Option<Vec<u8>>,
    pub blocks: Vec<GifBlock>,
}

/// GIF header (6 bytes)
#[derive(Debug, Clone)]
pub struct GifHeader {
    pub signature: [u8; 3], // "GIF"
    pub version: [u8; 3],   // "89a" or "87a"
}

/// GIF blocks (simplified)
#[derive(Debug, Clone)]
pub enum GifBlock {
    ImageDescriptor(Vec<u8>),
    GraphicControlExtension(Vec<u8>),
    CommentExtension(Vec<u8>),
    PlainTextExtension(Vec<u8>),
    ApplicationExtension(Vec<u8>),
    Unknown(Vec<u8>),
}

impl GifFile {
    /// Load GIF file from path
    pub fn from_file(path: &Path) -> PolyglotResult<Self> {
        let raw_data = fs::read(path)?;
        
        if raw_data.len() < 6 {
            return Err(PolyglotError::PngParse("File too short for GIF".to_string())); // reusing error type
        }
        
        if &raw_data[0..3] != b"GIF" {
            return Err(PolyglotError::PngParse("Invalid GIF signature".to_string()));
        }
        
        // Basic structure parsing would go here
        let parsed = GifStructure::parse(&raw_data)?;
        
        Ok(Self { raw_data, parsed })
    }
    
    /// Add ZIP data embedded in a comment extension (parasitic)
    pub fn add_zip_comment_extension(&mut self, zip_data: &[u8]) -> PolyglotResult<()> {
        // Embed ZIP data in GIF comment extension
        // This is similar to PNG text chunks but using GIF comment blocks
        let mut comment_data = Vec::new();
        comment_data.extend_from_slice(b"ZIP_ARCHIVE:");
        comment_data.extend_from_slice(zip_data);
        
        // Build comment extension: 0x21 0xFE + length + data + 0x00
        let mut extension = vec![0x21, 0xFE]; // Comment extension introducer
        
        // Add comment sub-blocks
        let mut remaining = &comment_data[..];
        while remaining.len() > 255 {
            extension.push(255);
            extension.extend_from_slice(&remaining[0..255]);
            remaining = &remaining[255..];
        }
        if !remaining.is_empty() {
            extension.push(remaining.len() as u8);
            extension.extend_from_slice(remaining);
        }
        extension.push(0x00); // End of extension
        
        // Insert before trailer (0x3B)
        if let Some(trailer_pos) = self.raw_data.iter().position(|&b| b == 0x3B) {
            let mut new_data = self.raw_data[0..trailer_pos].to_vec();
            new_data.extend_from_slice(&extension);
            new_data.push(0x3B); // Trailer
            self.raw_data = new_data;
        }
        
        Ok(())
    }
    
    /// Write modified GIF to file
    pub fn write_to_file(&self, path: &Path) -> PolyglotResult<()> {
        fs::write(path, &self.raw_data)?;
        Ok(())
    }
    
    /// Get raw data
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw_data
    }
}

impl GifStructure {
    pub fn parse(data: &[u8]) -> PolyglotResult<Self> {
        // Simplified GIF parsing - just extract header for now
        let header = GifHeader {
            signature: [data[0], data[1], data[2]],
            version: [data[3], data[4], data[5]],
        };
        
        Ok(Self {
            header,
            global_color_table: None, // Would parse LSD and GCT properly
            blocks: Vec::new(),        // Would parse all blocks
        })
    }
}

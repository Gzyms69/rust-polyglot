//! FLAC file format support for parasitic polyglots (PNG in PADDING blocks)

use std::path::Path;
use std::fs;
use crate::{PolyglotError, PolyglotResult};

/// FLAC file signature
const FLAC_SIGNATURE: &[u8; 4] = b"fLaC";

/// FLAC metadata block types
#[derive(Debug, Clone)]
pub enum MetadataBlock {
    StreamInfo(StreamInfo),
    Padding { length: u32, data: Vec<u8> },
    Application { id: [u8; 4], data: Vec<u8> },
    SeekTable(Vec<u8>),
    VorbisComment(Vec<u8>),
    Cuesheet(Vec<u8>),
    Picture(Vec<u8>),
    Unknown { block_type: u8, length: u32, data: Vec<u8> },
}

/// Parsed FLAC structure
#[derive(Debug, Clone)]
pub struct FlacStructure {
    pub streaminfo: StreamInfo,
    pub metadata_blocks: Vec<MetadataBlock>,
}

/// STREAMINFO block (always the first metadata block)
#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub min_block_size: u16,
    pub max_block_size: u16,
    pub min_frame_size: u32,
    pub max_frame_size: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub bits_per_sample: u8,
    pub total_samples: u64,
    pub md5_signature: [u8; 16],
}

/// FLAC file handler for parasitic polyglots
#[derive(Debug, Clone)]
pub struct FlacFile {
    pub raw_data: Vec<u8>,
    pub structure: FlacStructure,
}

impl FlacFile {
    /// Load FLAC file from path
    pub fn from_file(path: &Path) -> PolyglotResult<Self> {
        let raw_data = fs::read(path)?;
        
        if raw_data.len() < 8 {
            return Err(PolyglotError::PngParse("File too short for FLAC".to_string()));
        }
        
        if &raw_data[0..4] != FLAC_SIGNATURE {
            return Err(PolyglotError::PngParse("Invalid FLAC signature".to_string()));
        }
        
        let structure = FlacStructure::parse(&raw_data)?;
        
        Ok(Self { raw_data, structure })
    }
    
    /// Inject PNG data into PADDING metadata blocks (parasitic embedding)
    pub fn inject_png_to_padding(&mut self, png_data: &[u8]) -> PolyglotResult<()> {
        // Find a PADDING block large enough, or find one to expand
        let (block_idx, padding_block) = self.find_or_create_padding_for_png(png_data.len())?;
        
        if let MetadataBlock::Padding { length, data: _ } = padding_block {
            // Replace the PADDING block content with PNG data
            self.replace_padding_content(block_idx, png_data)?;
        }
        
        // Rebuild the raw data with updated metadata block
        self.raw_data = self.structure.to_bytes()?;
        
        Ok(())
    }
    
    /// Find existing PADDING block large enough for PNG, or create/enlarge one
    fn find_or_create_padding_for_png(&self, png_size: usize) -> PolyglotResult<(usize, &MetadataBlock)> {
        // Look for existing PADDING blocks
        for (i, block) in self.structure.metadata_blocks.iter().enumerate() {
            if let MetadataBlock::Padding { length, .. } = block {
                if *length >= png_size as u32 {
                    return Ok((i, block));
                }
            }
        }
        
        // No suitable PADDING block found - would need to add one
        // For now, require the FLAC to already have a suitable PADDING block
        Err(PolyglotError::InvalidInput(
            format!("No PADDING block large enough for PNG data ({} bytes) found in FLAC file", png_size)
        ))
    }
    
    /// Replace the content of a PADDING block with PNG data
    fn replace_padding_content(&mut self, block_idx: usize, png_data: &[u8]) -> PolyglotResult<()> {
        if let Some(block) = self.structure.metadata_blocks.get_mut(block_idx) {
            if let MetadataBlock::Padding { length, data } = block {
                // Replace the padding data with PNG data (up to the block capacity)
                let copy_len = (*length as usize).min(png_data.len());
                *data = png_data[0..copy_len].to_vec();
                
                // Pad with zeros if PNG is smaller than block
                data.resize(*length as usize, 0);
            }
        }
        
        Ok(())
    }
    
    /// Write modified FLAC to file
    pub fn write_to_file(&self, path: &Path) -> PolyglotResult<()> {
        fs::write(path, &self.raw_data)?;
        Ok(())
    }
    
    /// Get raw data
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw_data
    }
}

impl FlacStructure {
    pub fn parse(data: &[u8]) -> PolyglotResult<Self> {
        let mut offset = 4; // Skip "fLaC" signature
        
        // Parse STREAMINFO (first and mandatory block)
        let (streaminfo, new_offset) = StreamInfo::parse(data, offset)?;
        offset = new_offset;
        
        let mut metadata_blocks = vec![MetadataBlock::StreamInfo(streaminfo.clone())];
        
        // Parse remaining metadata blocks until we hit a data frame
        while offset < data.len() {
            let is_last = (data[offset] & 0x80) != 0;
            let (block, new_offset) = Self::parse_metadata_block(data, offset)?;
            metadata_blocks.push(block);
            offset = new_offset;

            if is_last {
                // Last metadata block, parsing would continue for frames...
                break;
            }
        }
        
        Ok(FlacStructure { streaminfo, metadata_blocks })
    }
    
    fn parse_metadata_block(data: &[u8], offset: usize) -> PolyglotResult<(MetadataBlock, usize)> {
        let block_type = data[offset] & 0x7F;
        let length = u32::from_be_bytes([data[offset + 1], data[offset + 2], data[offset + 3], data[offset + 4]]);
        let data_start = offset + 4;
        let data_end = data_start + length as usize;
        
        let block_data = data[data_start..data_end].to_vec();
        
        let block = match block_type {
            0 => (MetadataBlock::StreamInfo(StreamInfo::parse_from_data(&block_data)?), data_end),
            1 => (MetadataBlock::Padding { length, data: block_data }, data_end),
            2 => {
                if block_data.len() >= 4 {
                    let mut id = [0u8; 4];
                    id.copy_from_slice(&block_data[0..4]);
                    (MetadataBlock::Application { id, data: block_data }, data_end)
                } else {
                    (MetadataBlock::Unknown { block_type, length, data: block_data }, data_end)
                }
            },
            3 => (MetadataBlock::SeekTable(block_data), data_end),
            4 => (MetadataBlock::VorbisComment(block_data), data_end),
            6 => (MetadataBlock::Picture(block_data), data_end),
            _ => (MetadataBlock::Unknown { block_type, length, data: block_data }, data_end),
        };
        
        Ok(block)
    }
    
    pub fn to_bytes(&self) -> PolyglotResult<Vec<u8>> {
        let mut result = FLAC_SIGNATURE.to_vec();
        
        // Write all metadata blocks
        for block in &self.metadata_blocks {
            Self::write_metadata_block(block, &mut result)?;
        }
        
        // Would need to write frames for complete implementation
        // For now, assume metadata-only operations
        
        Ok(result)
    }
    
    fn write_metadata_block(block: &MetadataBlock, output: &mut Vec<u8>) -> PolyglotResult<()> {
        match block {
            MetadataBlock::Padding { data, .. } => {
                let length = data.len() as u32;
                output.push(1); // PADDING type
                output.extend_from_slice(&length.to_be_bytes());
                output.extend_from_slice(data);
            }
            MetadataBlock::StreamInfo(streaminfo) => {
                output.push(0); // STREAMINFO type
                streaminfo.write_to(output)?;
            }
            // Other block types would be implemented here
            _ => {
                // Placeholder - would implement full serialization
                return Err(PolyglotError::InvalidInput("Block serialization not implemented".to_string()));
            }
        }
        
        Ok(())
    }
}

impl StreamInfo {
    pub fn parse(data: &[u8], offset: usize) -> PolyglotResult<(StreamInfo, usize)> {
        let block_start = offset + 4; // Skip block header
        let streaminfo_data = &data[block_start..block_start + 34];
        
        Self::parse_from_data(streaminfo_data).map(|si| (si, block_start + 34))
    }
    
    pub fn parse_from_data(data: &[u8]) -> PolyglotResult<StreamInfo> {
        if data.len() < 34 {
            return Err(PolyglotError::PngParse("STREAMINFO data too short".to_string()));
        }
        
        let min_block_size = u16::from_be_bytes([data[0], data[1]]);
        let max_block_size = u16::from_be_bytes([data[2], data[3]]);
        let min_frame_size = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let max_frame_size = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        
        let sample_rate_20 = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
        let sample_rate = (sample_rate_20 >> 12) & 0xFFFFF;
        let channels = (((sample_rate_20 >> 9) & 0x07) + 1) as u8;
        let bits_per_sample = (((sample_rate_20 >> 4) & 0x1F) + 1) as u8;
        
        let total_samples_36 = u64::from_be_bytes([data[16], data[17], data[18], data[19], 
                                                  data[20], data[21], data[22], data[23]]);
        let total_samples = total_samples_36 & 0xFFFFFFFFF;
        
        let mut md5_signature = [0u8; 16];
        md5_signature.copy_from_slice(&data[18..34]);
        
        Ok(StreamInfo {
            min_block_size,
            max_block_size, 
            min_frame_size,
            max_frame_size,
            sample_rate,
            channels,
            bits_per_sample,
            total_samples,
            md5_signature,
        })
    }
    
    pub fn write_to(&self, output: &mut Vec<u8>) -> PolyglotResult<()> {
        // Would implement STREAMINFO serialization
        Err(PolyglotError::InvalidInput("STREAMINFO serialization not implemented yet".to_string()))
    }
}

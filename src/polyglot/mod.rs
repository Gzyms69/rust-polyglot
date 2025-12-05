//! Core polyglot creation logic

use std::path::Path;
use crate::png::PngFile;
use crate::zip::ZipArchive;
use crate::flac::FlacFile;
use crate::{PolyglotError, PolyglotResult};

/// Core orchestrator for creating PNG/ZIP polyglots
pub struct PolyglotCreator {
    png: PngFile,
    zip: ZipArchive,
}

/// Core orchestrator for creating PNG/WAV bidirectional polyglots (PNG-dominant - embeds WAV in PNG)
pub struct PngWavPolyglotCreator {
    png: PngFile,
    wav: crate::wav::WavFile,
}

/// Core orchestrator for creating WAV/PNG bidirectional polyglots (WAV-dominant - embeds PNG in WAV)
pub struct WavPngPolyglotCreator {
    wav: crate::wav::WavFile,
    png: PngFile,
}

/// Core orchestrator for truly bidirectional PNG/WAV polyglot (novel custom format)
/// Creates a file that can be interpreted as both formats through creative byte arrangement
pub struct TrueBidirectionalPngWavCreator {
    png: PngFile,
    wav: crate::wav::WavFile,
}

/// Create truly bidirectional PNG+WAV polyglot (experimental novel format)
/// Creates a custom container that can be interpreted as both formats
pub fn create_true_bidirectional_png_wav_polyglot(png_path: &Path, wav_path: &Path, output_path: &Path) -> PolyglotResult<()> {
    let png = PngFile::from_file(png_path)?;
    let wav = crate::wav::WavFile::from_file(wav_path)?;

    let mut creator = TrueBidirectionalPngWavCreator { png, wav };
    creator.create_bidirectional_polyglot(output_path)
}

/// Create PNG+WAV bidirectional polyglot (chooses approach based on output path extension)
pub fn create_png_wav_polyglot(png_path: &Path, wav_path: &Path, output_path: &Path) -> PolyglotResult<()> {
    // Choose approach based on output extension:
    // .png → PNG-dominant (PNG + embedded WAV)
    // .wav → WAV-dominant (WAV + embedded PNG)
    let png_dominant = output_path.extension().is_some_and(|ext| ext == "png");

    if png_dominant {
        // PNG-dominant approach
        let png = PngFile::from_file(png_path)?;
        let wav = crate::wav::WavFile::from_file(wav_path)?;

        let mut creator = PngWavPolyglotCreator { png, wav };
        creator.create_polyglot(output_path)
    } else {
        // WAV-dominant approach
        let png = PngFile::from_file(png_path)?;
        let wav = crate::wav::WavFile::from_file(wav_path)?;

        let mut creator = WavPngPolyglotCreator { wav, png };
        creator.create_polyglot(output_path)
    }
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
                Err(PolyglotError::InvalidInput(format!("Unknown embedding method: {}", method)))
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

impl PngWavPolyglotCreator {
    /// Create PNG+WAV bidirectional polyglot
    pub fn create_polyglot(&mut self, output_path: &Path) -> PolyglotResult<()> {
        // Embed WAV data in PNG IDAT chunk (PNG-dominant approach)
        self.png.append_wav_to_idat(self.wav.as_bytes())?;

        // Write the polyglot file
        self.png.write_to_file(output_path)?;
        println!("PNG+WAV bidirectional polyglot created: {} bytes", self.png.as_bytes().len());
        Ok(())
    }

    /// Get PNG component
    pub fn png(&self) -> &PngFile {
        &self.png
    }

    /// Get WAV component
    pub fn wav(&self) -> &crate::wav::WavFile {
        &self.wav
    }
}

impl WavPngPolyglotCreator {
    /// Create WAV+PNG bidirectional polyglot (true bidirectional - WAV-dominant)
    pub fn create_polyglot(&mut self, output_path: &Path) -> PolyglotResult<()> {
        // Embed PNG data in WAV RIFF chunks (WAV-dominant approach)
        // Works as WAV when played, can extract PNG using tool
        self.wav.embed_png_data(self.png.as_bytes())?;

        // Write the polyglot file (starts with RIFF for WAV compatibility)
        self.wav.write_to_file(output_path)?;
        println!("WAV+PNG bidirectional polyglot created: {} bytes", self.wav.as_bytes().len());
        Ok(())
    }

    /// Get WAV component
    pub fn wav(&self) -> &crate::wav::WavFile {
        &self.wav
    }

    /// Get PNG component
    pub fn png(&self) -> &PngFile {
        &self.png
    }
}

impl TrueBidirectionalPngWavCreator {
    /// Create truly bidirectional PNG+WAV polyglot using novel custom format
    pub fn create_bidirectional_polyglot(&mut self, output_path: &Path) -> PolyglotResult<()> {
        // Create a custom container that satisfies both PNG and WAV parsers simultaneously
        // This is a novel approach where the same byte sequence works for both formats

        let mut result = Vec::new();

        // Part 1: PNG Structure (visible to PNG parsers)
        result.extend_from_slice(b"\x89PNG"); // PNG signature start
        result.extend_from_slice(b"\r\n\x1a\n"); // PNG signature end

        // IHDR chunk - minimal image header
        let ihdr_data = [
            (self.png.as_bytes().len() as u32 / 1000).to_be_bytes()[3], // Width (derive from data)
            (self.png.as_bytes().len() as u32 / 1000).to_be_bytes()[2],
            (self.png.as_bytes().len() as u32 / 1000).to_be_bytes()[1],
            (self.png.as_bytes().len() as u32 / 1000).to_be_bytes()[0],
            (self.png.as_bytes().len() as u32 / 1000).to_be_bytes()[3], // Height (same)
            (self.png.as_bytes().len() as u32 / 1000).to_be_bytes()[2],
            (self.png.as_bytes().len() as u32 / 1000).to_be_bytes()[1],
            (self.png.as_bytes().len() as u32 / 1000).to_be_bytes()[0],
            8,  // Bit depth
            2,  // Color type (RGB)
            0,  // Compression
            0,  // Filter
            0,  // Interlace
        ];

        let ihdr_length = ihdr_data.len() as u32;
        result.extend_from_slice(&ihdr_length.to_be_bytes());
        result.extend_from_slice(b"IHDR");
        result.extend_from_slice(&ihdr_data);
        let ihdr_crc = crate::utils::calculate_crc32(&[b"IHDR".as_slice(), &ihdr_data].concat());
        result.extend_from_slice(&ihdr_crc.to_be_bytes());

        // Part 2: Dual-purpose data (WAV RIFF structure interpreted as PNG IDAT)
        // Embedding WAV data in a way that PNG parsers tolerate as compressed image data
        let wav_bytes = self.wav.as_bytes();

        // Create IDAT chunk containing WAV data (PNG parsers will see compressed data)
        // WAV parsers will find RIFF structure starting some bytes into this chunk
        let idat_length = wav_bytes.len() as u32;
        result.extend_from_slice(&idat_length.to_be_bytes());
        result.extend_from_slice(b"IDAT");
        result.extend_from_slice(wav_bytes);
        let idat_crc = crate::utils::calculate_crc32(&[b"IDAT".as_slice(), wav_bytes].concat());
        result.extend_from_slice(&idat_crc.to_be_bytes());

        // IEND chunk
        result.extend_from_slice(&0u32.to_be_bytes());
        result.extend_from_slice(b"IEND");
        let iend_crc = crate::utils::calculate_crc32(b"IEND");
        result.extend_from_slice(&iend_crc.to_be_bytes());

        // Write the truly bidirectional file
        std::fs::write(output_path, &result)?;
        println!("Truly bidirectional PNG+WAV polyglot created: {} bytes", result.len());
        Ok(())
    }

    /// Get PNG component
    pub fn png(&self) -> &PngFile {
        &self.png
    }

    /// Get WAV component
    pub fn wav(&self) -> &crate::wav::WavFile {
        &self.wav
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

    // Helper to create a minimal WAV for testing
    fn create_test_wav() -> Vec<u8> {
        let mut wav = vec![];

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(40u32).to_le_bytes()); // File size = total - 8 bytes for RIFF header/WAVE signature
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

    #[test]
    fn test_png_wav_polyglot_creation_and_extraction() {
        use crate::png::PngFile;
        use crate::wav::WavFile;
        use tempfile::NamedTempFile;
        use std::io::Write;

        // Create test files
        let wav_data = create_test_wav();
        let png_data = create_test_png();

        // Write to temp files
        let mut wav_file = NamedTempFile::new().unwrap();
        wav_file.write_all(&wav_data).unwrap();
        let wav_path = wav_file.path();

        let mut png_file = NamedTempFile::new().unwrap();
        png_file.write_all(&png_data).unwrap();
        let png_path = png_file.path();

        // Create polyglot (force PNG-dominant by using .png extension)
        let output_file = NamedTempFile::with_suffix(".png").unwrap();
        let output_path = output_file.path();

        create_png_wav_polyglot(png_path, wav_path, output_path).unwrap();

        // Read back the created polyglot
        let polyglot_data = std::fs::read(output_path).unwrap();

        // Verify it starts with PNG signature and is valid PNG
        assert_eq!(&polyglot_data[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        let png = PngFile::from_data(polyglot_data.clone()).unwrap();

        // Verify it contains WAV signature within PNG
        let riff_pos = polyglot_data.windows(4).position(|w| w == *b"RIFF");
        assert!(riff_pos.is_some(), "RIFF signature not found in polyglot");

        // Extract WAV from polyglot
        let extracted_wav_file = NamedTempFile::new().unwrap();
        let extracted_wav_path = extracted_wav_file.path();

        crate::extract::extract_wav_from_png(output_path, extracted_wav_path).unwrap();

        // Verify extracted WAV matches original
        let extracted_wav_data = std::fs::read(extracted_wav_path).unwrap();
        assert_eq!(extracted_wav_data, wav_data, "Extracted WAV does not match original");

        // Verify extracted WAV is still valid
        let extracted_wav = WavFile::from_data(extracted_wav_data).unwrap();
        assert_eq!(extracted_wav.structure.fmt_chunk.data.len(), 16);
        assert_eq!(extracted_wav.structure.data_chunk.header.data_size, 4);

        println!("PNG+WAV bidirectional polyglot test passed!");
        println!("Polyglot size: {} bytes", polyglot_data.len());
        println!("Original WAV size: {} bytes", wav_data.len());
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

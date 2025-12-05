# Polyglot Creator - Developer Guide

## Development Setup

### Prerequisites
- Rust toolchain (stable recommended)
- Git

### Building
```bash
# Debug build
cargo build

# Release build with optimizations
cargo build --release

# Check for errors without building
cargo check
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_polyglot_creation

# Run with verbose output
cargo test -- --nocapture
```

### Code Quality
```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Clean build artifacts
cargo clean
```

## Project Architecture

### Overview
Rust implementation of polyglot file creation - files valid in multiple formats simultaneously. Currently supports:
- **PNG+ZIP polyglots (full support)**: PNG images that function as ZIP archives - creation, validation, and extraction
- **PNG+WAV polyglots (partial support)**: PNG images with embedded WAV audio data - creation and extraction work, but validation only supports PNG+ZIP format
- **True bidirectional PNG+WAV files (experimental)**: Custom format intended to work as both formats simultaneously

### Core Components

**cli/ - Command-line interface**
- Argument parsing with `clap`
- Input validation and user interaction
- Output formatting

**png/ - PNG format handling**
- `parser.rs`: Low-level PNG chunk parsing and CRC verification
- `mod.rs`: High-level PNG manipulation and chunk modification

**zip/ - ZIP archive handling**
- `mod.rs`: ZIP file parsing and manipulation
- `offsets.rs`: Central directory offset calculations

**wav/ - WAV audio format handling**
- `mod.rs`: WAV file parsing, RIFF header validation, and data extraction

**polyglot/ - Core logic**
- `PolyglotCreator`: Orchestrates format combination
- Multiple embedding strategies
- Result validation

**extract/ - Polyglot processing**
- Extract embedded content from polyglots
- Format validation and integrity checking

**utils/ - Shared utilities**
- CRC32 calculation
- Endian conversions
- Format signatures

### Key Types

- `PolyglotCreator`: Main orchestration struct
- `PngFile`: PNG manipulation wrapper
- `ZipArchive`: ZIP data management
- `WavFile`: WAV audio file parsing and manipulation
- `PolyglotError`: Comprehensive error types
- `ValidationResult`: Polyglot integrity results

## Embedding Methods

### Text Embedding (DEFAULT)
- Data stored in PNG tEXt chunks (metadata)
- PNG viewers work normally
- ZIP/WAV accessible by renaming extension or extraction
- **Recommended for PNG+ZIP polyglots**

### Container Method (ZIP-dominant)
- PNG file embedded within ZIP archive
- ZIP tools work normally
- PNG extraction possible
- **Container approach for ZIP-dominant polyglots**

### Bidirectional Method
- True bidirectional embedding for PNG+WAV
- File works as both PNG image and WAV audio simultaneously
- No format dominance - both formats equally valid
- **Advanced format for seamless dual functionality**

### IDAT Method (BROKEN)
- Data in PNG image data chunks (corrupts IDAT compression)
- PNG display works
- Embedded data structure corrupted
- **Don't use - fundamentally broken**

## Development Guidelines

### Code Style
```rust
// Use module-level imports, grouped by source
use std::{fs, path::Path};
use crate::{polyglot::PolyglotCreator, PolyglotError};

// Doc comments for public APIs
/// Creates a new polyglot from PNG and ZIP files
pub fn create_polyglot(...) -> Result<(), Error> {
    // Implementation
}
```

### Error Handling
- Use `Result<T, PolyglotError>` for all public APIs
- Provide specific error variants for different failure modes
- Use `thiserror` for automatic error derivation

### Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_polyglot_creation() {
        // Arrange
        let png = create_test_png();
        let zip = create_test_zip();

        // Act
        let result = PolyglotCreator::from_data(png, zip);

        // Assert
        assert!(result.is_ok());
    }
}
```

### Performance Considerations
- Load entire files into memory (acceptable for current use case)
- Minimal allocations during processing
- CRC calculations integrated into workflows

## Contributing

### Adding New Formats
1. Create format module with parsing capabilities
2. Implement embedding strategies
3. Add CLI integration
4. Write comprehensive tests

### Adding Embedding Methods
1. Extend `PolyglotCreator` with new method logic
2. Add CLI option for method selection
3. Test both format validities
4. Document method characteristics

### Bug Fixes
1. Isolate the issue with minimal test case
2. Fix the root cause
3. Add regression test
4. Update documentation

## Known Issues & Limitations

- IDAT embedding broken due to compression interference
- PNG+WAV polyglots: creation and extraction work, but validation only supports PNG+ZIP format
- No support for compressed ZIP content
- Limited format validation
- Basic error messages
- Not optimized for very large files

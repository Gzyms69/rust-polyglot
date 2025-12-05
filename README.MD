# Polyglot File Creator

A command-line tool for creating polyglot files - files that are valid in multiple formats simultaneously.

Currently supports:
- **PNG+ZIP polyglots**: PNG images that function as ZIP archives
- **PNG+WAV polyglots**: PNG images containing embedded WAV audio data
- **Bidirectional polyglots**: True bidirectional PNG+WAV files

## Quick Start

```bash
# Install Rust and clone repository
cargo build --release

# Create a PNG polyglot containing ZIP data
./target/release/rust-polyglot create --png image.png --zip archive.zip --output polyglot.png

# Create a PNG+WAV bidirectional polyglot
./target/release/rust-polyglot create --png image.png --wav audio.wav --output media.png

# Create truly bidirectional PNG+WAV (works as both formats simultaneously)
./target/release/rust-polyglot create --png image.png --wav audio.wav --output bidirectional.png --method bidirectional

# Use as PNG image (opens normally in viewers)
eog polyglot.png

# Use as ZIP archive (rename extension)
mv polyglot.png polyglot.zip
unzip polyglot.zip

# Use WAV file from PNG+WAV polyglot
./target/release/rust-polyglot extract --input media.png --output extracted.wav
```

## How It Works

This tool creates polyglot files through multiple embedding strategies:

**PNG+ZIP Polyglots**: Embed ZIP archive data within PNG image metadata chunks, creating files that:
- Display correctly as PNG images
- Function as ZIP archives when renamed to `.zip`
- Maintain file format validity in both contexts
- **Fully supported**: creation, validation, and extraction

**PNG+WAV Polyglots**: Embed WAV audio data within PNG files, supporting:
- PNG-dominant embedding (PNG with embedded WAV)
- WAV-dominant embedding (PNG with embedded WAV)
- Creation and extraction from PNG+WAV polyglots works
- **Validation only checks PNG+ZIP format (limitation)**

## Installation

Requires Rust toolchain:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone <repository>
cd rust-polyglot
cargo build --release
```

## Usage

```
USAGE:
    rust-polyglot <COMMAND>

COMMANDS:
    create    Create polyglot from PNG image and ZIP archive or WAV file
    extract   Extract embedded ZIP/WAV contents from polyglot file
    validate  Validate polyglot file integrity
    help      Print help information
```

### Create Polyglot

**PNG+ZIP Polyglots:**
```bash
rust-polyglot create --png <image> --zip <archive> --output <result> [--method <embedding>]
```

**PNG+WAV Polyglots:**
```bash
rust-polyglot create --png <image> --wav <audio> --output <result> [--method bidirectional]
```

**Methods:**
- `text` (default): Embed in PNG text/metadata chunks
- `zip`: Container approach (ZIP-dominant)
- `idat`: Broken - don't use

### Validate Polyglot

```bash
rust-polyglot validate --input <file>
# [OK] File is a valid PNG/ZIP polyglot
```

### Extract Archive

```bash
rust-polyglot extract --input <polyglot> --output <extracted.zip>
```

## Architecture

- **Memory-safe**: Written in Rust with compile-time safety guarantees
- **Modular**: Separate libraries for PNG parsing, ZIP handling, polyglot creation
- **Extensible**: Designed to support additional file formats
- **Efficient**: Minimal performance overhead

## Limitations

- Currently only supports PNG as host format
- PNG+WAV polyglots: creation and extraction work, but validation only supports PNG+ZIP format
- Experimental formats (GIF, FLAC) present but not fully integrated
- No encryption or compression support
- Basic error handling for common cases

## Contributing

This is an experimental project exploring polyglot file techniques. Pull requests welcome for:
- Additional file format support
- Improved embedding methods
- Better error handling
- Performance optimizations

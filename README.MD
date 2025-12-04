# Polyglot File Creator

A command-line tool for creating polyglot files - files that are valid in multiple formats simultaneously.

Currently supports PNG+ZIP polyglots where PNG images can also function as ZIP archives.

## Quick Start

```bash
# Install Rust and clone repository
cargo build --release

# Create a PNG polyglot containing ZIP data
./target/release/rust-polyglot create --png image.png --zip archive.zip --output polyglot.png

# Use as PNG image (opens normally in viewers)
eog polyglot.png

# Use as ZIP archive (rename extension)
mv polyglot.png polyglot.zip
unzip polyglot.zip
```

## How It Works

This tool embeds ZIP archive data within PNG image metadata chunks (text chunks), creating files that:
- Display correctly as PNG images
- Contain functional ZIP archives accessed by renaming `.png` to `.zip`
- Maintain file format validity in both contexts

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
    create    Create PNG+ZIP polyglot from PNG image and ZIP archive
    extract   Extract ZIP contents from polyglot file
    validate  Validate polyglot file integrity
    help      Print help information
```

### Create Polyglot

```bash
rust-polyglot create --png <image> --zip <archive> --output <result> [--method <embedding>]
```

**Methods:**
- `text` (default): Embed in PNG text/metadata chunks
- `zip`: Container approach (ZIP-dominant)
- `idat`: Broken - don't use

### Validate Polyglot

```bash
rust-polyglot validate --input <file>
# ✓ File is a valid PNG/ZIP polyglot
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
- Experimental formats (GIF, FLAC) present but not fully integrated
- No encryption or compression support
- Basic error handling for common cases

## Contributing

This is an experimental project exploring polyglot file techniques. Pull requests welcome for:
- Additional file format support
- Improved embedding methods
- Better error handling
- Performance optimizations

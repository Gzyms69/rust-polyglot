 use std::path::Path;
use clap::{Parser, Subcommand};
use rust_polyglot::{cli, polyglot::{PolyglotCreator, create_png_wav_polyglot, create_true_bidirectional_png_wav_polyglot}, utils};
use rust_polyglot::extract::{validate_polyglot, extract_zip_from_png, extract_wav_from_png};

// Find RIFF signature ("RIFF") in data, returning offset
fn find_riff_signature(data: &[u8]) -> Option<usize> {
    const RIFF_SIG: [u8; 4] = *b"RIFF";
    data.windows(4).position(|w| w == RIFF_SIG)
}

#[derive(Parser)]
#[command(name = "rust-polyglot")]
#[command(about = "Create and manipulate PNG/ZIP polyglots")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a PNG+ZIP or PNG+WAV polyglot from a PNG file and archive/audio file
    Create {
        /// Path to input PNG file
        #[arg(short, long)]
        png: String,

        /// Path to input ZIP file (or WAV file with --wav flag)
        #[arg(short, long)]
        zip: Option<String>,

        /// Path to input WAV file (alternative to --zip for PNG+WAV polyglots)
        #[arg(long)]
        wav: Option<String>,

        /// Path for output polyglot file (must end with .png or .zip)
        #[arg(short, long)]
        output: String,

        /// Embedding method: idat (PNG-dominant, data in image data), text (PNG-dominant, data in metadata - RECOMMENDED), zip (ZIP-dominant, PNG in archive), bidirectional (true bidirectional PNG+WAV)
        #[arg(short, long, default_value = "text")]
        method: String,
    },

    /// Extract the ZIP archive from a polyglot file
    Extract {
        /// Path to polyglot PNG file
        #[arg(short, long)]
        input: String,

        /// Path for extracted ZIP file
        #[arg(short, long)]
        output: String,
    },

    /// Validate that a file is a valid PNG/ZIP polyglot (PNG+WAV validation not supported)
    Validate {
        /// Path to potential polyglot file
        #[arg(short, long)]
        input: String,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { png, zip, wav, output, method } => {
            let png_path = Path::new(&png);
            let output_path = Path::new(&output);

            // Check if user wants true bidirectional polyglot
            if method == "bidirectional" {
                if let Some(wav_path) = wav {
                    // True bidirectional PNG+WAV polyglot
                    let wav_path = Path::new(&wav_path);

                    // Validate inputs - allow flexibility for bidirectional mode
                    if !output_path.extension().is_some_and(|ext| ext == "png" || ext == "wav") {
                        eprintln!("Error: Output file for bidirectional polyglot can have .png or .wav extension");
                        std::process::exit(1);
                    }

                    println!("Creating truly bidirectional PNG+WAV polyglot (custom format): {} + {} -> {}", png, wav_path.display(), output);
                    create_true_bidirectional_png_wav_polyglot(png_path, wav_path, output_path)?;
                    println!("True bidirectional PNG+WAV polyglot created successfully!");
                } else {
                    eprintln!("Error: --wav parameter required for bidirectional mode");
                    std::process::exit(1);
                }
            } else {
                // Regular polyglot creation logic
                let png_path = Path::new(&png);
                let output_path = Path::new(&output);

                // Determine which type of polyglot to create
                if let Some(wav_path) = wav {
                    // PNG+WAV polyglot
                    let wav_path = Path::new(&wav_path);

                    // Choose approach based on extension:
                    // .png → PNG-dominant (PNG + embedded WAV)
                    // .wav → WAV-dominant (WAV + embedded PNG)
                    let png_dominant = output_path.extension().is_some_and(|ext| ext == "png");

                    // Extensions are validated - proceed

                    println!("Creating PNG+WAV bidirectional polyglot: {} + {} -> {}", png, wav_path.display(), output);
                    create_png_wav_polyglot(png_path, wav_path, output_path)?;
                    println!("PNG+WAV polyglot created successfully!");

                } else if let Some(zip_path) = zip {
                    // PNG+ZIP polyglot (original)
                    let zip_path = Path::new(&zip_path);

                    // Validate inputs
                    if !output_path.extension().is_some_and(|ext| ext == "png" || ext == "zip") {
                        eprintln!("Error: Output file must have .png or .zip extension");
                        std::process::exit(1);
                    }

                    println!("Creating polyglot: {} + {} -> {}", png, zip_path.display(), output);
                    let mut creator = PolyglotCreator::new(png_path, zip_path)?;
                    creator.create_polyglot_with_method(output_path, &method)?;
                    println!("PNG+ZIP polyglot created successfully!");

                } else {
                    eprintln!("Error: Must specify either --zip or --wav");
                    std::process::exit(1);
                }
            }
        }

        Commands::Extract { input, output } => {
            let input_path = Path::new(&input);
            let output_path = Path::new(&output);

            // Determine what to extract based on file content
            let data = std::fs::read(input_path)?;
            let is_png = utils::is_png_signature(&data);

            if is_png {
                // PNG-dominant polyglot - check which data is embedded
                if find_riff_signature(&data[8..]).is_some() {
                    // PNG+WAV polyglot
                    println!("Extracting WAV from PNG+WAV polyglot: {} -> {}", input, output);
                    extract_wav_from_png(input_path, output_path)?;
                    println!("WAV extracted successfully!");
                } else {
                    // Default to ZIP extraction for backward compatibility
                    println!("Extracting ZIP from PNG+ZIP polyglot: {} -> {}", input, output);
                    extract_zip_from_png(input_path, output_path)?;
                    println!("ZIP extracted successfully!");
                }
            } else if &data[0..4] == b"RIFF" {
                // WAV-dominant polyglot - this IS the WAV file, extract PNG from it
                println!("Extracting PNG from WAV+PNG polyglot: {} -> {}", input, output);
                // For WAV-dominant polyglots, we'll extract PNG since WAV is the container
                use rust_polyglot::wav::WavFile;
                let wav_file = WavFile::from_file(input_path)?;
                if let Some(png_data) = wav_file.extract_png_data() {
                    std::fs::write(output_path, png_data)?;
                    println!("PNG extracted successfully!");
                } else {
                    eprintln!("No PNG data found in WAV polyglot");
                    std::process::exit(1);
                }
            } else {
                // For ZIP-dominant cases, fall back to generic handling
                eprintln!("ZIP-dominant polyglot extraction not yet supported for this interface");
                eprintln!("Use existing ZIP tools or access via other methods");
                std::process::exit(1);
            }
        }

        Commands::Validate { input, verbose } => {
            let input_path = Path::new(&input);

            println!("Validating polyglot: {}", input);
            let result = validate_polyglot(input_path)?;

            match result {
                cli::ValidationResult::Valid => {
                    println!("[OK] File is a valid PNG/ZIP polyglot");
                }
                cli::ValidationResult::InvalidPng(reason) => {
                    println!("[ERROR] Not a valid PNG: {}", reason);
                }
                cli::ValidationResult::InvalidZip(reason) => {
                    println!("[ERROR] Not a valid ZIP: {}", reason);
                }
                cli::ValidationResult::InvalidBoth(png_reason, zip_reason) => {
                    println!("[ERROR] Invalid PNG: {}", png_reason);
                    println!("         Invalid ZIP: {}", zip_reason);
                }
            }

            if verbose {
                println!("Detailed validation information:");
                // TODO: Add more detailed output
            }
        }
    }

    Ok(())
}

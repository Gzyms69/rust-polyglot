use std::path::Path;
use clap::{Parser, Subcommand};
use rust_polyglot::{cli, polyglot::PolyglotCreator};
use rust_polyglot::extract::{validate_polyglot, extract_zip_from_png};

#[derive(Parser)]
#[command(name = "rust-polyglot")]
#[command(about = "Create and manipulate PNG/ZIP polyglots")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a PNG/ZIP polyglot from a PNG file and ZIP archive
    Create {
        /// Path to input PNG file
        #[arg(short, long)]
        png: String,

        /// Path to input ZIP file
        #[arg(short, long)]
        zip: String,

        /// Path for output polyglot file (must end with .png or .zip)
        #[arg(short, long)]
        output: String,

        /// Embedding method: idat (PNG-dominant, ZIP in image data), text (PNG-dominant, ZIP in metadata - RECOMMENDED), zip (ZIP-dominant, PNG in archive)
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

    /// Validate that a file is a valid PNG/ZIP polyglot
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
        Commands::Create { png, zip, output, method } => {
            let png_path = Path::new(&png);
            let zip_path = Path::new(&zip);
            let output_path = Path::new(&output);

            // Validate inputs
            if !output_path.extension().map_or(false, |ext| ext == "png" || ext == "zip") {
                eprintln!("Error: Output file must have .png or .zip extension");
                std::process::exit(1);
            }

            println!("Creating polyglot: {} + {} -> {}", png, zip, output);
            let mut creator = PolyglotCreator::new(png_path, zip_path)?;
            creator.create_polyglot_with_method(output_path, &method)?;
            println!("Polyglot created successfully!");
        }

        Commands::Extract { input, output } => {
            let input_path = Path::new(&input);
            let output_path = Path::new(&output);

            println!("Extracting ZIP from polyglot: {} -> {}", input, output);
            extract_zip_from_png(input_path, output_path)?;
            println!("ZIP extracted successfully!");
        }

        Commands::Validate { input, verbose } => {
            let input_path = Path::new(&input);

            println!("Validating polyglot: {}", input);
            let result = validate_polyglot(input_path)?;

            match result {
                cli::ValidationResult::Valid => {
                    println!("✓ File is a valid PNG/ZIP polyglot");
                }
                cli::ValidationResult::InvalidPng(reason) => {
                    println!("✗ Not a valid PNG: {}", reason);
                }
                cli::ValidationResult::InvalidZip(reason) => {
                    println!("✗ Not a valid ZIP: {}", reason);
                }
                cli::ValidationResult::InvalidBoth(png_reason, zip_reason) => {
                    println!("✗ Invalid PNG: {}", png_reason);
                    println!("  Invalid ZIP: {}", zip_reason);
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

//! # VirtuC Compiler CLI
//!
//! This module provides the command-line interface for the `virtuc` compiler.
//! It handles parsing command-line arguments and orchestrating the compilation
//! process from source files to executable binaries.
//!
//! ## Usage
//!
//! ```bash
//! virtuc input.c -o output
//! ```
//!
//! ## Features
//!
//! - Compile C subset source files to native executables via LLVM
//! - Optional output file specification

use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;

use virtuc::compile;

#[derive(Parser)]
#[command(name = "virtuc")]
#[command(about = "A Rust-based subset C compiler")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile C source to executable
    Compile {
        /// Input C source file
        input: String,

        /// Output executable file
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Commands::Compile { input, output } => {
            // Read input file
            let source = fs::read_to_string(&input)?;

            // Determine output file
            // Note: Defaulting to ".out" extension is tailored towards macOS and Linux systems.
            // Windows users should explicitly specify an output file with ".exe" extension.
            let output_str =
                output.unwrap_or_else(|| input.trim_end_matches(".c").to_string() + ".out");
            let output_path = Path::new(&output_str);

            // Compile
            match compile(&source, output_path) {
                Ok(_) => {
                    println!("Compiled {} to {}", input, output_str);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

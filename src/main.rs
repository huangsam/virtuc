//! # Virtuc Compiler CLI
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
//! - Future: Support for bytecode generation and VM execution

use clap::Parser;

#[derive(Parser)]
#[command(name = "virtuc")]
#[command(about = "A Rust-based subset C compiler")]
struct Args {
    /// Input C source file
    input: String,

    /// Output executable file
    #[arg(short, long)]
    output: Option<String>,
}

fn main() {
    let args = Args::parse();
    // TODO: Implement compilation logic
    println!("Compiling {} to {:?}", args.input, args.output);
}

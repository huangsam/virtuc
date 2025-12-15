//! # VirtuC Compiler Library
//!
//! This is the main library crate for the `virtuc` compiler, a Rust-based compiler
//! for a minimal subset of the C programming language. The compiler supports only
//! primitives (int, float), for loops, if-elseif-else statements, and functions.
//!
//! ## Architecture
//!
//! The compiler follows a standard compilation pipeline:
//! 1. **Lexing**: Source code → Tokens
//! 2. **Parsing**: Tokens → Abstract Syntax Tree (AST)
//! 3. **Semantic Analysis**: AST validation and type checking
//! 4. **Code Generation**: AST → LLVM Intermediate Representation (IR)
//! 5. **Execution**: IR → Native executable

pub mod ast;
pub mod codegen;
pub mod error;
pub mod header_registry;
pub mod lexer;
pub mod parser;
pub mod semantic;

use std::fs;
use std::path::Path;
use std::process::Command;

/// Compiles a C subset source string to an executable at the specified output path.
///
/// # Arguments
///
/// * `source` - The source code string.
/// * `output` - The path where the executable should be written.
///
/// # Returns
///
/// * `Result<(), Box<dyn std::error::Error>>` - Ok if compilation succeeds, Err otherwise.
pub fn compile(source: &str, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Lexical analysis
    let tokens = lexer::lex(source)?;

    // Parsing
    let ast = parser::parse(&tokens)?;

    // Semantic analysis
    let errors = semantic::analyze(&ast);
    if !errors.is_empty() {
        let error_msg = errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!("Semantic errors:\n{}", error_msg).into());
    }

    // Code generation
    let ir = codegen::generate_ir(&ast)?;

    // Write IR to temporary file
    // Use output path with .ll extension
    let ir_file = output.with_extension("ll");
    fs::write(&ir_file, &ir)?;

    // Compile IR to executable using clang
    let status = Command::new("clang")
        .args([
            ir_file.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "-lc",
            "-Wno-override-module",
        ])
        .status()?;

    if !status.success() {
        return Err("Compilation failed".into());
    }

    // Clean up IR file
    let _ = fs::remove_file(ir_file);

    Ok(())
}

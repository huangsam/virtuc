//! # Virtuc Compiler Library
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
//! 5. **Execution**: IR → Native executable or bytecode for VM
//!
//! ## Modules
//!
//! - [`error`]: Compiler error types and diagnostics
//! - [`lexer`]: Lexical analysis and tokenization
//! - [`parser`]: Syntax analysis and AST construction
//! - [`ast`]: Abstract Syntax Tree node definitions
//! - [`semantic`]: Semantic analysis, type checking, and validation
//! - [`codegen`]: Code generation to LLVM IR
//! - [`vm`]: Optional simple virtual machine for bytecode execution

pub mod error;
pub mod lexer;
pub mod parser;
pub mod ast;
pub mod semantic;
pub mod codegen;
pub mod vm;

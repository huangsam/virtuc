//! # Compiler Error Types
//!
//! This module defines the error types and diagnostics used throughout the
//! `virtuc` compiler. It provides structured error reporting for various
//! compilation phases including lexing, parsing, semantic analysis, and
//! code generation.
//!
//! ## Error Categories
//!
//! - **Lexical Errors**: Invalid tokens, unexpected characters
//! - **Syntax Errors**: Malformed syntax, parsing failures
//! - **Semantic Errors**: Type mismatches, undefined variables, scope issues
//! - **Code Generation Errors**: LLVM IR generation failures
//!
//! ## Design
//!
//! Errors implement `std::error::Error` and `std::fmt::Display` for
//! user-friendly error messages. Each error includes location information
//! (file, line, column) when possible for better debugging.

// Compiler error types

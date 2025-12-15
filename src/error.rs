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

use std::fmt;

/// Represents errors that can occur during lexical analysis.
///
/// This error is produced when the lexer encounters characters or sequences
/// that do not match any valid token pattern in the C subset grammar.
/// Examples include invalid operators, malformed literals, or unexpected
/// characters in the source code.
///
/// # Usage
///
/// Returned by the [`lex`](crate::lexer::lex) function when tokenization fails.
#[derive(Debug, PartialEq, Clone)]
pub struct LexerError;

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid token encountered")
    }
}

impl std::error::Error for LexerError {}

/// Represents errors that can occur during parsing.
///
/// This error wraps error messages from the parser combinator library
/// when the source code cannot be parsed according to the C subset grammar.
/// Common causes include missing semicolons, unmatched parentheses, or
/// malformed expressions/statements.
///
/// # Usage
///
/// Returned by the [`parse`](crate::parser::parse) function when AST construction fails.
#[derive(Debug, PartialEq, Clone)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

/// Represents errors that can occur during semantic analysis.
///
/// These errors are detected after parsing when validating the program's
/// semantic correctness, including type checking, scope resolution, and
/// symbol validation. They ensure the program adheres to the language rules
/// before code generation.
///
/// # Usage
///
/// Returned by the [`analyze`](crate::semantic::analyze) function when semantic
/// validation fails.
#[derive(Debug, PartialEq, Clone)]
pub enum SemanticError {
    /// Variable is used but not declared
    UndefinedVariable(String),
    /// Variable is declared multiple times in the same scope
    DuplicateVariable(String),
    /// Type mismatch in assignment or operation
    TypeMismatch(String),
    /// Function is called but not declared
    UndefinedFunction(String),
    /// Wrong number of arguments in function call
    WrongArgumentCount(String, usize, usize),
    /// Return type mismatch
    ReturnTypeMismatch(String),
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SemanticError::UndefinedVariable(name) => {
                write!(f, "Undefined variable: {}", name)
            }
            SemanticError::DuplicateVariable(name) => {
                write!(f, "Duplicate variable declaration: {}", name)
            }
            SemanticError::TypeMismatch(msg) => {
                write!(f, "Type mismatch: {}", msg)
            }
            SemanticError::UndefinedFunction(name) => {
                write!(f, "Undefined function: {}", name)
            }
            SemanticError::WrongArgumentCount(func, expected, got) => {
                write!(
                    f,
                    "Wrong number of arguments for {}: expected {}, got {}",
                    func, expected, got
                )
            }
            SemanticError::ReturnTypeMismatch(msg) => {
                write!(f, "Return type mismatch: {}", msg)
            }
        }
    }
}

impl std::error::Error for SemanticError {}

/// Represents errors that can occur during code generation.
///
/// This error wraps error messages from LLVM IR generation failures.
/// Common causes include invalid operations, unsupported constructs, or
/// issues with the inkwell LLVM bindings.
///
/// # Usage
///
/// Returned by code generation functions when LLVM IR emission fails.
#[derive(Debug, PartialEq, Clone)]
pub struct CodegenError(pub String);

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Code generation error: {}", self.0)
    }
}

impl std::error::Error for CodegenError {}

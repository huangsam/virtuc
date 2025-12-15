//! # Syntax Analysis and AST Construction
//!
//! This module implements the parser that converts a stream of tokens into
//! an Abstract Syntax Tree (AST). It uses the `nom` parser combinator library
//! for building the parsing logic.
//!
//! ## Grammar
//!
//! The parser handles the C subset grammar including:
//! - Expressions: arithmetic, comparison, assignment
//! - Statements: variable declarations, assignments, control flow
//! - Functions: declarations and definitions
//! - Control structures: if-else, for loops
//!
//! ## Parser Combinators
//!
//! Uses `nom`'s combinator approach to build modular parsers for each
//! grammar rule. Provides good error messages and recovery for syntax errors.

// AST building using nom crate

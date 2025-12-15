//! # Lexical Analysis
//!
//! This module handles the lexical analysis phase of the compiler, converting
//! raw source code into a stream of tokens. It uses the `logos` crate for
//! efficient tokenization.
//!
//! ## Supported Tokens
//!
//! The lexer recognizes tokens for the C subset including:
//! - Keywords: `int`, `float`, `if`, `else`, `for`, `return`
//! - Operators: `+`, `-`, `*`, `/`, `=`, `==`, `!=`, `<`, `>`, etc.
//! - Literals: Integer and floating-point numbers
//! - Identifiers: Variable and function names
//! - Punctuation: `(`, `)`, `{`, `}`, `;`, `,`, etc.
//!
//! ## Implementation
//!
//! Uses the `logos` procedural macro to define token patterns and generate
//! the lexer automatically. Handles whitespace, comments, and error recovery.

// Tokenization using logos crate

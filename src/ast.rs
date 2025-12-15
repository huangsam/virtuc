//! # Abstract Syntax Tree Definitions
//!
//! This module defines the data structures that represent the Abstract Syntax
//! Tree (AST) nodes for the C subset. Each node corresponds to a construct
//! in the language grammar.
//!
//! ## Node Types
//!
//! - **Expressions**: Binary operations, literals, identifiers, function calls
//! - **Statements**: Variable declarations, assignments, returns, blocks
//! - **Control Flow**: If-else statements, for loops
//! - **Functions**: Function declarations and definitions
//! - **Program**: Top-level program structure
//!
//! ## Design
//!
//! AST nodes are defined as enums and structs with owned data to simplify
//! lifetime management. Each node includes source location information for
//! error reporting and debugging.

// AST node definitions

//! # Semantic Analysis
//!
//! This module performs semantic analysis on the AST, including type checking,
//! symbol resolution, and validation of language semantics. It ensures the
//! program is semantically correct before code generation.
//!
//! ## Analysis Phases
//!
//! 1. **Symbol Collection**: Gather all declarations and build symbol tables
//! 2. **Type Checking**: Verify type compatibility in expressions and assignments
//! 3. **Scope Resolution**: Ensure variables are declared before use
//! 4. **Control Flow Validation**: Check loop and conditional constructs
//!
//! ## Symbol Tables
//!
//! Maintains scoped symbol tables for variables, functions, and types.
//! Handles nested scopes for blocks, functions, and control structures.

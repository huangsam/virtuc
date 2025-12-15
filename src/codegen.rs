//! # Code Generation
//!
//! This module generates LLVM Intermediate Representation (IR) from the
//! validated AST. It uses the `inkwell` crate to interface with LLVM for
//! creating optimized native executables.
//!
//! ## Code Generation Strategy
//!
//! - **Expressions**: Generate IR for arithmetic, comparison, and assignment
//! - **Statements**: Handle control flow, variable allocation, and function calls
//! - **Functions**: Create LLVM functions with proper signatures and bodies
//! - **Program**: Link all components into a complete module
//!
//! ## LLVM Integration
//!
//! Uses `inkwell` to build LLVM IR incrementally. Handles type mapping from
//! the C subset types to LLVM types, and generates efficient code with
//! optimizations enabled.

// IR emission using inkwell for LLVM

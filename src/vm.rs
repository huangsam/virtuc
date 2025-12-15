//! # Virtual Machine
//!
//! This module implements an optional simple virtual machine for executing
//! bytecode generated from the compiler. It provides an alternative execution
//! path to native compilation via LLVM.
//!
//! ## Architecture
//!
//! - **Bytecode Format**: Simple stack-based instruction set
//! - **Execution Model**: Interpreter with call stack and heap
//! - **Primitives**: Support for int and float operations
//! - **Control Flow**: Conditional jumps and loops
//!
//! ## Use Cases
//!
//! - Educational: Study execution semantics without LLVM complexity
//! - Debugging: Step-through execution for compiler testing
//! - Portability: Cross-platform execution without native compilation
//!
//! ## Future Extensions
//!
//! Could be extended to support just-in-time (JIT) compilation or
//! additional bytecode optimizations.

// Optional simple VM for bytecode execution

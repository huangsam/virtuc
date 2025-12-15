# VirtuC

[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/huangsam/virtuc/ci.yml)](https://github.com/huangsam/virtuc/actions)
[![License](https://img.shields.io/github/license/huangsam/virtuc)](https://github.com/huangsam/virtuc/blob/main/LICENSE)

A compiler for a subset of the C programming language, implemented in Rust.

VirtuC supports lexical analysis, parsing, semantic analysis, code generation to LLVM IR, and execution via a custom virtual machine. It compiles C-like code into efficient LLVM intermediate representation for optimization and native execution, while also providing a bytecode interpreter for direct runtime evaluation.

## Features

- **Lexing & Parsing**: Tokenizes and parses C subset into AST using `logos` and `nom`.
- **Semantic Analysis**: Type checking and symbol resolution.
- **Code Generation**: Emits LLVM IR via `inkwell`.
- **VM & CLI**: Executes bytecode or compiles to executables.

## Supported C Subset

- Primitive types: `int` (64-bit), `float` (64-bit)
- Variables and assignments
- Arithmetic and comparison operators
- Control flow: `if-else`, `for` loops
- Functions with parameters and return values
- Function calls

## Getting Started

Ensure you have Rust and LLVM installed. Then:

```bash
cargo install --path .
```

Once compiled, you can use the tool to compile C source files:

```bash
# Compile the source file
virtuc compile hello.c

# Run the generated executable
./hello.out
```

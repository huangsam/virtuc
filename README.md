# VirtuC

[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/huangsam/virtuc/ci.yml)](https://github.com/huangsam/virtuc/actions)
[![License](https://img.shields.io/github/license/huangsam/virtuc)](https://github.com/huangsam/virtuc/blob/main/LICENSE)

A compiler for a subset of the C programming language, implemented in Rust. VirtuC supports lexical analysis, parsing, semantic analysis, code generation to LLVM IR, and execution via a custom virtual machine.

## Features

- **Lexer**: Tokenizes C subset source code using the `logos` crate.
- **Parser**: Parses tokens into an Abstract Syntax Tree (AST) using the `nom` parser combinator library.
- **Semantic Analysis**: Performs type checking, symbol resolution, and validation.
- **Code Generation**: Translates AST to LLVM Intermediate Representation (IR) using the `inkwell` crate.
- **Virtual Machine**: Executes compiled bytecode with a stack-based architecture.
- **CLI**: Command-line interface for compiling source files to executables.

## Supported C Subset

- Primitive types: `int` (64-bit), `float` (64-bit)
- Variables and assignments
- Arithmetic and comparison operators
- Control flow: `if-else`, `for` loops
- Functions with parameters and return values
- Function calls

## Building

Ensure you have Rust and LLVM installed. Then:

```bash
cargo build --release
```

## Usage

Compile a C subset source file to an executable:

```bash
cargo run -- compile source.c -o output
```

Run the executable:

```bash
./output
```

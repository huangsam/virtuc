# VirtuC

[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/huangsam/virtuc/ci.yml)](https://github.com/huangsam/virtuc/actions)
[![License](https://img.shields.io/github/license/huangsam/virtuc)](https://github.com/huangsam/virtuc/blob/main/LICENSE)

A compiler for a subset of the C programming language, implemented in Rust.

VirtuC supports the E2E compilation from C source code to [LLVM IR] for native execution. The IR artifact is then compiled and linked using `clang` to produce a final executable.

[LLVM IR]: https://llvm.org/docs/LangRef.html

## Features

- **Lexing & Parsing**: Tokenizes and parses C subset into AST using `logos` and `nom`.
- **Semantic Analysis**: Type checking and symbol resolution.
- **Code Generation**: Emits LLVM IR via `inkwell` and links with system libraries via `clang`.
- **CLI**: Compiles to native executables.
- **C Interop**: Supports `extern` declarations and `#include <...>` headers.

## Supported C subset

- Primitive types: `int` (64-bit), `float` (64-bit)
- Variables and assignments
- Arithmetic and comparison operators
- Control flow: `if-else`, `for` loops
- Functions with parameters and return values
- Function calls

## Getting started

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

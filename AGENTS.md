# Agent Guidelines for VirtuC

## Core Invariants
- **Pipeline Stage Separation**: Compilation strictly flows through Lexer (`logos`) → Parser (`nom`) → Semantic Analysis (`semantic.rs`) → Code Generation (`inkwell`). Do not bypass semantic analysis before IR emission, and keep AST definitions independent of LLVM constructs.
- **Type System Strictness**: C primitives map to 64-bit LLVM types (`int` → `i64`, `float` → `f64`), along with `void`, pointers (`T*`), fixed-size 1D and 2D arrays (`T[N]`, `T[R][C]`), and named structs (`struct Name`). No implicit type coercion; type mismatches must be caught in `semantic.rs`.
- **Lexical Scoping & Symbol Integrity**: Symbol tables (`scopes`, `array_scopes`, `structs`) manage variable, array, struct, and function declarations. Variables and struct fields cannot be redeclared within the same scope, and all identifiers, member accesses, and function calls must be validated before codegen.
- **Struct Layout & Member Resolution Safety**: Member accesses (`.` and `->`) must resolve against verified struct definitions in `semantic.rs` prior to emitting LLVM `build_struct_gep`. Recursive data structures must use pointer indirection (`struct T*`) to maintain well-defined aggregate layouts.
- **Control Flow & Jump Target Safety**: `break` and `continue` statements are strictly confined to active loop contexts (`loop_depth > 0` in semantic analysis; `loop_stack` in codegen). Logical operators (`&&`, `||`) must branch with basic blocks to guarantee short-circuit evaluation.
- **Header Registry & Linker Parity**: `#include` directives inject pre-defined declarations via `header_registry.rs`. Standard header function signatures, semantic symbol tables, and clang linker flags (`-lc`, `-lm`) must remain consistent.

## Build & Quality Checks
Run before completing tasks:
```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

# CLI compilation check
cargo run -- tests/fixtures/for_loops.c -o /tmp/for_loops.out
```

## Performance Expectations
- Frontend and IR generation latency: <50ms for typical C subset programs.
- Test suite runtime: <10s for the entire workspace test suite (unit and integration tests).
- Intermediate artifact hygiene: Temporary `.ll` files must be cleaned up immediately after clang linking.
- Zero-panic contract: Invalid syntax or semantic violations must produce structured diagnostics (`LexerError`, `ParseError`, `SemanticError`, `CodegenError`) with no panics.

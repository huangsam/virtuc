//! # Header Registry
//!
//! Provides a mapping of C system headers to their corresponding extern function
//! declarations. When a source file includes a header (e.g., `#include <stdio.h>`),
//! this registry automatically injects the appropriate function declarations.
//!
//! This allows code to use standard library functions without explicit extern declarations.
//!
//! ## Supported Headers
//!
//! Currently supports:
//! - `stdio.h` - Standard I/O functions (`printf`, `puts`, `putchar`)
//! - `stdlib.h` - General utility functions (`abs`, `exit`)
//! - `math.h` - Common math functions (`sqrt`, `pow`, `sin`, `cos`, `floor`, `ceil`)

use crate::ast::{ExternFunction, Type};

/// Returns the list of extern functions that should be automatically available for a header.
///
/// # Arguments
/// * `header` - The header name (e.g., "stdio.h")
///
/// # Returns
/// A vector of extern function declarations provided by this header.
pub fn externs_for_header(header: &str) -> Vec<ExternFunction> {
    match header {
        "stdio.h" => vec![
            ExternFunction {
                return_ty: Type::Int,
                name: "printf".to_string(),
                param_types: vec![Type::String],
                is_variadic: true,
            },
            ExternFunction {
                return_ty: Type::Int,
                name: "puts".to_string(),
                param_types: vec![Type::String],
                is_variadic: false,
            },
            ExternFunction {
                return_ty: Type::Int,
                name: "putchar".to_string(),
                param_types: vec![Type::Int],
                is_variadic: false,
            },
        ],
        "stdlib.h" => vec![
            ExternFunction {
                return_ty: Type::Int,
                name: "abs".to_string(),
                param_types: vec![Type::Int],
                is_variadic: false,
            },
            ExternFunction {
                return_ty: Type::Void,
                name: "exit".to_string(),
                param_types: vec![Type::Int],
                is_variadic: false,
            },
        ],
        "math.h" => vec![
            ExternFunction {
                return_ty: Type::Float,
                name: "sqrt".to_string(),
                param_types: vec![Type::Float],
                is_variadic: false,
            },
            ExternFunction {
                return_ty: Type::Float,
                name: "pow".to_string(),
                param_types: vec![Type::Float, Type::Float],
                is_variadic: false,
            },
            ExternFunction {
                return_ty: Type::Float,
                name: "sin".to_string(),
                param_types: vec![Type::Float],
                is_variadic: false,
            },
            ExternFunction {
                return_ty: Type::Float,
                name: "cos".to_string(),
                param_types: vec![Type::Float],
                is_variadic: false,
            },
            ExternFunction {
                return_ty: Type::Float,
                name: "floor".to_string(),
                param_types: vec![Type::Float],
                is_variadic: false,
            },
            ExternFunction {
                return_ty: Type::Float,
                name: "ceil".to_string(),
                param_types: vec![Type::Float],
                is_variadic: false,
            },
            ExternFunction {
                return_ty: Type::Float,
                name: "fabs".to_string(),
                param_types: vec![Type::Float],
                is_variadic: false,
            },
        ],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdio_injects_stdio_functions() {
        let exts = externs_for_header("stdio.h");
        assert_eq!(exts.len(), 3);
        assert!(exts.iter().any(|e| e.name == "printf" && e.is_variadic));
        assert!(exts.iter().any(|e| e.name == "puts"));
    }

    #[test]
    fn stdlib_injects_stdlib_functions() {
        let exts = externs_for_header("stdlib.h");
        assert_eq!(exts.len(), 2);
        assert!(exts.iter().any(|e| e.name == "abs"));
        assert!(
            exts.iter()
                .any(|e| e.name == "exit" && e.return_ty == Type::Void)
        );
    }

    #[test]
    fn math_injects_math_functions() {
        let exts = externs_for_header("math.h");
        assert_eq!(exts.len(), 7);
        assert!(exts.iter().any(|e| e.name == "sqrt"));
        assert!(exts.iter().any(|e| e.name == "pow"));
    }

    #[test]
    fn unknown_header_empty() {
        let exts = externs_for_header("unknown.h");
        assert!(exts.is_empty());
    }
}

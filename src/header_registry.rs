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
//! - `stdio.h` - Standard I/O functions (printf, etc.)

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
        "stdio.h" => vec![ExternFunction {
            return_ty: Type::Int,
            name: "printf".to_string(),
            param_types: vec![Type::String],
            is_variadic: true,
        }],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdio_injects_printf() {
        let exts = externs_for_header("stdio.h");
        assert_eq!(exts.len(), 1);
        let e = &exts[0];
        assert_eq!(e.name, "printf");
        assert!(e.is_variadic);
    }

    #[test]
    fn unknown_header_empty() {
        let exts = externs_for_header("unknown.h");
        assert!(exts.is_empty());
    }
}

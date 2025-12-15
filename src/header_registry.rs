//! Small header -> externs registry.
//!
//! Provides a minimal mapping of system headers to extern function
//! declarations (used to map `#include <stdio.h>` -> `printf`, etc.).

use crate::ast::{ExternFunction, Type};

/// Returns the list of extern functions that should be injected for `header`.
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

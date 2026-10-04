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

mod expr;
mod stmt;
mod symbols;

use crate::ast::*;
use crate::error::SemanticError;
use std::collections::HashMap;

/// Represents the semantic analyzer.
pub struct SemanticAnalyzer {
    /// Global function symbols: name -> (return_type, param_types, is_variadic)
    pub(crate) functions: HashMap<String, (Type, Vec<Type>, bool)>,
    /// Global struct definitions: name -> StructDef
    pub(crate) structs: HashMap<String, StructDef>,
    /// Stack of scopes for variables: each scope is name -> type
    pub(crate) scopes: Vec<HashMap<String, Type>>,
    /// Stack of scopes for arrays: each scope is name -> (element type, dims)
    pub(crate) array_scopes: Vec<HashMap<String, (Type, Vec<usize>)>>,
    /// Current function's expected return type (during analysis)
    pub(crate) current_return_type: Option<Type>,
    /// Loop nesting depth
    pub(crate) loop_depth: usize,
    /// Collected errors
    pub(crate) errors: Vec<SemanticError>,
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    /// Creates a new semantic analyzer.
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            structs: HashMap::new(),
            scopes: vec![HashMap::new()], // Global scope
            array_scopes: vec![HashMap::new()],
            current_return_type: None,
            loop_depth: 0,
            errors: Vec::new(),
        }
    }

    /// Analyzes the program and returns any semantic errors.
    pub fn analyze(&mut self, program: &Program) -> Vec<SemanticError> {
        self.collect_structs(program);
        self.collect_functions(program);
        for function in &program.functions {
            self.analyze_function(function);
        }
        self.errors.clone()
    }

    /// Analyzes a single function.
    fn analyze_function(&mut self, function: &Function) {
        // Set the expected return type for this function
        let prev_return_type = self.current_return_type.clone();
        self.current_return_type = Some(function.return_ty.clone());

        // Enter function scope
        self.scopes.push(HashMap::new());
        self.array_scopes.push(HashMap::new());
        // Add parameters to scope
        for (ty, name) in &function.params {
            if *ty == Type::Void {
                self.errors.push(SemanticError::TypeMismatch(
                    "Parameter cannot have void type".to_string(),
                ));
            }
            self.scopes
                .last_mut()
                .unwrap()
                .insert(name.clone(), ty.clone());
        }
        // Analyze body
        self.check_stmt(&function.body);

        // Restore previous return type
        self.current_return_type = prev_return_type;
        // Pop function scope
        self.scopes.pop();
        self.array_scopes.pop();
    }
}

/// Convenience function to analyze a program.
pub fn analyze(program: &Program) -> Vec<SemanticError> {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    #[test]
    fn test_valid_function() {
        let input = "int add(int a, int b) { return a + b; }";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_undefined_variable() {
        let input = "int foo() { return x; }";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], SemanticError::UndefinedVariable(_)));
    }

    #[test]
    fn test_type_mismatch() {
        let input = "int foo() { int x = 5.0; return x; }";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert!(!errors.is_empty()); // Should have type mismatch
    }

    #[test]
    fn test_duplicate_variable() {
        let input = "int foo() { int x = 5; int x = 6; return x; }";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], SemanticError::DuplicateVariable(_)));
    }

    #[test]
    fn test_return_type_mismatch_float_to_int() {
        let input = "int foo() { return 3.14; }";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], SemanticError::TypeMismatch(_)));
    }

    #[test]
    fn test_return_type_mismatch_int_to_float() {
        let input = "float foo() { return 42; }";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], SemanticError::TypeMismatch(_)));
    }

    #[test]
    fn test_missing_return_value() {
        let input = "int foo() { int x = 5; return; }";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], SemanticError::TypeMismatch(_)));
    }

    #[test]
    fn test_valid_float_function() {
        let input = "float add(float a, float b) { return a + b; }";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_valid_array_operations() {
        let input = "int test() { int arr[10]; arr[0] = 5; return arr[0]; }";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_invalid_array_type_mismatch() {
        let input = "int test() { int arr[10]; arr[0] = 3.14; return arr[0]; }";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], SemanticError::TypeMismatch(_)));
    }

    #[test]
    fn test_valid_struct_operations() {
        let input = "
            struct Point {
                int x;
                float y;
            };

            int test() {
                struct Point p;
                p.x = 10;
                p.y = 3.14;
                struct Point* ptr = &p;
                ptr->x = 20;
                return ptr->x;
            }
        ";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert!(
            errors.is_empty(),
            "Unexpected semantic errors: {:?}",
            errors
        );
    }

    #[test]
    fn test_invalid_struct_field_access() {
        let input = "
            struct Point {
                int x;
            };

            int test() {
                struct Point p;
                p.z = 10;
                return p.x;
            }
        ";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let errors = analyze(&ast);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], SemanticError::TypeMismatch(_)));
    }
}

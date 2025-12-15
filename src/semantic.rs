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

use std::collections::HashMap;
use crate::ast::*;
use crate::error::SemanticError;

/// Represents the semantic analyzer.
pub struct SemanticAnalyzer {
    /// Global function symbols: name -> (return_type, param_types)
    functions: HashMap<String, (Type, Vec<Type>)>,
    /// Stack of scopes for variables: each scope is name -> type
    scopes: Vec<HashMap<String, Type>>,
    /// Collected errors
    errors: Vec<SemanticError>,
}

impl SemanticAnalyzer {
    /// Creates a new semantic analyzer.
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            scopes: vec![HashMap::new()], // Global scope
            errors: Vec::new(),
        }
    }

    /// Analyzes the program and returns any semantic errors.
    pub fn analyze(&mut self, program: &Program) -> Vec<SemanticError> {
        self.collect_functions(program);
        for function in &program.functions {
            self.analyze_function(function);
        }
        self.errors.clone()
    }

    /// Collects function declarations into the global symbol table.
    fn collect_functions(&mut self, program: &Program) {
        for function in &program.functions {
            let param_types: Vec<Type> = function.params.iter().map(|(ty, _)| *ty).collect();
            if self.functions.contains_key(&function.name) {
                self.errors.push(SemanticError::DuplicateVariable(function.name.clone()));
            } else {
                self.functions.insert(function.name.clone(), (function.return_ty, param_types));
            }
        }
    }

    /// Analyzes a single function.
    fn analyze_function(&mut self, function: &Function) {
        // Enter function scope
        self.scopes.push(HashMap::new());
        // Add parameters to scope
        for (ty, name) in &function.params {
            self.scopes.last_mut().unwrap().insert(name.clone(), *ty);
        }
        // Analyze body
        self.check_stmt(&function.body);
        // Check return type if body has return
        // For simplicity, assume functions return correctly
        // TODO: Check return statements match function return type
        // Pop function scope
        self.scopes.pop();
    }

    /// Checks a statement.
    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Declaration { ty, name, init } => {
                if self.scopes.last().unwrap().contains_key(name) {
                    self.errors.push(SemanticError::DuplicateVariable(name.clone()));
                } else {
                    self.scopes.last_mut().unwrap().insert(name.clone(), *ty);
                    if let Some(expr) = init {
                        let expr_ty = self.check_expr(expr);
                        if expr_ty != Some(*ty) {
                            self.errors.push(SemanticError::TypeMismatch(format!("Cannot assign {:?} to {:?}", expr_ty, ty)));
                        }
                    }
                }
            }
            Stmt::Assignment { name, expr } => {
                let expr_ty = self.check_expr(expr);
                if let Some(var_ty) = self.lookup_variable(name) {
                    if expr_ty != Some(var_ty) {
                        self.errors.push(SemanticError::TypeMismatch(format!("Cannot assign {:?} to {:?}", expr_ty, var_ty)));
                    }
                } else {
                    self.errors.push(SemanticError::UndefinedVariable(name.clone()));
                }
            }
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    self.check_expr(e);
                }
                // TODO: Check return type matches function
            }
            Stmt::Block(stmts) => {
                self.scopes.push(HashMap::new());
                for stmt in stmts {
                    self.check_stmt(stmt);
                }
                self.scopes.pop();
            }
            Stmt::If { cond, then, else_ } => {
                let cond_ty = self.check_expr(cond);
                if cond_ty != Some(Type::Int) {
                    self.errors.push(SemanticError::TypeMismatch("Condition must be int".to_string()));
                }
                self.check_stmt(then);
                if let Some(else_stmt) = else_ {
                    self.check_stmt(else_stmt);
                }
            }
            Stmt::For { init, cond, update, body } => {
                self.scopes.push(HashMap::new());
                if let Some(init_stmt) = init {
                    self.check_stmt(init_stmt);
                }
                if let Some(cond_expr) = cond {
                    let cond_ty = self.check_expr(cond_expr);
                    if cond_ty != Some(Type::Int) {
                        self.errors.push(SemanticError::TypeMismatch("Condition must be int".to_string()));
                    }
                }
                if let Some(update_expr) = update {
                    self.check_expr(update_expr);
                }
                self.check_stmt(body);
                self.scopes.pop();
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr);
            }
        }
    }

    /// Checks an expression and returns its type.
    fn check_expr(&mut self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(_) => Some(Type::Int),
                Literal::Float(_) => Some(Type::Float),
            },
            Expr::Identifier(name) => {
                if let Some(ty) = self.lookup_variable(name) {
                    Some(ty)
                } else {
                    self.errors.push(SemanticError::UndefinedVariable(name.clone()));
                    None
                }
            }
            Expr::Binary { left, op, right } => {
                let left_ty = self.check_expr(left);
                let right_ty = self.check_expr(right);
                match op {
                    BinOp::Plus | BinOp::Minus | BinOp::Multiply | BinOp::Divide => {
                        if left_ty == right_ty && left_ty.is_some() {
                            left_ty
                        } else {
                            self.errors.push(SemanticError::TypeMismatch("Arithmetic operands must have same type".to_string()));
                            None
                        }
                    }
                    BinOp::Equal | BinOp::NotEqual | BinOp::LessThan | BinOp::GreaterThan | BinOp::LessEqual | BinOp::GreaterEqual => {
                        if left_ty == right_ty && left_ty.is_some() {
                            Some(Type::Int) // Comparisons return int
                        } else {
                            self.errors.push(SemanticError::TypeMismatch("Comparison operands must have same type".to_string()));
                            None
                        }
                    }
                }
            }
            Expr::Call { name, args } => {
                let func_info = self.functions.get(name).cloned();
                if let Some((ret_ty, param_types)) = func_info {
                    if args.len() != param_types.len() {
                        self.errors.push(SemanticError::WrongArgumentCount(name.clone(), param_types.len(), args.len()));
                        return Some(ret_ty);
                    }
                    for (i, arg) in args.iter().enumerate() {
                        let arg_ty = self.check_expr(arg);
                        if arg_ty != Some(param_types[i]) {
                            self.errors.push(SemanticError::TypeMismatch(format!("Argument {} type mismatch", i)));
                        }
                    }
                    Some(ret_ty)
                } else {
                    self.errors.push(SemanticError::UndefinedFunction(name.clone()));
                    None
                }
            }
        }
    }

    /// Looks up a variable in the current scopes.
    fn lookup_variable(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(*ty);
            }
        }
        None
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
    use crate::parser::parse;
    use crate::lexer::lex;

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
}

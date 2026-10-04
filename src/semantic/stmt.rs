use super::SemanticAnalyzer;
use crate::ast::*;
use crate::error::SemanticError;
use std::collections::HashMap;

impl SemanticAnalyzer {
    /// Checks a statement.
    pub(crate) fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::StructDef(struct_def) => {
                self.register_struct(struct_def);
            }
            Stmt::Declaration { ty, name, init } => {
                if *ty == Type::Void {
                    self.errors.push(SemanticError::TypeMismatch(
                        "Variable cannot have void type".to_string(),
                    ));
                }
                if let Type::Struct(struct_name) = ty
                    && !self.structs.contains_key(struct_name)
                {
                    self.errors.push(SemanticError::TypeMismatch(format!(
                        "Undefined struct type: {}",
                        struct_name
                    )));
                }
                if self.scopes.last().unwrap().contains_key(name)
                    || self.array_scopes.last().unwrap().contains_key(name)
                {
                    self.errors
                        .push(SemanticError::DuplicateVariable(name.clone()));
                } else {
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert(name.clone(), ty.clone());
                    if let Some(expr) = init {
                        let expr_ty = self.check_expr(expr);
                        if expr_ty.as_ref() != Some(ty) {
                            self.errors.push(SemanticError::TypeMismatch(format!(
                                "Cannot assign {:?} to {:?}",
                                expr_ty, ty
                            )));
                        }
                    }
                }
            }
            Stmt::ArrayDeclaration { ty, name, dims } => {
                if *ty == Type::Void {
                    self.errors.push(SemanticError::TypeMismatch(
                        "Array element cannot have void type".to_string(),
                    ));
                }
                if let Type::Struct(struct_name) = ty
                    && !self.structs.contains_key(struct_name)
                {
                    self.errors.push(SemanticError::TypeMismatch(format!(
                        "Undefined struct type: {}",
                        struct_name
                    )));
                }
                if dims.is_empty() || dims.contains(&0) {
                    self.errors.push(SemanticError::TypeMismatch(
                        "Array size must be greater than zero".to_string(),
                    ));
                }
                if self.scopes.last().unwrap().contains_key(name)
                    || self.array_scopes.last().unwrap().contains_key(name)
                {
                    self.errors
                        .push(SemanticError::DuplicateVariable(name.clone()));
                } else {
                    self.array_scopes
                        .last_mut()
                        .unwrap()
                        .insert(name.clone(), (ty.clone(), dims.clone()));
                }
            }
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    let expr_ty = self.check_expr(e);
                    // Only check return type if the expression type is valid (not None from undefined var)
                    if let Some(expected_ty) = &self.current_return_type {
                        if *expected_ty == Type::Void {
                            self.errors.push(SemanticError::TypeMismatch(
                                "Void function cannot return a value".to_string(),
                            ));
                        } else if let Some(actual_ty) = &expr_ty
                            && actual_ty != expected_ty
                        {
                            self.errors.push(SemanticError::TypeMismatch(format!(
                                "Return type mismatch: expected {:?}, got {:?}",
                                expected_ty, actual_ty
                            )));
                        }
                    }
                } else if let Some(expected_ty) = &self.current_return_type
                    && *expected_ty != Type::Void
                {
                    // Function expects a return value but got bare 'return'
                    self.errors.push(SemanticError::TypeMismatch(format!(
                        "Function expects return value of type {:?}",
                        expected_ty
                    )));
                }
            }
            Stmt::Block(stmts) => {
                self.scopes.push(HashMap::new());
                self.array_scopes.push(HashMap::new());
                for stmt in stmts {
                    self.check_stmt(stmt);
                }
                self.scopes.pop();
                self.array_scopes.pop();
            }
            Stmt::If { cond, then, else_ } => {
                let cond_ty = self.check_expr(cond);
                if cond_ty != Some(Type::Int) {
                    self.errors.push(SemanticError::TypeMismatch(
                        "Condition must be int".to_string(),
                    ));
                }
                self.check_stmt(then);
                if let Some(else_stmt) = else_ {
                    self.check_stmt(else_stmt);
                }
            }
            Stmt::For {
                init,
                cond,
                update,
                body,
            } => {
                self.scopes.push(HashMap::new());
                self.array_scopes.push(HashMap::new());
                if let Some(init_stmt) = init {
                    self.check_stmt(init_stmt);
                }
                if let Some(cond_expr) = cond {
                    let cond_ty = self.check_expr(cond_expr);
                    if cond_ty != Some(Type::Int) {
                        self.errors.push(SemanticError::TypeMismatch(
                            "Condition must be int".to_string(),
                        ));
                    }
                }
                if let Some(update_expr) = update {
                    self.check_expr(update_expr);
                }
                self.loop_depth += 1;
                self.check_stmt(body);
                self.loop_depth -= 1;
                self.scopes.pop();
                self.array_scopes.pop();
            }
            Stmt::While { cond, body } => {
                let cond_ty = self.check_expr(cond);
                if cond_ty != Some(Type::Int) {
                    self.errors.push(SemanticError::TypeMismatch(
                        "While condition must be an integer".to_string(),
                    ));
                }
                self.scopes.push(HashMap::new());
                self.array_scopes.push(HashMap::new());
                self.loop_depth += 1;
                self.check_stmt(body);
                self.loop_depth -= 1;
                self.scopes.pop();
                self.array_scopes.pop();
            }
            Stmt::Break => {
                if self.loop_depth == 0 {
                    self.errors.push(SemanticError::TypeMismatch(
                        "Break statement outside of loop".to_string(),
                    ));
                }
            }
            Stmt::Continue => {
                if self.loop_depth == 0 {
                    self.errors.push(SemanticError::TypeMismatch(
                        "Continue statement outside of loop".to_string(),
                    ));
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr);
            }
        }
    }
}

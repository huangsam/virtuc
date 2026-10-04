use super::SemanticAnalyzer;
use crate::ast::*;
use crate::error::SemanticError;

impl SemanticAnalyzer {
    /// Checks an expression and returns its type.
    pub(crate) fn check_expr(&mut self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(_) => Some(Type::Int),
                Literal::Float(_) => Some(Type::Float),
                Literal::String(_) => Some(Type::String),
            },
            Expr::Identifier(name) => {
                if let Some(ty) = self.lookup_variable(name) {
                    Some(ty)
                } else if let Some((elem_ty, _)) = self.lookup_array(name) {
                    Some(Type::Pointer(Box::new(elem_ty)))
                } else {
                    self.errors
                        .push(SemanticError::UndefinedVariable(name.clone()));
                    None
                }
            }
            Expr::Unary { op, expr } => match op {
                UnaryOp::Neg => {
                    let ty = self.check_expr(expr)?;
                    if ty == Type::Int || ty == Type::Float {
                        Some(ty)
                    } else {
                        self.errors.push(SemanticError::TypeMismatch(
                            "Unary minus requires int or float operand".to_string(),
                        ));
                        None
                    }
                }
                UnaryOp::Not => {
                    let ty = self.check_expr(expr)?;
                    if ty == Type::Int || ty == Type::Float {
                        Some(Type::Int)
                    } else {
                        self.errors.push(SemanticError::TypeMismatch(
                            "Logical NOT requires int or float operand".to_string(),
                        ));
                        None
                    }
                }
                UnaryOp::AddrOf => match &**expr {
                    Expr::Identifier(name) => {
                        if let Some(ty) = self.lookup_variable(name) {
                            Some(Type::Pointer(Box::new(ty)))
                        } else if let Some((elem_ty, _)) = self.lookup_array(name) {
                            Some(Type::Pointer(Box::new(elem_ty)))
                        } else {
                            self.errors
                                .push(SemanticError::UndefinedVariable(name.clone()));
                            None
                        }
                    }
                    Expr::Index { name, indices } => {
                        for index in indices {
                            let idx_ty = self.check_expr(index);
                            if idx_ty != Some(Type::Int) {
                                self.errors.push(SemanticError::TypeMismatch(
                                    "Array index must be an integer".to_string(),
                                ));
                            }
                        }
                        if let Some((elem_ty, dims)) = self.lookup_array(name) {
                            if indices.len() > dims.len() {
                                self.errors.push(SemanticError::TypeMismatch(format!(
                                    "Too many indices for array '{}': expected at most {}, got {}",
                                    name,
                                    dims.len(),
                                    indices.len()
                                )));
                                None
                            } else {
                                Some(Type::Pointer(Box::new(elem_ty)))
                            }
                        } else if let Some(Type::Pointer(elem_ty)) = self.lookup_variable(name) {
                            if indices.len() != 1 {
                                self.errors.push(SemanticError::TypeMismatch(format!(
                                    "Cannot index pointer '{}' with {} indices",
                                    name,
                                    indices.len()
                                )));
                                None
                            } else {
                                Some(Type::Pointer(elem_ty))
                            }
                        } else {
                            self.errors
                                .push(SemanticError::UndefinedVariable(name.clone()));
                            None
                        }
                    }
                    Expr::Unary {
                        op: UnaryOp::Deref,
                        expr: inner,
                    } => self.check_expr(inner),
                    Expr::MemberAccess { .. } | Expr::ArrowAccess { .. } => {
                        let member_ty = self.check_expr(expr)?;
                        Some(Type::Pointer(Box::new(member_ty)))
                    }
                    _ => {
                        self.errors.push(SemanticError::TypeMismatch(
                            "Cannot take address of rvalue".to_string(),
                        ));
                        None
                    }
                },
                UnaryOp::Deref => {
                    let ty = self.check_expr(expr)?;
                    match ty {
                        Type::Pointer(inner) => Some(*inner),
                        _ => {
                            self.errors.push(SemanticError::TypeMismatch(
                                "Cannot dereference non-pointer type".to_string(),
                            ));
                            None
                        }
                    }
                }
            },
            Expr::LogicalAnd { left, right } | Expr::LogicalOr { left, right } => {
                let left_ty = self.check_expr(left);
                let right_ty = self.check_expr(right);
                if (left_ty == Some(Type::Int) || left_ty == Some(Type::Float))
                    && (right_ty == Some(Type::Int) || right_ty == Some(Type::Float))
                {
                    Some(Type::Int)
                } else {
                    self.errors.push(SemanticError::TypeMismatch(
                        "Logical operator operands must be numeric".to_string(),
                    ));
                    None
                }
            }
            Expr::Binary { left, op, right } => {
                let left_ty = self.check_expr(left);
                let right_ty = self.check_expr(right);
                match op {
                    BinOp::Plus => {
                        if left_ty == right_ty && left_ty.is_some() {
                            left_ty
                        } else if let (Some(Type::Pointer(_)), Some(Type::Int)) =
                            (&left_ty, &right_ty)
                        {
                            left_ty
                        } else if let (Some(Type::Int), Some(Type::Pointer(_))) =
                            (&left_ty, &right_ty)
                        {
                            right_ty
                        } else {
                            self.errors.push(SemanticError::TypeMismatch(
                                "Arithmetic operands must have same type or pointer + int"
                                    .to_string(),
                            ));
                            None
                        }
                    }
                    BinOp::Minus => {
                        if left_ty == right_ty && left_ty.is_some() {
                            if let Some(Type::Pointer(_)) = left_ty {
                                Some(Type::Int)
                            } else {
                                left_ty
                            }
                        } else if let (Some(Type::Pointer(_)), Some(Type::Int)) =
                            (&left_ty, &right_ty)
                        {
                            left_ty
                        } else {
                            self.errors.push(SemanticError::TypeMismatch(
                                "Arithmetic operands must have same type or pointer - int"
                                    .to_string(),
                            ));
                            None
                        }
                    }
                    BinOp::Multiply | BinOp::Divide => {
                        if left_ty == right_ty && left_ty.is_some() {
                            left_ty
                        } else {
                            self.errors.push(SemanticError::TypeMismatch(
                                "Arithmetic operands must have same type".to_string(),
                            ));
                            None
                        }
                    }
                    BinOp::Modulo => {
                        if left_ty == Some(Type::Int) && right_ty == Some(Type::Int) {
                            Some(Type::Int)
                        } else {
                            self.errors.push(SemanticError::TypeMismatch(
                                "Modulo operands must both be integers".to_string(),
                            ));
                            None
                        }
                    }
                    BinOp::Equal
                    | BinOp::NotEqual
                    | BinOp::LessThan
                    | BinOp::GreaterThan
                    | BinOp::LessEqual
                    | BinOp::GreaterEqual => {
                        if left_ty == right_ty && left_ty.is_some() {
                            Some(Type::Int) // Comparisons return int
                        } else {
                            self.errors.push(SemanticError::TypeMismatch(
                                "Comparison operands must have same type".to_string(),
                            ));
                            None
                        }
                    }
                }
            }
            Expr::Call { name, args } => {
                let func_info = self.functions.get(name).cloned();
                if let Some((ret_ty, param_types, is_variadic)) = func_info {
                    if !is_variadic {
                        if args.len() != param_types.len() {
                            self.errors.push(SemanticError::WrongArgumentCount(
                                name.clone(),
                                param_types.len(),
                                args.len(),
                            ));
                            return Some(ret_ty);
                        }
                    } else if args.len() < param_types.len() {
                        self.errors.push(SemanticError::WrongArgumentCount(
                            name.clone(),
                            param_types.len(),
                            args.len(),
                        ));
                        return Some(ret_ty);
                    }
                    for (i, arg) in args.iter().enumerate().take(param_types.len()) {
                        let arg_ty = self.check_expr(arg);
                        if arg_ty.as_ref() != Some(&param_types[i]) {
                            self.errors.push(SemanticError::TypeMismatch(format!(
                                "Argument {} type mismatch",
                                i
                            )));
                        }
                    }
                    Some(ret_ty)
                } else {
                    self.errors
                        .push(SemanticError::UndefinedFunction(name.clone()));
                    None
                }
            }
            Expr::Assignment { name, value } => {
                let value_ty = self.check_expr(value);
                if let Some(var_ty) = self.lookup_variable(name) {
                    if value_ty.as_ref() != Some(&var_ty) {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Cannot assign {:?} to {:?}",
                            value_ty, var_ty
                        )));
                    }
                    Some(var_ty)
                } else {
                    self.errors
                        .push(SemanticError::UndefinedVariable(name.clone()));
                    None
                }
            }
            Expr::Index { name, indices } => {
                for index in indices {
                    let idx_ty = self.check_expr(index);
                    if idx_ty != Some(Type::Int) {
                        self.errors.push(SemanticError::TypeMismatch(
                            "Array index must be an integer".to_string(),
                        ));
                    }
                }
                if let Some((elem_ty, dims)) = self.lookup_array(name) {
                    if indices.len() > dims.len() {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Too many indices for array '{}': expected at most {}, got {}",
                            name,
                            dims.len(),
                            indices.len()
                        )));
                        None
                    } else if indices.len() < dims.len() {
                        Some(Type::Pointer(Box::new(elem_ty)))
                    } else {
                        Some(elem_ty)
                    }
                } else if let Some(Type::Pointer(elem_ty)) = self.lookup_variable(name) {
                    if indices.len() != 1 {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Cannot index pointer '{}' with {} indices",
                            name,
                            indices.len()
                        )));
                        None
                    } else {
                        Some(*elem_ty)
                    }
                } else {
                    self.errors
                        .push(SemanticError::UndefinedVariable(name.clone()));
                    None
                }
            }
            Expr::IndexAssignment {
                name,
                indices,
                value,
            } => {
                for index in indices {
                    let idx_ty = self.check_expr(index);
                    if idx_ty != Some(Type::Int) {
                        self.errors.push(SemanticError::TypeMismatch(
                            "Array index must be an integer".to_string(),
                        ));
                    }
                }
                let val_ty = self.check_expr(value);
                if let Some((elem_ty, dims)) = self.lookup_array(name) {
                    if indices.len() != dims.len() {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Invalid number of indices for array assignment to '{}': expected {}, got {}",
                            name,
                            dims.len(),
                            indices.len()
                        )));
                    } else if val_ty.as_ref() != Some(&elem_ty) {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Cannot assign {:?} to array element of type {:?}",
                            val_ty, elem_ty
                        )));
                    }
                    Some(elem_ty)
                } else if let Some(Type::Pointer(elem_ty)) = self.lookup_variable(name) {
                    if indices.len() != 1 {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Cannot index pointer '{}' with {} indices",
                            name,
                            indices.len()
                        )));
                    } else if val_ty.as_ref() != Some(&*elem_ty) {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Cannot assign {:?} to pointer element of type {:?}",
                            val_ty, elem_ty
                        )));
                    }
                    Some(*elem_ty)
                } else {
                    self.errors
                        .push(SemanticError::UndefinedVariable(name.clone()));
                    None
                }
            }
            Expr::DerefAssignment { target, value } => {
                let target_ty = self.check_expr(target);
                let value_ty = self.check_expr(value);
                if let Some(Type::Pointer(inner)) = target_ty {
                    if value_ty.as_ref() != Some(&*inner) {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Cannot assign {:?} to dereferenced pointer of type {:?}",
                            value_ty, inner
                        )));
                    }
                    Some(*inner)
                } else {
                    self.errors.push(SemanticError::TypeMismatch(
                        "Cannot dereference non-pointer type".to_string(),
                    ));
                    None
                }
            }
            Expr::MemberAccess { target, field } => {
                let target_ty = self.check_expr(target)?;
                match target_ty {
                    Type::Struct(name) => {
                        if let Some(def) = self.structs.get(&name) {
                            if let Some(f) = def.fields.iter().find(|f| f.name == *field) {
                                Some(f.ty.clone())
                            } else {
                                self.errors.push(SemanticError::TypeMismatch(format!(
                                    "Struct '{}' has no field '{}'",
                                    name, field
                                )));
                                None
                            }
                        } else {
                            self.errors.push(SemanticError::TypeMismatch(format!(
                                "Undefined struct type: {}",
                                name
                            )));
                            None
                        }
                    }
                    _ => {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Cannot access field '{}' of non-struct type {:?}",
                            field, target_ty
                        )));
                        None
                    }
                }
            }
            Expr::ArrowAccess { target, field } => {
                let target_ty = self.check_expr(target)?;
                match target_ty {
                    Type::Pointer(inner) => match *inner {
                        Type::Struct(name) => {
                            if let Some(def) = self.structs.get(&name) {
                                if let Some(f) = def.fields.iter().find(|f| f.name == *field) {
                                    Some(f.ty.clone())
                                } else {
                                    self.errors.push(SemanticError::TypeMismatch(format!(
                                        "Struct '{}' has no field '{}'",
                                        name, field
                                    )));
                                    None
                                }
                            } else {
                                self.errors.push(SemanticError::TypeMismatch(format!(
                                    "Undefined struct type: {}",
                                    name
                                )));
                                None
                            }
                        }
                        _ => {
                            self.errors.push(SemanticError::TypeMismatch(format!(
                                "Arrow operator requires pointer to struct, got pointer to {:?}",
                                inner
                            )));
                            None
                        }
                    },
                    _ => {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Arrow operator requires pointer to struct, got {:?}",
                            target_ty
                        )));
                        None
                    }
                }
            }
            Expr::MemberAssignment {
                target,
                field,
                value,
            } => {
                let target_ty = self.check_expr(target);
                let val_ty = self.check_expr(value);
                match target_ty {
                    Some(Type::Struct(name)) => {
                        if let Some(def) = self.structs.get(&name) {
                            if let Some(f) = def.fields.iter().find(|f| f.name == *field) {
                                if val_ty.as_ref() != Some(&f.ty) {
                                    self.errors.push(SemanticError::TypeMismatch(format!(
                                        "Cannot assign {:?} to field '{}' of type {:?}",
                                        val_ty, field, f.ty
                                    )));
                                }
                                Some(f.ty.clone())
                            } else {
                                self.errors.push(SemanticError::TypeMismatch(format!(
                                    "Struct '{}' has no field '{}'",
                                    name, field
                                )));
                                None
                            }
                        } else {
                            self.errors.push(SemanticError::TypeMismatch(format!(
                                "Undefined struct type: {}",
                                name
                            )));
                            None
                        }
                    }
                    _ => {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Cannot assign to field '{}' of non-struct type {:?}",
                            field, target_ty
                        )));
                        None
                    }
                }
            }
            Expr::ArrowAssignment {
                target,
                field,
                value,
            } => {
                let target_ty = self.check_expr(target);
                let val_ty = self.check_expr(value);
                match target_ty {
                    Some(Type::Pointer(inner)) => match *inner {
                        Type::Struct(name) => {
                            if let Some(def) = self.structs.get(&name) {
                                if let Some(f) = def.fields.iter().find(|f| f.name == *field) {
                                    if val_ty.as_ref() != Some(&f.ty) {
                                        self.errors.push(SemanticError::TypeMismatch(format!(
                                            "Cannot assign {:?} to field '{}' of type {:?}",
                                            val_ty, field, f.ty
                                        )));
                                    }
                                    Some(f.ty.clone())
                                } else {
                                    self.errors.push(SemanticError::TypeMismatch(format!(
                                        "Struct '{}' has no field '{}'",
                                        name, field
                                    )));
                                    None
                                }
                            } else {
                                self.errors.push(SemanticError::TypeMismatch(format!(
                                    "Undefined struct type: {}",
                                    name
                                )));
                                None
                            }
                        }
                        _ => {
                            self.errors.push(SemanticError::TypeMismatch(format!(
                                "Arrow operator requires pointer to struct, got pointer to {:?}",
                                inner
                            )));
                            None
                        }
                    },
                    _ => {
                        self.errors.push(SemanticError::TypeMismatch(format!(
                            "Arrow operator requires pointer to struct, got {:?}",
                            target_ty
                        )));
                        None
                    }
                }
            }
        }
    }
}

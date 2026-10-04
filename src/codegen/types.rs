use inkwell::AddressSpace;
use inkwell::types::{BasicType, BasicTypeEnum};

use super::CodeGenerator;
use crate::ast::*;

impl<'ctx> CodeGenerator<'ctx> {
    /// Determines the type of an expression during codegen.
    pub(crate) fn expr_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(_) => Some(Type::Int),
                Literal::Float(_) => Some(Type::Float),
                Literal::String(_) => Some(Type::String),
            },
            Expr::Identifier(name) => {
                if let Some((_, ty)) = self.variables.get(name) {
                    Some(ty.clone())
                } else if let Some((_, ty, _)) = self.arrays.get(name) {
                    Some(Type::Pointer(Box::new(ty.clone())))
                } else {
                    None
                }
            }
            Expr::Unary { op, expr } => match op {
                UnaryOp::Neg => self.expr_type(expr),
                UnaryOp::Not => Some(Type::Int),
                UnaryOp::AddrOf => {
                    let inner_ty = self.expr_type(expr)?;
                    Some(Type::Pointer(Box::new(inner_ty)))
                }
                UnaryOp::Deref => {
                    let inner_ty = self.expr_type(expr)?;
                    match inner_ty {
                        Type::Pointer(p) => Some(*p),
                        _ => None,
                    }
                }
            },
            Expr::Binary { left, op, right } => match op {
                BinOp::Plus => {
                    let left_ty = self.expr_type(left)?;
                    let right_ty = self.expr_type(right)?;
                    if let Type::Pointer(_) = left_ty {
                        Some(left_ty)
                    } else if let Type::Pointer(_) = right_ty {
                        Some(right_ty)
                    } else {
                        Some(left_ty)
                    }
                }
                BinOp::Minus => {
                    let left_ty = self.expr_type(left)?;
                    let right_ty = self.expr_type(right)?;
                    if let (Type::Pointer(_), Type::Pointer(_)) = (&left_ty, &right_ty) {
                        Some(Type::Int)
                    } else if let Type::Pointer(_) = left_ty {
                        Some(left_ty)
                    } else {
                        Some(left_ty)
                    }
                }
                BinOp::Modulo
                | BinOp::Equal
                | BinOp::NotEqual
                | BinOp::LessThan
                | BinOp::GreaterThan
                | BinOp::LessEqual
                | BinOp::GreaterEqual => Some(Type::Int),
                _ => self.expr_type(left),
            },
            Expr::LogicalAnd { .. } | Expr::LogicalOr { .. } => Some(Type::Int),
            Expr::Call { name, .. } => self.function_return_types.get(name).cloned(),
            Expr::Assignment { name, .. } => self.variables.get(name).map(|(_, ty)| ty.clone()),
            Expr::Index { name, indices } => {
                if let Some((_, ty, dims)) = self.arrays.get(name) {
                    if indices.len() < dims.len() {
                        Some(Type::Pointer(Box::new(ty.clone())))
                    } else {
                        Some(ty.clone())
                    }
                } else if let Some((_, Type::Pointer(elem_ty))) = self.variables.get(name) {
                    Some(*elem_ty.clone())
                } else {
                    None
                }
            }
            Expr::IndexAssignment { name, indices, .. } => {
                if let Some((_, ty, dims)) = self.arrays.get(name) {
                    if indices.len() < dims.len() {
                        Some(Type::Pointer(Box::new(ty.clone())))
                    } else {
                        Some(ty.clone())
                    }
                } else if let Some((_, Type::Pointer(elem_ty))) = self.variables.get(name) {
                    Some(*elem_ty.clone())
                } else {
                    None
                }
            }
            Expr::DerefAssignment { target, .. } => {
                let target_ty = self.expr_type(target)?;
                match target_ty {
                    Type::Pointer(p) => Some(*p),
                    _ => None,
                }
            }
            Expr::MemberAccess { target, field } => {
                let target_ty = self.expr_type(target)?;
                match target_ty {
                    Type::Struct(name) => {
                        let def = self.struct_defs.get(&name)?;
                        let f = def.fields.iter().find(|f| f.name == *field)?;
                        Some(f.ty.clone())
                    }
                    _ => None,
                }
            }
            Expr::ArrowAccess { target, field } => {
                let target_ty = self.expr_type(target)?;
                match target_ty {
                    Type::Pointer(inner) => match *inner {
                        Type::Struct(name) => {
                            let def = self.struct_defs.get(&name)?;
                            let f = def.fields.iter().find(|f| f.name == *field)?;
                            Some(f.ty.clone())
                        }
                        _ => None,
                    },
                    _ => None,
                }
            }
            Expr::MemberAssignment { target, field, .. } => {
                let target_ty = self.expr_type(target)?;
                match target_ty {
                    Type::Struct(name) => {
                        let def = self.struct_defs.get(&name)?;
                        let f = def.fields.iter().find(|f| f.name == *field)?;
                        Some(f.ty.clone())
                    }
                    _ => None,
                }
            }
            Expr::ArrowAssignment { target, field, .. } => {
                let target_ty = self.expr_type(target)?;
                match target_ty {
                    Type::Pointer(inner) => match *inner {
                        Type::Struct(name) => {
                            let def = self.struct_defs.get(&name)?;
                            let f = def.fields.iter().find(|f| f.name == *field)?;
                            Some(f.ty.clone())
                        }
                        _ => None,
                    },
                    _ => None,
                }
            }
        }
    }

    /// Creates an LLVM ArrayType for multi-dimensional fixed-size arrays.
    pub(crate) fn llvm_array_type(
        &self,
        ty: &Type,
        dims: &[usize],
    ) -> inkwell::types::ArrayType<'ctx> {
        assert!(!dims.is_empty());
        let mut current_arr = self.llvm_type(ty).array_type(*dims.last().unwrap() as u32);
        for &dim in dims[..dims.len() - 1].iter().rev() {
            current_arr = current_arr.array_type(dim as u32);
        }
        current_arr
    }

    /// Maps C type to LLVM type.
    pub(crate) fn llvm_type(&self, ty: &Type) -> BasicTypeEnum<'ctx> {
        match ty {
            Type::Int => self.context.i64_type().into(),
            Type::Float => self.context.f64_type().into(),
            Type::String => self.context.ptr_type(AddressSpace::default()).into(),
            Type::Pointer(_) => self.context.ptr_type(AddressSpace::default()).into(),
            Type::Struct(name) => {
                let struct_ty = self.struct_types.get(name).unwrap_or_else(|| {
                    panic!("Undefined struct in codegen: {}", name);
                });
                (*struct_ty).into()
            }
            Type::Void => panic!("Void cannot be converted to BasicTypeEnum"),
        }
    }
}

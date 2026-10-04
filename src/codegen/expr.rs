use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, PointerValue};
use inkwell::{FloatPredicate, IntPredicate};

use super::CodeGenerator;
use crate::ast::*;
use crate::error::CodegenError;

impl<'ctx> CodeGenerator<'ctx> {
    /// Generates an expression.
    pub(crate) fn generate_expr(
        &mut self,
        expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, CodegenError> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(n) => Ok(self.context.i64_type().const_int(*n as u64, false).into()),
                Literal::Float(f) => Ok(self.context.f64_type().const_float(*f).into()),
                Literal::String(s) => {
                    let global = self
                        .builder
                        .build_global_string_ptr(s, "str")
                        .map_err(|e| CodegenError(format!("Builder error: {:?}", e)))?;
                    Ok(global.as_pointer_value().into())
                }
            },
            Expr::Identifier(name) => {
                if let Some((ptr, ty)) = self.variables.get(name) {
                    Ok(self
                        .builder
                        .build_load(self.llvm_type(ty), *ptr, name)
                        .unwrap())
                } else if let Some(&(ptr, ref ty, ref dims)) = self.arrays.get(name) {
                    let arr_llvm_ty = self.llvm_array_type(ty, dims);
                    let zeros = vec![self.context.i64_type().const_zero(); dims.len() + 1];
                    let elem_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(arr_llvm_ty, ptr, &zeros, "decay")
                            .unwrap()
                    };
                    Ok(elem_ptr.into())
                } else {
                    Err(CodegenError(format!("Undefined variable: {}", name)))
                }
            }
            Expr::Unary { op, expr } => match op {
                UnaryOp::Neg => {
                    let val = self.generate_expr(expr)?;
                    if val.get_type().is_int_type() {
                        Ok(self
                            .builder
                            .build_int_neg(val.into_int_value(), "neg")
                            .unwrap()
                            .into())
                    } else if val.get_type().is_float_type() {
                        Ok(self
                            .builder
                            .build_float_neg(val.into_float_value(), "fneg")
                            .unwrap()
                            .into())
                    } else {
                        Err(CodegenError("Cannot negate non-numeric type".to_string()))
                    }
                }
                UnaryOp::Not => {
                    let val = self.generate_expr(expr)?;
                    let bool_val = self.to_bool(val, "not.bool");
                    let not_bool = self
                        .builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            bool_val,
                            self.context.bool_type().const_zero(),
                            "not.inv",
                        )
                        .unwrap();
                    let res = self
                        .builder
                        .build_int_z_extend(not_bool, self.context.i64_type(), "not.ext")
                        .unwrap();
                    Ok(res.into())
                }
                UnaryOp::AddrOf => match &**expr {
                    Expr::Identifier(name) => {
                        if let Some((ptr, _)) = self.variables.get(name) {
                            Ok((*ptr).into())
                        } else if let Some(&(ptr, ref ty, ref dims)) = self.arrays.get(name) {
                            let arr_llvm_ty = self.llvm_array_type(ty, dims);
                            let zeros = vec![self.context.i64_type().const_zero(); dims.len() + 1];
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(arr_llvm_ty, ptr, &zeros, "arr_decay")
                                    .unwrap()
                            };
                            Ok(elem_ptr.into())
                        } else {
                            Err(CodegenError(format!("Undefined variable for & : {}", name)))
                        }
                    }
                    Expr::Index { name, indices } => {
                        let mut idx_ints = Vec::with_capacity(indices.len());
                        for idx in indices {
                            let idx_val = self.generate_expr(idx)?;
                            idx_ints.push(idx_val.into_int_value());
                        }
                        if let Some(&(ptr, ref ty, ref dims)) = self.arrays.get(name) {
                            let arr_llvm_ty = self.llvm_array_type(ty, dims);
                            let mut gep_indices = vec![self.context.i64_type().const_zero()];
                            gep_indices.extend(idx_ints);
                            while gep_indices.len() < dims.len() + 1 {
                                gep_indices.push(self.context.i64_type().const_zero());
                            }
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(arr_llvm_ty, ptr, &gep_indices, "arr_idx")
                                    .unwrap()
                            };
                            Ok(elem_ptr.into())
                        } else if let Some(&(ptr, ref var_ty)) = self.variables.get(name) {
                            if let Type::Pointer(elem_ty) = var_ty {
                                if idx_ints.len() != 1 {
                                    return Err(CodegenError(
                                        "Cannot index pointer with multiple indices".to_string(),
                                    ));
                                }
                                let elem_llvm_ty = self.llvm_type(elem_ty);
                                let base_ptr = self
                                    .builder
                                    .build_load(self.llvm_type(var_ty), ptr, "ptr_base")
                                    .unwrap()
                                    .into_pointer_value();
                                let elem_ptr = unsafe {
                                    self.builder
                                        .build_in_bounds_gep(
                                            elem_llvm_ty,
                                            base_ptr,
                                            &[idx_ints[0]],
                                            "ptr_idx",
                                        )
                                        .unwrap()
                                };
                                Ok(elem_ptr.into())
                            } else {
                                Err(CodegenError(format!(
                                    "Cannot index non-pointer/array: {}",
                                    name
                                )))
                            }
                        } else {
                            Err(CodegenError(format!("Undefined identifier: {}", name)))
                        }
                    }
                    Expr::Unary {
                        op: UnaryOp::Deref,
                        expr: inner,
                    } => self.generate_expr(inner),
                    Expr::MemberAccess { .. } | Expr::ArrowAccess { .. } => {
                        let (field_ptr, _) = self.get_member_pointer(expr)?;
                        Ok(field_ptr.into())
                    }
                    _ => Err(CodegenError("Cannot take address of rvalue".to_string())),
                },
                UnaryOp::Deref => {
                    let target_ty = self.expr_type(expr).ok_or_else(|| {
                        CodegenError("Cannot determine type of dereference operand".to_string())
                    })?;
                    let elem_ty = match target_ty {
                        Type::Pointer(inner) => *inner,
                        _ => {
                            return Err(CodegenError("Cannot dereference non-pointer".to_string()));
                        }
                    };
                    let elem_llvm_ty = self.llvm_type(&elem_ty);
                    let ptr_val = self.generate_expr(expr)?;
                    let ptr = ptr_val.into_pointer_value();
                    let loaded = self.builder.build_load(elem_llvm_ty, ptr, "deref").unwrap();
                    Ok(loaded)
                }
            },
            Expr::LogicalAnd { left, right } => {
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();

                let lhs_val = self.generate_expr(left)?;
                let lhs_block = self.builder.get_insert_block().unwrap();
                let lhs_bool = self.to_bool(lhs_val, "land.lhs.bool");

                let rhs_block = self.context.append_basic_block(current_fn, "land.rhs");
                let merge_block = self.context.append_basic_block(current_fn, "land.merge");

                self.builder
                    .build_conditional_branch(lhs_bool, rhs_block, merge_block)
                    .unwrap();

                // RHS block
                self.builder.position_at_end(rhs_block);
                let rhs_val = self.generate_expr(right)?;
                let rhs_eval_block = self.builder.get_insert_block().unwrap();
                let rhs_bool = self.to_bool(rhs_val, "land.rhs.bool");
                self.builder
                    .build_unconditional_branch(merge_block)
                    .unwrap();

                // Merge block
                self.builder.position_at_end(merge_block);
                let phi = self
                    .builder
                    .build_phi(self.context.bool_type(), "land.res")
                    .unwrap();
                phi.add_incoming(&[
                    (&self.context.bool_type().const_zero(), lhs_block),
                    (&rhs_bool, rhs_eval_block),
                ]);
                let res_i64 = self
                    .builder
                    .build_int_z_extend(
                        phi.as_basic_value().into_int_value(),
                        self.context.i64_type(),
                        "land.ext",
                    )
                    .unwrap();
                Ok(res_i64.into())
            }
            Expr::LogicalOr { left, right } => {
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();

                let lhs_val = self.generate_expr(left)?;
                let lhs_block = self.builder.get_insert_block().unwrap();
                let lhs_bool = self.to_bool(lhs_val, "lor.lhs.bool");

                let rhs_block = self.context.append_basic_block(current_fn, "lor.rhs");
                let merge_block = self.context.append_basic_block(current_fn, "lor.merge");

                self.builder
                    .build_conditional_branch(lhs_bool, merge_block, rhs_block)
                    .unwrap();

                // RHS block
                self.builder.position_at_end(rhs_block);
                let rhs_val = self.generate_expr(right)?;
                let rhs_eval_block = self.builder.get_insert_block().unwrap();
                let rhs_bool = self.to_bool(rhs_val, "lor.rhs.bool");
                self.builder
                    .build_unconditional_branch(merge_block)
                    .unwrap();

                // Merge block
                self.builder.position_at_end(merge_block);
                let phi = self
                    .builder
                    .build_phi(self.context.bool_type(), "lor.res")
                    .unwrap();
                phi.add_incoming(&[
                    (&self.context.bool_type().const_int(1, false), lhs_block),
                    (&rhs_bool, rhs_eval_block),
                ]);
                let res_i64 = self
                    .builder
                    .build_int_z_extend(
                        phi.as_basic_value().into_int_value(),
                        self.context.i64_type(),
                        "lor.ext",
                    )
                    .unwrap();
                Ok(res_i64.into())
            }
            Expr::Binary { left, op, right } => {
                let left_val = self.generate_expr(left)?;
                let right_val = self.generate_expr(right)?;
                match op {
                    BinOp::Plus => {
                        if left_val.is_pointer_value() {
                            let elem_ty = match self.expr_type(left) {
                                Some(Type::Pointer(inner)) => *inner,
                                _ => Type::Int,
                            };
                            let elem_llvm_ty = self.llvm_type(&elem_ty);
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm_ty,
                                        left_val.into_pointer_value(),
                                        &[right_val.into_int_value()],
                                        "ptr_add",
                                    )
                                    .unwrap()
                            };
                            Ok(elem_ptr.into())
                        } else if right_val.is_pointer_value() {
                            let elem_ty = match self.expr_type(right) {
                                Some(Type::Pointer(inner)) => *inner,
                                _ => Type::Int,
                            };
                            let elem_llvm_ty = self.llvm_type(&elem_ty);
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm_ty,
                                        right_val.into_pointer_value(),
                                        &[left_val.into_int_value()],
                                        "ptr_add",
                                    )
                                    .unwrap()
                            };
                            Ok(elem_ptr.into())
                        } else if left_val.get_type().is_int_type() {
                            Ok(self
                                .builder
                                .build_int_add(
                                    left_val.into_int_value(),
                                    right_val.into_int_value(),
                                    "add",
                                )
                                .unwrap()
                                .into())
                        } else {
                            Ok(self
                                .builder
                                .build_float_add(
                                    left_val.into_float_value(),
                                    right_val.into_float_value(),
                                    "fadd",
                                )
                                .unwrap()
                                .into())
                        }
                    }
                    BinOp::Minus => {
                        if left_val.is_pointer_value() && right_val.get_type().is_int_type() {
                            let elem_ty = match self.expr_type(left) {
                                Some(Type::Pointer(inner)) => *inner,
                                _ => Type::Int,
                            };
                            let elem_llvm_ty = self.llvm_type(&elem_ty);
                            let neg_idx = self
                                .builder
                                .build_int_neg(right_val.into_int_value(), "neg_idx")
                                .unwrap();
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm_ty,
                                        left_val.into_pointer_value(),
                                        &[neg_idx],
                                        "ptr_sub",
                                    )
                                    .unwrap()
                            };
                            Ok(elem_ptr.into())
                        } else if left_val.get_type().is_int_type() {
                            Ok(self
                                .builder
                                .build_int_sub(
                                    left_val.into_int_value(),
                                    right_val.into_int_value(),
                                    "sub",
                                )
                                .unwrap()
                                .into())
                        } else {
                            Ok(self
                                .builder
                                .build_float_sub(
                                    left_val.into_float_value(),
                                    right_val.into_float_value(),
                                    "fsub",
                                )
                                .unwrap()
                                .into())
                        }
                    }
                    BinOp::Multiply => {
                        if left_val.get_type().is_int_type() {
                            Ok(self
                                .builder
                                .build_int_mul(
                                    left_val.into_int_value(),
                                    right_val.into_int_value(),
                                    "mul",
                                )
                                .unwrap()
                                .into())
                        } else {
                            Ok(self
                                .builder
                                .build_float_mul(
                                    left_val.into_float_value(),
                                    right_val.into_float_value(),
                                    "fmul",
                                )
                                .unwrap()
                                .into())
                        }
                    }
                    BinOp::Divide => {
                        if left_val.get_type().is_int_type() {
                            Ok(self
                                .builder
                                .build_int_signed_div(
                                    left_val.into_int_value(),
                                    right_val.into_int_value(),
                                    "div",
                                )
                                .unwrap()
                                .into())
                        } else {
                            Ok(self
                                .builder
                                .build_float_div(
                                    left_val.into_float_value(),
                                    right_val.into_float_value(),
                                    "fdiv",
                                )
                                .unwrap()
                                .into())
                        }
                    }
                    BinOp::Modulo => {
                        if left_val.get_type().is_int_type() {
                            Ok(self
                                .builder
                                .build_int_signed_rem(
                                    left_val.into_int_value(),
                                    right_val.into_int_value(),
                                    "rem",
                                )
                                .unwrap()
                                .into())
                        } else {
                            Err(CodegenError(
                                "Modulo operator requires integer operands".to_string(),
                            ))
                        }
                    }
                    BinOp::Equal => {
                        if left_val.is_pointer_value() {
                            let l_int = self
                                .builder
                                .build_ptr_to_int(
                                    left_val.into_pointer_value(),
                                    self.context.i64_type(),
                                    "ptr_int",
                                )
                                .unwrap();
                            let r_int = self
                                .builder
                                .build_ptr_to_int(
                                    right_val.into_pointer_value(),
                                    self.context.i64_type(),
                                    "ptr_int",
                                )
                                .unwrap();
                            let cmp = self
                                .builder
                                .build_int_compare(IntPredicate::EQ, l_int, r_int, "eq")
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        } else if left_val.get_type().is_int_type() {
                            let cmp = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::EQ,
                                    left_val.into_int_value(),
                                    right_val.into_int_value(),
                                    "eq",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        } else {
                            let cmp = self
                                .builder
                                .build_float_compare(
                                    FloatPredicate::OEQ,
                                    left_val.into_float_value(),
                                    right_val.into_float_value(),
                                    "feq",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        }
                    }
                    BinOp::NotEqual => {
                        if left_val.is_pointer_value() {
                            let l_int = self
                                .builder
                                .build_ptr_to_int(
                                    left_val.into_pointer_value(),
                                    self.context.i64_type(),
                                    "ptr_int",
                                )
                                .unwrap();
                            let r_int = self
                                .builder
                                .build_ptr_to_int(
                                    right_val.into_pointer_value(),
                                    self.context.i64_type(),
                                    "ptr_int",
                                )
                                .unwrap();
                            let cmp = self
                                .builder
                                .build_int_compare(IntPredicate::NE, l_int, r_int, "ne")
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        } else if left_val.get_type().is_int_type() {
                            let cmp = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::NE,
                                    left_val.into_int_value(),
                                    right_val.into_int_value(),
                                    "ne",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        } else {
                            let cmp = self
                                .builder
                                .build_float_compare(
                                    FloatPredicate::ONE,
                                    left_val.into_float_value(),
                                    right_val.into_float_value(),
                                    "fne",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        }
                    }
                    BinOp::LessThan => {
                        if left_val.get_type().is_int_type() {
                            let cmp = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::SLT,
                                    left_val.into_int_value(),
                                    right_val.into_int_value(),
                                    "lt",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        } else {
                            let cmp = self
                                .builder
                                .build_float_compare(
                                    FloatPredicate::OLT,
                                    left_val.into_float_value(),
                                    right_val.into_float_value(),
                                    "flt",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        }
                    }
                    BinOp::GreaterThan => {
                        if left_val.get_type().is_int_type() {
                            let cmp = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::SGT,
                                    left_val.into_int_value(),
                                    right_val.into_int_value(),
                                    "gt",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        } else {
                            let cmp = self
                                .builder
                                .build_float_compare(
                                    FloatPredicate::OGT,
                                    left_val.into_float_value(),
                                    right_val.into_float_value(),
                                    "fgt",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        }
                    }
                    BinOp::LessEqual => {
                        if left_val.get_type().is_int_type() {
                            let cmp = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::SLE,
                                    left_val.into_int_value(),
                                    right_val.into_int_value(),
                                    "le",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        } else {
                            let cmp = self
                                .builder
                                .build_float_compare(
                                    FloatPredicate::OLE,
                                    left_val.into_float_value(),
                                    right_val.into_float_value(),
                                    "fle",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        }
                    }
                    BinOp::GreaterEqual => {
                        if left_val.get_type().is_int_type() {
                            let cmp = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::SGE,
                                    left_val.into_int_value(),
                                    right_val.into_int_value(),
                                    "ge",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        } else {
                            let cmp = self
                                .builder
                                .build_float_compare(
                                    FloatPredicate::OGE,
                                    left_val.into_float_value(),
                                    right_val.into_float_value(),
                                    "fge",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "bool_ext")
                                .unwrap()
                                .into())
                        }
                    }
                }
            }
            Expr::Call { name, args } => {
                // For simplicity, assume function exists
                let function = self.module.get_function(name).unwrap();
                let arg_values: Vec<BasicMetadataValueEnum> = args
                    .iter()
                    .map(|arg| self.generate_expr(arg).map(|v| v.into()))
                    .collect::<Result<_, _>>()?;
                let call = self
                    .builder
                    .build_call(function, &arg_values, "call")
                    .unwrap();
                match call.try_as_basic_value().basic() {
                    Some(val) => Ok(val),
                    None => Ok(self.context.i64_type().const_zero().into()),
                }
            }
            Expr::Assignment { name, value } => {
                let val = self.generate_expr(value)?;
                if let Some((ptr, _)) = self.variables.get(name) {
                    self.builder.build_store(*ptr, val).unwrap();
                    Ok(val)
                } else {
                    Err(CodegenError(format!("Undefined variable: {}", name)))
                }
            }
            Expr::Index { name, indices } => {
                let mut idx_ints = Vec::with_capacity(indices.len());
                for idx in indices {
                    let idx_val = self.generate_expr(idx)?;
                    if idx_val.get_type().is_int_type() {
                        idx_ints.push(idx_val.into_int_value());
                    } else {
                        return Err(CodegenError("Array index must be an integer".to_string()));
                    }
                }
                if let Some(&(ptr, ref ty, ref dims)) = self.arrays.get(name) {
                    let arr_llvm_ty = self.llvm_array_type(ty, dims);
                    let mut gep_indices = vec![self.context.i64_type().const_zero()];
                    gep_indices.extend(idx_ints);
                    if gep_indices.len() < dims.len() + 1 {
                        while gep_indices.len() < dims.len() + 1 {
                            gep_indices.push(self.context.i64_type().const_zero());
                        }
                        let elem_ptr = unsafe {
                            self.builder
                                .build_in_bounds_gep(arr_llvm_ty, ptr, &gep_indices, "arr_row")
                                .unwrap()
                        };
                        Ok(elem_ptr.into())
                    } else {
                        let elem_llvm_ty = self.llvm_type(ty);
                        let elem_ptr = unsafe {
                            self.builder
                                .build_in_bounds_gep(arr_llvm_ty, ptr, &gep_indices, "arr_idx")
                                .unwrap()
                        };
                        let val = self
                            .builder
                            .build_load(elem_llvm_ty, elem_ptr, "arr_elem")
                            .unwrap();
                        Ok(val)
                    }
                } else if let Some(&(ptr, ref var_ty)) = self.variables.get(name) {
                    if let Type::Pointer(elem_ty) = var_ty {
                        if idx_ints.len() != 1 {
                            return Err(CodegenError(
                                "Cannot index pointer with multiple indices".to_string(),
                            ));
                        }
                        let elem_llvm_ty = self.llvm_type(elem_ty);
                        let base_ptr = self
                            .builder
                            .build_load(self.llvm_type(var_ty), ptr, "ptr_base")
                            .unwrap()
                            .into_pointer_value();
                        let elem_ptr = unsafe {
                            self.builder
                                .build_in_bounds_gep(
                                    elem_llvm_ty,
                                    base_ptr,
                                    &[idx_ints[0]],
                                    "ptr_idx",
                                )
                                .unwrap()
                        };
                        let val = self
                            .builder
                            .build_load(elem_llvm_ty, elem_ptr, "ptr_elem")
                            .unwrap();
                        Ok(val)
                    } else {
                        Err(CodegenError(format!(
                            "Cannot index non-pointer/non-array variable: {}",
                            name
                        )))
                    }
                } else {
                    Err(CodegenError(format!(
                        "Undefined array or pointer: {}",
                        name
                    )))
                }
            }
            Expr::IndexAssignment {
                name,
                indices,
                value,
            } => {
                let val = self.generate_expr(value)?;
                let mut idx_ints = Vec::with_capacity(indices.len());
                for idx in indices {
                    let idx_val = self.generate_expr(idx)?;
                    if idx_val.get_type().is_int_type() {
                        idx_ints.push(idx_val.into_int_value());
                    } else {
                        return Err(CodegenError("Array index must be an integer".to_string()));
                    }
                }
                if let Some(&(ptr, ref ty, ref dims)) = self.arrays.get(name) {
                    let arr_llvm_ty = self.llvm_array_type(ty, dims);
                    let mut gep_indices = vec![self.context.i64_type().const_zero()];
                    gep_indices.extend(idx_ints);
                    let elem_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(arr_llvm_ty, ptr, &gep_indices, "arr_idx")
                            .unwrap()
                    };
                    self.builder.build_store(elem_ptr, val).unwrap();
                    Ok(val)
                } else if let Some(&(ptr, ref var_ty)) = self.variables.get(name) {
                    if let Type::Pointer(elem_ty) = var_ty {
                        if idx_ints.len() != 1 {
                            return Err(CodegenError(
                                "Cannot index pointer with multiple indices".to_string(),
                            ));
                        }
                        let elem_llvm_ty = self.llvm_type(elem_ty);
                        let base_ptr = self
                            .builder
                            .build_load(self.llvm_type(var_ty), ptr, "ptr_base")
                            .unwrap()
                            .into_pointer_value();
                        let elem_ptr = unsafe {
                            self.builder
                                .build_in_bounds_gep(
                                    elem_llvm_ty,
                                    base_ptr,
                                    &[idx_ints[0]],
                                    "ptr_idx",
                                )
                                .unwrap()
                        };
                        self.builder.build_store(elem_ptr, val).unwrap();
                        Ok(val)
                    } else {
                        Err(CodegenError(format!(
                            "Cannot index non-pointer/non-array variable: {}",
                            name
                        )))
                    }
                } else {
                    Err(CodegenError(format!(
                        "Undefined array or pointer: {}",
                        name
                    )))
                }
            }
            Expr::DerefAssignment { target, value } => {
                let val = self.generate_expr(value)?;
                let ptr_val = self.generate_expr(target)?;
                let ptr = ptr_val.into_pointer_value();
                self.builder.build_store(ptr, val).unwrap();
                Ok(val)
            }
            Expr::MemberAccess { .. } | Expr::ArrowAccess { .. } => {
                let (field_ptr, field_ty) = self.get_member_pointer(expr)?;
                let field_llvm_ty = self.llvm_type(&field_ty);
                let val = self
                    .builder
                    .build_load(field_llvm_ty, field_ptr, "member")
                    .unwrap();
                Ok(val)
            }
            Expr::MemberAssignment {
                target,
                field,
                value,
            } => {
                let val = self.generate_expr(value)?;
                let (field_ptr, _) = self.get_member_pointer(&Expr::MemberAccess {
                    target: target.clone(),
                    field: field.clone(),
                })?;
                self.builder.build_store(field_ptr, val).unwrap();
                Ok(val)
            }
            Expr::ArrowAssignment {
                target,
                field,
                value,
            } => {
                let val = self.generate_expr(value)?;
                let (field_ptr, _) = self.get_member_pointer(&Expr::ArrowAccess {
                    target: target.clone(),
                    field: field.clone(),
                })?;
                self.builder.build_store(field_ptr, val).unwrap();
                Ok(val)
            }
        }
    }

    /// Gets a pointer to a struct member and its type.
    pub(crate) fn get_member_pointer(
        &mut self,
        expr: &Expr,
    ) -> Result<(PointerValue<'ctx>, Type), CodegenError> {
        match expr {
            Expr::MemberAccess { target, field } => {
                let (struct_ptr, struct_name) = self.get_struct_base_pointer(target)?;
                let struct_def =
                    self.struct_defs.get(&struct_name).cloned().ok_or_else(|| {
                        CodegenError(format!("Undefined struct: {}", struct_name))
                    })?;
                let idx = struct_def
                    .fields
                    .iter()
                    .position(|f| f.name == *field)
                    .ok_or_else(|| {
                        CodegenError(format!(
                            "Field '{}' not found in struct '{}'",
                            field, struct_name
                        ))
                    })?;
                let field_ty = struct_def.fields[idx].ty.clone();
                let struct_ty = *self.struct_types.get(&struct_name).ok_or_else(|| {
                    CodegenError(format!("Struct type not found: {}", struct_name))
                })?;
                let field_ptr = self
                    .builder
                    .build_struct_gep(struct_ty, struct_ptr, idx as u32, field)
                    .map_err(|e| CodegenError(format!("GEP error: {:?}", e)))?;
                Ok((field_ptr, field_ty))
            }
            Expr::ArrowAccess { target, field } => {
                let target_val = self.generate_expr(target)?;
                let struct_ptr = target_val.into_pointer_value();
                let target_ty = self.expr_type(target).ok_or_else(|| {
                    CodegenError("Could not determine type of arrow target".to_string())
                })?;
                let struct_name = match target_ty {
                    Type::Pointer(inner) => match *inner {
                        Type::Struct(name) => name,
                        _ => {
                            return Err(CodegenError(
                                "Arrow operator on pointer to non-struct".to_string(),
                            ));
                        }
                    },
                    _ => {
                        return Err(CodegenError("Arrow operator on non-pointer".to_string()));
                    }
                };
                let struct_def =
                    self.struct_defs.get(&struct_name).cloned().ok_or_else(|| {
                        CodegenError(format!("Undefined struct: {}", struct_name))
                    })?;
                let idx = struct_def
                    .fields
                    .iter()
                    .position(|f| f.name == *field)
                    .ok_or_else(|| {
                        CodegenError(format!(
                            "Field '{}' not found in struct '{}'",
                            field, struct_name
                        ))
                    })?;
                let field_ty = struct_def.fields[idx].ty.clone();
                let struct_ty = *self.struct_types.get(&struct_name).ok_or_else(|| {
                    CodegenError(format!("Struct type not found: {}", struct_name))
                })?;
                let field_ptr = self
                    .builder
                    .build_struct_gep(struct_ty, struct_ptr, idx as u32, field)
                    .map_err(|e| CodegenError(format!("GEP error: {:?}", e)))?;
                Ok((field_ptr, field_ty))
            }
            _ => Err(CodegenError(
                "Expected MemberAccess or ArrowAccess".to_string(),
            )),
        }
    }

    /// Resolves the pointer to the base struct and its struct name.
    pub(crate) fn get_struct_base_pointer(
        &mut self,
        target: &Expr,
    ) -> Result<(PointerValue<'ctx>, String), CodegenError> {
        match target {
            Expr::Identifier(name) => {
                if let Some((ptr, ty)) = self.variables.get(name) {
                    match ty {
                        Type::Struct(struct_name) => Ok((*ptr, struct_name.clone())),
                        _ => Err(CodegenError(format!("Variable '{}' is not a struct", name))),
                    }
                } else {
                    Err(CodegenError(format!("Undefined variable: {}", name)))
                }
            }
            Expr::MemberAccess { .. } | Expr::ArrowAccess { .. } => {
                let (ptr, ty) = self.get_member_pointer(target)?;
                match ty {
                    Type::Struct(struct_name) => Ok((ptr, struct_name)),
                    _ => Err(CodegenError("Member is not a struct".to_string())),
                }
            }
            Expr::Unary {
                op: UnaryOp::Deref,
                expr,
            } => {
                let val = self.generate_expr(expr)?;
                let ptr = val.into_pointer_value();
                let ty = self
                    .expr_type(expr)
                    .ok_or_else(|| CodegenError("Could not determine deref type".to_string()))?;
                match ty {
                    Type::Pointer(inner) => match *inner {
                        Type::Struct(struct_name) => Ok((ptr, struct_name)),
                        _ => Err(CodegenError("Deref of pointer to non-struct".to_string())),
                    },
                    _ => Err(CodegenError("Deref of non-pointer".to_string())),
                }
            }
            Expr::Index { name, indices } => {
                let mut idx_ints = Vec::with_capacity(indices.len());
                for idx in indices {
                    let idx_val = self.generate_expr(idx)?;
                    if idx_val.get_type().is_int_type() {
                        idx_ints.push(idx_val.into_int_value());
                    } else {
                        return Err(CodegenError("Array index must be an integer".to_string()));
                    }
                }
                if let Some(&(ptr, ref ty, ref dims)) = self.arrays.get(name) {
                    match ty {
                        Type::Struct(struct_name) => {
                            let arr_llvm_ty = self.llvm_array_type(ty, dims);
                            let mut gep_indices = vec![self.context.i64_type().const_zero()];
                            gep_indices.extend(idx_ints);
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        arr_llvm_ty,
                                        ptr,
                                        &gep_indices,
                                        "arr_struct_elem",
                                    )
                                    .unwrap()
                            };
                            Ok((elem_ptr, struct_name.clone()))
                        }
                        _ => Err(CodegenError(format!(
                            "Array '{}' is not an array of structs",
                            name
                        ))),
                    }
                } else {
                    Err(CodegenError(format!("Undefined array: {}", name)))
                }
            }
            other => {
                let val = self.generate_expr(other)?;
                let ty = self
                    .expr_type(other)
                    .ok_or_else(|| CodegenError("Could not determine expr type".to_string()))?;
                match ty {
                    Type::Struct(struct_name) => {
                        let llvm_ty = self.llvm_type(&Type::Struct(struct_name.clone()));
                        let temp = self.builder.build_alloca(llvm_ty, "tmp_struct").unwrap();
                        self.builder.build_store(temp, val).unwrap();
                        Ok((temp, struct_name))
                    }
                    _ => Err(CodegenError("Expression is not a struct".to_string())),
                }
            }
        }
    }

    /// Converts a basic value to an i1 boolean.
    pub(crate) fn to_bool(
        &self,
        val: BasicValueEnum<'ctx>,
        name: &str,
    ) -> inkwell::values::IntValue<'ctx> {
        if val.get_type().is_int_type() {
            self.builder
                .build_int_compare(
                    IntPredicate::NE,
                    val.into_int_value(),
                    self.context.i64_type().const_zero(),
                    name,
                )
                .unwrap()
        } else if val.get_type().is_float_type() {
            self.builder
                .build_float_compare(
                    FloatPredicate::ONE,
                    val.into_float_value(),
                    self.context.f64_type().const_zero(),
                    name,
                )
                .unwrap()
        } else {
            panic!("Non-numeric value cannot be converted to bool")
        }
    }
}

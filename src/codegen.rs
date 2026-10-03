//! # Code Generation
//!
//! This module generates LLVM Intermediate Representation (IR) from the
//! validated AST. It uses the `inkwell` crate to interface with LLVM for
//! creating optimized native executables.
//!
//! ## Code Generation Strategy
//!
//! - **Expressions**: Generate IR for arithmetic, comparison, and assignment
//! - **Statements**: Handle control flow, variable allocation, and function calls
//! - **Functions**: Create LLVM functions with proper signatures and bodies
//! - **Program**: Link all components into a complete module
//!
//! ## LLVM Integration
//!
//! Uses `inkwell` to build LLVM IR incrementally. Handles type mapping from
//! the C subset types to LLVM types, and generates efficient code with
//! optimizations enabled.

use inkwell::AddressSpace;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{InitializationConfig, Target, TargetMachine};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, PointerValue};
use inkwell::{FloatPredicate, IntPredicate};
use std::collections::HashMap;

use crate::ast::*;
use crate::error::CodegenError;

/// Code generator for LLVM IR.
pub struct CodeGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    /// Variable environment: name -> (pointer to value, type)
    variables: HashMap<String, (PointerValue<'ctx>, Type)>,
    /// Stack of loops: (break_target, continue_target)
    loop_stack: Vec<(BasicBlock<'ctx>, BasicBlock<'ctx>)>,
}

impl<'ctx> CodeGenerator<'ctx> {
    /// Creates a new code generator.
    pub fn new(context: &'ctx Context) -> Self {
        // Initialize native target to ensure we can get the default triple
        Target::initialize_native(&InitializationConfig::default()).ok();

        let module = context.create_module("virtuc");

        // Set the target triple to the host machine's triple
        let triple = TargetMachine::get_default_triple();
        module.set_triple(&triple);

        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
            loop_stack: Vec::new(),
        }
    }

    /// Generates LLVM IR for the program.
    pub fn generate(&mut self, program: &Program) -> Result<(), CodegenError> {
        for extern_func in &program.extern_functions {
            self.declare_extern_function(extern_func)?;
        }
        for function in &program.functions {
            self.generate_function(function)?;
        }
        Ok(())
    }

    /// Gets the LLVM IR as a string.
    pub fn get_ir(&self) -> String {
        self.module.print_to_string().to_string()
    }

    /// Declares an extern function.
    fn declare_extern_function(
        &mut self,
        extern_func: &ExternFunction,
    ) -> Result<(), CodegenError> {
        let param_types: Vec<BasicMetadataTypeEnum> = extern_func
            .param_types
            .iter()
            .map(|ty| self.llvm_type(*ty).into())
            .collect();
        let fn_type = match extern_func.return_ty {
            Type::Void => self
                .context
                .void_type()
                .fn_type(&param_types, extern_func.is_variadic),
            _ => self
                .llvm_type(extern_func.return_ty)
                .fn_type(&param_types, extern_func.is_variadic),
        };
        self.module.add_function(&extern_func.name, fn_type, None);
        Ok(())
    }

    /// Generates a function.
    fn generate_function(&mut self, function: &Function) -> Result<(), CodegenError> {
        // Create function type
        let param_types: Vec<BasicMetadataTypeEnum> = function
            .params
            .iter()
            .map(|(ty, _)| self.llvm_type(*ty).into())
            .collect();
        let fn_type = match function.return_ty {
            Type::Void => self.context.void_type().fn_type(&param_types, false),
            _ => self
                .llvm_type(function.return_ty)
                .fn_type(&param_types, false),
        };

        // Create function
        let llvm_function = self.module.add_function(&function.name, fn_type, None);

        // Create entry block
        let entry_block = self.context.append_basic_block(llvm_function, "entry");
        self.builder.position_at_end(entry_block);

        // Clear variables for new function
        self.variables.clear();
        self.loop_stack.clear();

        // Allocate parameters
        for (i, (ty, name)) in function.params.iter().enumerate() {
            let param = llvm_function.get_nth_param(i as u32).unwrap();
            let alloca = self.builder.build_alloca(param.get_type(), name).unwrap();
            self.builder.build_store(alloca, param).unwrap();
            self.variables.insert(name.clone(), (alloca, *ty));
        }

        // Generate function body
        self.generate_stmt(&function.body)?;

        // Check if the current block has a terminator
        let current_block = self.builder.get_insert_block().unwrap();
        if current_block.get_terminator().is_none() {
            // Add implicit return if missing
            match function.return_ty {
                Type::Int => {
                    self.builder
                        .build_return(Some(&self.context.i64_type().const_zero()))
                        .unwrap();
                }
                Type::Float => {
                    self.builder
                        .build_return(Some(&self.context.f64_type().const_zero()))
                        .unwrap();
                }
                Type::String => {
                    self.builder
                        .build_return(Some(
                            &self.context.ptr_type(AddressSpace::default()).const_null(),
                        ))
                        .unwrap();
                }
                Type::Void => {
                    self.builder.build_return(None).unwrap();
                }
            }
        }

        // Verify function
        if llvm_function.verify(true) {
            Ok(())
        } else {
            Err(CodegenError("Function verification failed".to_string()))
        }
    }

    /// Generates a statement.
    fn generate_stmt(&mut self, stmt: &Stmt) -> Result<(), CodegenError> {
        if let Some(block) = self.builder.get_insert_block() {
            if block.get_terminator().is_some() {
                // Block already terminated by return, break, or continue
                return Ok(());
            }
        }
        match stmt {
            Stmt::Declaration { ty, name, init } => {
                let llvm_ty = self.llvm_type(*ty);
                let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                self.variables.insert(name.clone(), (alloca, *ty));
                if let Some(expr) = init {
                    let value = self.generate_expr(expr)?;
                    self.builder.build_store(alloca, value).unwrap();
                }
            }
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    let value = self.generate_expr(e)?;
                    self.builder.build_return(Some(&value)).unwrap();
                } else {
                    self.builder.build_return(None).unwrap();
                }
            }
            Stmt::Block(stmts) => {
                for stmt in stmts {
                    self.generate_stmt(stmt)?;
                }
            }
            Stmt::If { cond, then, else_ } => {
                let cond_value = self.generate_expr(cond)?;
                let cond_bool = if cond_value.get_type().is_int_type() {
                    self.builder
                        .build_int_compare(
                            IntPredicate::NE,
                            cond_value.into_int_value(),
                            self.context.i64_type().const_zero(),
                            "cond",
                        )
                        .unwrap()
                } else {
                    return Err(CodegenError("Non-integer condition".to_string()));
                };

                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let then_block = self.context.append_basic_block(current_fn, "then");
                let else_block = self.context.append_basic_block(current_fn, "else");
                let merge_block = self.context.append_basic_block(current_fn, "merge");

                self.builder
                    .build_conditional_branch(cond_bool, then_block, else_block)
                    .unwrap();

                // Then block
                self.builder.position_at_end(then_block);
                self.generate_stmt(then)?;
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .unwrap();
                }

                // Else block
                self.builder.position_at_end(else_block);
                if let Some(else_stmt) = else_ {
                    self.generate_stmt(else_stmt)?;
                }
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .unwrap();
                }

                // Merge block
                self.builder.position_at_end(merge_block);
            }
            Stmt::For {
                init,
                cond,
                update,
                body,
            } => {
                // Step 1: Generate initialization statement
                if let Some(init_stmt) = init {
                    self.generate_stmt(init_stmt)?;
                }

                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();

                // Step 2: Create basic blocks
                let cond_block = self.context.append_basic_block(current_fn, "loop.cond");
                let body_block = self.context.append_basic_block(current_fn, "loop.body");
                let update_block = self.context.append_basic_block(current_fn, "loop.update");
                let after_loop = self.context.append_basic_block(current_fn, "loop.end");

                // Step 3: Branch to condition check
                self.builder.build_unconditional_branch(cond_block).unwrap();

                // Step 4: Generate condition block
                self.builder.position_at_end(cond_block);
                if let Some(cond_expr) = cond {
                    let cond_value = self.generate_expr(cond_expr)?;
                    let cond_bool = if cond_value.get_type().is_int_type() {
                        self.builder
                            .build_int_compare(
                                IntPredicate::NE,
                                cond_value.into_int_value(),
                                self.context.i64_type().const_zero(),
                                "loop.cond.bool",
                            )
                            .unwrap()
                    } else {
                        return Err(CodegenError("Loop condition must be integer".to_string()));
                    };
                    self.builder
                        .build_conditional_branch(cond_bool, body_block, after_loop)
                        .unwrap();
                } else {
                    self.builder.build_unconditional_branch(body_block).unwrap();
                }

                // Step 5: Generate body block
                self.builder.position_at_end(body_block);
                let continue_target = if update.is_some() {
                    update_block
                } else {
                    cond_block
                };
                self.loop_stack.push((after_loop, continue_target));
                self.generate_stmt(body)?;
                self.loop_stack.pop();

                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    if update.is_some() {
                        self.builder
                            .build_unconditional_branch(update_block)
                            .unwrap();
                    } else {
                        self.builder.build_unconditional_branch(cond_block).unwrap();
                    }
                }

                // Step 6: Generate update block (if exists)
                if let Some(update_expr) = update {
                    self.builder.position_at_end(update_block);
                    self.generate_expr(update_expr)?;
                    self.builder.build_unconditional_branch(cond_block).unwrap();
                }

                // Step 7: Continue after loop
                self.builder.position_at_end(after_loop);
            }
            Stmt::While { cond, body } => {
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();

                let cond_block = self.context.append_basic_block(current_fn, "while.cond");
                let body_block = self.context.append_basic_block(current_fn, "while.body");
                let after_loop = self.context.append_basic_block(current_fn, "while.end");

                self.builder.build_unconditional_branch(cond_block).unwrap();

                // Condition block
                self.builder.position_at_end(cond_block);
                let cond_val = self.generate_expr(cond)?;
                let cond_bool = if cond_val.get_type().is_int_type() {
                    self.builder
                        .build_int_compare(
                            IntPredicate::NE,
                            cond_val.into_int_value(),
                            self.context.i64_type().const_zero(),
                            "while.cond.bool",
                        )
                        .unwrap()
                } else {
                    return Err(CodegenError("While condition must be integer".to_string()));
                };
                self.builder
                    .build_conditional_branch(cond_bool, body_block, after_loop)
                    .unwrap();

                // Body block
                self.builder.position_at_end(body_block);
                self.loop_stack.push((after_loop, cond_block));
                self.generate_stmt(body)?;
                self.loop_stack.pop();

                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(cond_block).unwrap();
                }

                // After loop block
                self.builder.position_at_end(after_loop);
            }
            Stmt::Break => {
                if let Some((break_target, _)) = self.loop_stack.last() {
                    self.builder.build_unconditional_branch(*break_target).unwrap();
                } else {
                    return Err(CodegenError("Break outside of loop".to_string()));
                }
            }
            Stmt::Continue => {
                if let Some((_, continue_target)) = self.loop_stack.last() {
                    self.builder
                        .build_unconditional_branch(*continue_target)
                        .unwrap();
                } else {
                    return Err(CodegenError("Continue outside of loop".to_string()));
                }
            }
            Stmt::Expr(expr) => {
                self.generate_expr(expr)?;
            }
        }
        Ok(())
    }

    /// Generates an expression.
    fn generate_expr(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, CodegenError> {
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
                        .build_load(self.llvm_type(*ty), *ptr, name)
                        .unwrap())
                } else {
                    Err(CodegenError(format!("Undefined variable: {}", name)))
                }
            }
            Expr::Unary { op, expr } => {
                let val = self.generate_expr(expr)?;
                match op {
                    UnaryOp::Neg => {
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
                        if val.get_type().is_int_type() {
                            let cmp = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::EQ,
                                    val.into_int_value(),
                                    self.context.i64_type().const_zero(),
                                    "not",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "not_ext")
                                .unwrap()
                                .into())
                        } else if val.get_type().is_float_type() {
                            let cmp = self
                                .builder
                                .build_float_compare(
                                    FloatPredicate::OEQ,
                                    val.into_float_value(),
                                    self.context.f64_type().const_zero(),
                                    "fnot",
                                )
                                .unwrap();
                            Ok(self
                                .builder
                                .build_int_z_extend(cmp, self.context.i64_type(), "not_ext")
                                .unwrap()
                                .into())
                        } else {
                            Err(CodegenError(
                                "Cannot apply logical NOT to non-numeric type".to_string(),
                            ))
                        }
                    }
                }
            }
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
                self.builder.build_unconditional_branch(merge_block).unwrap();

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
                self.builder.build_unconditional_branch(merge_block).unwrap();

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
                        if left_val.get_type().is_int_type() {
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
                        if left_val.get_type().is_int_type() {
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
                            return Err(CodegenError(
                                "Modulo operator requires integer operands".to_string(),
                            ));
                        }
                    }
                    BinOp::Equal => {
                        if left_val.get_type().is_int_type() {
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
                        if left_val.get_type().is_int_type() {
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
        }
    }

    /// Converts a basic value to an i1 boolean.
    fn to_bool(&self, val: BasicValueEnum<'ctx>, name: &str) -> inkwell::values::IntValue<'ctx> {
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

    /// Maps C type to LLVM type.
    fn llvm_type(&self, ty: Type) -> BasicTypeEnum<'ctx> {
        match ty {
            Type::Int => self.context.i64_type().into(),
            Type::Float => self.context.f64_type().into(),
            Type::String => self.context.ptr_type(AddressSpace::default()).into(),
            Type::Void => panic!("Void cannot be converted to BasicTypeEnum"),
        }
    }
}

/// Generates LLVM IR for the program.
pub fn generate_ir(program: &Program) -> Result<String, CodegenError> {
    let context = Context::create();
    let mut generator = CodeGenerator::new(&context);
    generator.generate(program)?;
    Ok(generator.get_ir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    #[test]
    fn test_generate_simple_function() {
        let input = "int add(int a, int b) { return a + b; }";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let ir = generate_ir(&ast).unwrap();
        // Check that IR contains expected elements
        assert!(ir.contains("define i64 @add(i64 %0, i64 %1)"));
        assert!(ir.contains("add i64"));
        assert!(ir.contains("ret i64"));
    }
}

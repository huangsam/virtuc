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
    /// Array environment: name -> (pointer to array, element type, dims)
    arrays: HashMap<String, (PointerValue<'ctx>, Type, Vec<usize>)>,
    /// Function return types
    function_return_types: HashMap<String, Type>,
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
            arrays: HashMap::new(),
            function_return_types: HashMap::new(),
            loop_stack: Vec::new(),
        }
    }

    /// Generates LLVM IR for the program.
    pub fn generate(&mut self, program: &Program) -> Result<(), CodegenError> {
        for header in &program.includes {
            for ext in crate::header_registry::externs_for_header(header) {
                self.function_return_types
                    .insert(ext.name.clone(), ext.return_ty);
            }
        }
        for extern_func in &program.extern_functions {
            self.function_return_types
                .insert(extern_func.name.clone(), extern_func.return_ty.clone());
        }
        for function in &program.functions {
            self.function_return_types
                .insert(function.name.clone(), function.return_ty.clone());
        }
        for extern_func in &program.extern_functions {
            self.declare_extern_function(extern_func)?;
        }
        for function in &program.functions {
            self.generate_function(function)?;
        }
        Ok(())
    }

    /// Determines the type of an expression during codegen.
    fn expr_type(&self, expr: &Expr) -> Option<Type> {
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
        }
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
            .map(|ty| self.llvm_type(ty).into())
            .collect();
        let fn_type = match extern_func.return_ty {
            Type::Void => self
                .context
                .void_type()
                .fn_type(&param_types, extern_func.is_variadic),
            _ => self
                .llvm_type(&extern_func.return_ty)
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
            .map(|(ty, _)| self.llvm_type(ty).into())
            .collect();
        let fn_type = match function.return_ty {
            Type::Void => self.context.void_type().fn_type(&param_types, false),
            _ => self
                .llvm_type(&function.return_ty)
                .fn_type(&param_types, false),
        };

        // Create function
        let llvm_function = self.module.add_function(&function.name, fn_type, None);

        // Create entry block
        let entry_block = self.context.append_basic_block(llvm_function, "entry");
        self.builder.position_at_end(entry_block);

        // Clear variables for new function
        self.variables.clear();
        self.arrays.clear();
        self.loop_stack.clear();

        // Allocate parameters
        for (i, (ty, name)) in function.params.iter().enumerate() {
            let param = llvm_function.get_nth_param(i as u32).unwrap();
            let alloca = self.builder.build_alloca(param.get_type(), name).unwrap();
            self.builder.build_store(alloca, param).unwrap();
            self.variables.insert(name.clone(), (alloca, ty.clone()));
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
                Type::Pointer(_) => {
                    self.builder
                        .build_return(Some(
                            &self.context.ptr_type(AddressSpace::default()).const_null(),
                        ))
                        .unwrap();
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
        if let Some(block) = self.builder.get_insert_block()
            && block.get_terminator().is_some()
        {
            // Block already terminated by return, break, or continue
            return Ok(());
        }
        match stmt {
            Stmt::Declaration { ty, name, init } => {
                let llvm_ty = self.llvm_type(ty);
                let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                self.variables.insert(name.clone(), (alloca, ty.clone()));
                if let Some(expr) = init {
                    let value = self.generate_expr(expr)?;
                    self.builder.build_store(alloca, value).unwrap();
                }
            }
            Stmt::ArrayDeclaration { ty, name, dims } => {
                let arr_llvm_ty = self.llvm_array_type(ty, dims);
                let alloca = self.builder.build_alloca(arr_llvm_ty, name).unwrap();
                self.arrays
                    .insert(name.clone(), (alloca, ty.clone(), dims.clone()));
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
                self.loop_stack.push((after_loop, update_block));
                self.generate_stmt(body)?;
                self.loop_stack.pop();

                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder
                        .build_unconditional_branch(update_block)
                        .unwrap();
                }

                // Step 6: Generate update block
                self.builder.position_at_end(update_block);
                if let Some(update_expr) = update {
                    self.generate_expr(update_expr)?;
                }
                self.builder.build_unconditional_branch(cond_block).unwrap();

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
                    self.builder
                        .build_unconditional_branch(*break_target)
                        .unwrap();
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

    /// Creates an LLVM ArrayType for multi-dimensional fixed-size arrays.
    fn llvm_array_type(&self, ty: &Type, dims: &[usize]) -> inkwell::types::ArrayType<'ctx> {
        assert!(!dims.is_empty());
        let mut current_arr = self.llvm_type(ty).array_type(*dims.last().unwrap() as u32);
        for &dim in dims[..dims.len() - 1].iter().rev() {
            current_arr = current_arr.array_type(dim as u32);
        }
        current_arr
    }

    /// Maps C type to LLVM type.
    fn llvm_type(&self, ty: &Type) -> BasicTypeEnum<'ctx> {
        match ty {
            Type::Int => self.context.i64_type().into(),
            Type::Float => self.context.f64_type().into(),
            Type::String => self.context.ptr_type(AddressSpace::default()).into(),
            Type::Pointer(_) => self.context.ptr_type(AddressSpace::default()).into(),
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

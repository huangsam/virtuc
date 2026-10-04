use inkwell::IntPredicate;
use inkwell::types::BasicTypeEnum;

use super::CodeGenerator;
use crate::ast::*;
use crate::error::CodegenError;

impl<'ctx> CodeGenerator<'ctx> {
    /// Generates a statement.
    pub(crate) fn generate_stmt(&mut self, stmt: &Stmt) -> Result<(), CodegenError> {
        if let Some(block) = self.builder.get_insert_block()
            && block.get_terminator().is_some()
        {
            // Block already terminated by return, break, or continue
            return Ok(());
        }
        match stmt {
            Stmt::StructDef(struct_def) => {
                if !self.struct_types.contains_key(&struct_def.name) {
                    let struct_ty = self.context.opaque_struct_type(&struct_def.name);
                    self.struct_types.insert(struct_def.name.clone(), struct_ty);
                    self.struct_defs
                        .insert(struct_def.name.clone(), struct_def.clone());
                    let field_types: Vec<BasicTypeEnum<'ctx>> = struct_def
                        .fields
                        .iter()
                        .map(|f| self.llvm_type(&f.ty))
                        .collect();
                    struct_ty.set_body(&field_types, false);
                }
            }
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
}

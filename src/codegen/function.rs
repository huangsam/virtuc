use inkwell::AddressSpace;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};

use super::CodeGenerator;
use crate::ast::*;
use crate::error::CodegenError;

impl<'ctx> CodeGenerator<'ctx> {
    pub(crate) fn declare_extern_function(
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
    pub(crate) fn generate_function(&mut self, function: &Function) -> Result<(), CodegenError> {
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
                Type::Struct(ref name) => {
                    let struct_ty = *self.struct_types.get(name).unwrap();
                    self.builder
                        .build_return(Some(&struct_ty.const_zero()))
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
}

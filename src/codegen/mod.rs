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

mod expr;
mod function;
mod stmt;
mod types;

use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{InitializationConfig, Target, TargetMachine};
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::PointerValue;
use std::collections::HashMap;

use crate::ast::*;
use crate::error::CodegenError;

/// Code generator for LLVM IR.
pub struct CodeGenerator<'ctx> {
    pub(crate) context: &'ctx Context,
    pub(crate) module: Module<'ctx>,
    pub(crate) builder: Builder<'ctx>,
    /// Struct LLVM types
    pub(crate) struct_types: HashMap<String, StructType<'ctx>>,
    /// Struct definitions
    pub(crate) struct_defs: HashMap<String, StructDef>,
    /// Variable environment: name -> (pointer to value, type)
    pub(crate) variables: HashMap<String, (PointerValue<'ctx>, Type)>,
    /// Array environment: name -> (pointer to array, element type, dims)
    pub(crate) arrays: HashMap<String, (PointerValue<'ctx>, Type, Vec<usize>)>,
    /// Function return types
    pub(crate) function_return_types: HashMap<String, Type>,
    /// Stack of loops: (break_target, continue_target)
    pub(crate) loop_stack: Vec<(BasicBlock<'ctx>, BasicBlock<'ctx>)>,
}

impl<'ctx> CodeGenerator<'ctx> {
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
            struct_types: HashMap::new(),
            struct_defs: HashMap::new(),
            variables: HashMap::new(),
            arrays: HashMap::new(),
            function_return_types: HashMap::new(),
            loop_stack: Vec::new(),
        }
    }

    /// Generates LLVM IR for the program.
    pub fn generate(&mut self, program: &Program) -> Result<(), CodegenError> {
        // Register top-level struct declarations
        for struct_def in &program.structs {
            let struct_ty = self.context.opaque_struct_type(&struct_def.name);
            self.struct_types.insert(struct_def.name.clone(), struct_ty);
            self.struct_defs
                .insert(struct_def.name.clone(), struct_def.clone());
        }
        for struct_def in &program.structs {
            let field_types: Vec<BasicTypeEnum<'ctx>> = struct_def
                .fields
                .iter()
                .map(|f| self.llvm_type(&f.ty))
                .collect();
            let struct_ty = self.struct_types.get(&struct_def.name).unwrap();
            struct_ty.set_body(&field_types, false);
        }

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

    /// Gets the LLVM IR as a string.
    pub fn get_ir(&self) -> String {
        self.module.print_to_string().to_string()
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

    #[test]
    fn test_generate_struct() {
        let input = "
            struct Point {
                int x;
                int y;
            };

            int main() {
                struct Point p;
                p.x = 10;
                p.y = 20;
                struct Point* ptr = &p;
                ptr->x = 30;
                return p.x + ptr->y;
            }
        ";
        let tokens = lex(input).unwrap();
        let ast = parse(&tokens).unwrap();
        let ir = generate_ir(&ast).unwrap();
        assert!(ir.contains("%Point = type { i64, i64 }"));
        assert!(ir.contains("getelementptr"));
    }
}

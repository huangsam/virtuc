use super::SemanticAnalyzer;
use crate::ast::*;
use crate::error::SemanticError;
use std::collections::HashSet;

impl SemanticAnalyzer {
    /// Collects struct definitions into the global symbol table.
    pub(crate) fn collect_structs(&mut self, program: &Program) {
        for struct_def in &program.structs {
            self.register_struct(struct_def);
        }
    }

    /// Registers a struct definition and checks for duplicate fields.
    pub(crate) fn register_struct(&mut self, struct_def: &StructDef) {
        if self.structs.contains_key(&struct_def.name) {
            self.errors
                .push(SemanticError::DuplicateVariable(struct_def.name.clone()));
            return;
        }
        let mut field_names = HashSet::new();
        for field in &struct_def.fields {
            if !field_names.insert(&field.name) {
                self.errors
                    .push(SemanticError::DuplicateVariable(field.name.clone()));
            }
            if field.ty == Type::Void {
                self.errors.push(SemanticError::TypeMismatch(
                    "Struct field cannot have void type".to_string(),
                ));
            }
        }
        self.structs
            .insert(struct_def.name.clone(), struct_def.clone());
    }

    /// Collects function declarations into the global symbol table.
    pub(crate) fn collect_functions(&mut self, program: &Program) {
        for function in &program.functions {
            let param_types: Vec<Type> = function.params.iter().map(|(ty, _)| ty.clone()).collect();
            if self.functions.contains_key(&function.name) {
                self.errors
                    .push(SemanticError::DuplicateVariable(function.name.clone()));
            } else {
                self.functions.insert(
                    function.name.clone(),
                    (function.return_ty.clone(), param_types, false),
                );
            }
        }
        for extern_func in &program.extern_functions {
            if self.functions.contains_key(&extern_func.name) {
                self.errors
                    .push(SemanticError::DuplicateVariable(extern_func.name.clone()));
            } else {
                self.functions.insert(
                    extern_func.name.clone(),
                    (
                        extern_func.return_ty.clone(),
                        extern_func.param_types.clone(),
                        extern_func.is_variadic,
                    ),
                );
            }
        }

        // Handle includes: map known headers to builtin externs
        for header in &program.includes {
            for ext in crate::header_registry::externs_for_header(header) {
                if let std::collections::hash_map::Entry::Vacant(e) = self.functions.entry(ext.name)
                {
                    e.insert((ext.return_ty.clone(), ext.param_types, ext.is_variadic));
                }
            }
        }
    }

    /// Looks up a variable in the current scopes.
    pub(crate) fn lookup_variable(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    /// Looks up an array in the current scopes.
    pub(crate) fn lookup_array(&self, name: &str) -> Option<(Type, Vec<usize>)> {
        for scope in self.array_scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info.clone());
            }
        }
        None
    }
}

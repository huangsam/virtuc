use super::common::compile_source;

#[test]
fn test_non_variadic_wrong_arity() {
    let source = r#"
        extern int foo(int);

        int main() {
            foo();
            return 0;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_printf_without_declaration_fails() {
    let source = r#"
        int main() {
            printf("No decl\n");
            return 0;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_undefined_variable_fails() {
    let source = r#"
        int main() {
            return undeclared_var;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_type_mismatch_assignment_fails() {
    let source = r#"
        int main() {
            int x = "incompatible_string";
            return 0;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_duplicate_variable_in_same_scope_fails() {
    let source = r#"
        int main() {
            int x = 1;
            int x = 2;
            return x;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_break_outside_loop_fails() {
    let source = r#"
        int main() {
            break;
            return 0;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_continue_outside_loop_fails() {
    let source = r#"
        int main() {
            continue;
            return 0;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_void_function_returning_value_fails() {
    let source = r#"
        void foo() {
            return 42;
        }

        int main() {
            foo();
            return 0;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_non_void_function_missing_return_value_fails() {
    let source = r#"
        int foo() {
            return;
        }

        int main() {
            return foo();
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_modulo_on_float_fails() {
    let source = r#"
        int main() {
            float a = 5.5;
            float b = 2.0;
            float c = a % b;
            return 0;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_zero_sized_array_fails() {
    let source = r#"
        int main() {
            int arr[0];
            return 0;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_calling_undefined_function_fails() {
    let source = r#"
        int main() {
            non_existent_function();
            return 0;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_lexer_invalid_character_fails() {
    let source = r#"
        int main() {
            int x = @;
            return 0;
        }
    "#;

    assert!(compile_source(source).is_err());
}

#[test]
fn test_syntax_error_missing_semicolon_fails() {
    let source = r#"
        int main() {
            int x = 5
            return x;
        }
    "#;

    assert!(compile_source(source).is_err());
}

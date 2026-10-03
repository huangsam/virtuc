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

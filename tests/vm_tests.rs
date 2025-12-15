use virtuc::lexer::lex;
use virtuc::parser::parse;
use virtuc::semantic::analyze;
use virtuc::vm::{VM, Value};

fn run_vm(source: &str) -> Value {
    let tokens = lex(source).expect("Lexing failed");
    let ast = parse(&tokens).expect("Parsing failed");
    let errors = analyze(&ast);
    assert!(errors.is_empty(), "Semantic errors: {:?}", errors);

    let mut vm = VM::new(&ast);
    vm.run().expect("VM execution failed")
}

#[test]
fn test_vm_simple_add() {
    let source = r#"
        int main() {
            return 10 + 32;
        }
    "#;
    let result = run_vm(source);
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_vm_function_call() {
    let source = r#"
        int add(int a, int b) {
            return a + b;
        }

        int main() {
            return add(10, 20);
        }
    "#;
    let result = run_vm(source);
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_vm_control_flow() {
    let source = r#"
        int main() {
            int a = 10;
            if (a > 5) {
                return 1;
            } else {
                return 0;
            }
        }
    "#;
    let result = run_vm(source);
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_vm_loop() {
    let source = r#"
        int main() {
            int sum = 0;
            for (int i = 0; i < 5; i = i + 1) {
                sum = sum + i;
            }
            return sum;
        }
    "#;
    let result = run_vm(source);
    assert_eq!(result, Value::Int(10)); // 0+1+2+3+4 = 10
}

#[test]
fn test_vm_recursion() {
    let source = r#"
        int fib(int n) {
            if (n <= 1) {
                return n;
            }
            return fib(n - 1) + fib(n - 2);
        }

        int main() {
            return fib(6);
        }
    "#;
    let result = run_vm(source);
    assert_eq!(result, Value::Int(8)); // 0, 1, 1, 2, 3, 5, 8
}

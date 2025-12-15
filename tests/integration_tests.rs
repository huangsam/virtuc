use std::process::Command;
use tempfile::TempDir;
use virtuc::compile;

#[test]
fn test_compile_and_run_simple_program() {
    // Setup temp directory
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_prog");

    // Source code
    let source = r#"
        int add(int a, int b) {
            return a + b;
        }

        int main() {
            return add(30, 12);
        }
    "#;

    // Compile
    compile(source, &output_path).expect("Compilation failed");

    // Run the generated executable
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    // Check exit code (30 + 12 = 42)
    assert_eq!(status.code(), Some(42));
}

#[test]
fn test_compile_and_run_control_flow() {
    // Setup temp directory
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_flow");

    // Source code
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

    // Compile
    compile(source, &output_path).expect("Compilation failed");

    // Run the generated executable
    let status = Command::new(&output_path)
        .status()
        .expect("failed to run generated executable");

    // Check exit code
    assert_eq!(status.code(), Some(1));
}

#[test]
fn test_compile_and_run_with_printf() {
    // Setup temp directory
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_printf");

    // Source code
    let source = r#"
        extern int printf(string, ...);

        int main() {
            printf("Hello, World!\n");
            return 42;
        }
    "#;

    // Compile
    compile(source, &output_path).expect("Compilation failed");

    // Run the generated executable and capture output
    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    // Check exit code
    assert_eq!(output.status.code(), Some(42));

    // Check stdout contains "Hello, World!"
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello, World!"));
}

#[test]
fn test_compile_and_run_with_include() {
    // Setup temp directory
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_include");

    // Source code using include
    let source = r#"
        #include <stdio.h>

        int main() {
            printf("Hello from include!\n");
            return 7;
        }
    "#;

    // Compile
    compile(source, &output_path).expect("Compilation failed");

    // Run the generated executable and capture output
    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    // Check exit code
    assert_eq!(output.status.code(), Some(7));

    // Check stdout contains "Hello from include!"
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from include!"));
}

#[test]
fn test_printf_with_integer() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_printf_int");

    let source = r#"
        extern int printf(string, ...);

        int main() {
            printf("Number: %d\n", 7);
            return 0;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");

    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Number: 7"));
}

#[test]
fn test_printf_with_multiple_args_include() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_printf_multi");

    let source = r#"
        #include <stdio.h>

        int main() {
            printf("%s %d\n", "Hi", 10);
            return 0;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");

    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hi 10"));
}

#[test]
fn test_non_variadic_wrong_arity() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_wrong_arity");

    let source = r#"
        extern int foo(int);

        int main() {
            foo();
            return 0;
        }
    "#;

    assert!(compile(source, &output_path).is_err());
}

#[test]
fn test_printf_without_declaration_fails() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_no_decl");

    let source = r#"
        int main() {
            printf("No decl\n");
            return 0;
        }
    "#;

    assert!(compile(source, &output_path).is_err());
}

#[test]
fn test_duplicate_includes() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_dup_include");

    let source = r#"
        #include <stdio.h>
        #include <stdio.h>

        int main() {
            printf("Duplicate include\n");
            return 0;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let output = Command::new(&output_path).output().expect("failed to run");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Duplicate include"));
}

#[test]
fn test_variadic_format_mix() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_variadic_mix");

    let source = r#"
        #include <stdio.h>

        int main() {
            printf("%s %d %x\n", "val", 42, 255);
            return 0;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let output = Command::new(&output_path).output().expect("failed to run");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("val 42 ff") || stdout.contains("val 42 FF"));
}

#[test]
fn test_printf_many_args() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_printf_many");

    let source = r#"
        #include <stdio.h>

        int main() {
            printf("%d %d %d %d %d %d %d %d %d %d\n", 1,2,3,4,5,6,7,8,9,10);
            return 0;
        }
    "#;

    compile(source, &output_path).expect("Compilation failed");
    let output = Command::new(&output_path).output().expect("failed to run");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1 2 3 4 5 6 7 8 9 10"));
}

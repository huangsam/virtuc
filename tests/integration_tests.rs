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

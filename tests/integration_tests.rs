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

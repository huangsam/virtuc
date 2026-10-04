use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cli_compile_default_output() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let input_path = temp_dir.path().join("prog.c");
    let default_output = temp_dir.path().join("prog.out");

    let source = r#"
        int main() {
            return 42;
        }
    "#;
    fs::write(&input_path, source).expect("failed to write source");

    let status = Command::new(env!("CARGO_BIN_EXE_vcc"))
        .arg(input_path.to_str().unwrap())
        .status()
        .expect("failed to run vcc CLI");

    assert!(status.success());
    assert!(default_output.exists());

    let run_status = Command::new(&default_output)
        .status()
        .expect("failed to run compiled binary");
    assert_eq!(run_status.code(), Some(42));
}

#[test]
fn test_cli_compile_custom_output() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let input_path = temp_dir.path().join("input.c");
    let custom_output = temp_dir.path().join("custom_app");

    let source = r#"
        #include <stdio.h>

        int main() {
            printf("Custom output test\n");
            return 0;
        }
    "#;
    fs::write(&input_path, source).expect("failed to write source");

    let status = Command::new(env!("CARGO_BIN_EXE_vcc"))
        .args([
            input_path.to_str().unwrap(),
            "-o",
            custom_output.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run vcc CLI");

    assert!(status.success());
    assert!(custom_output.exists());

    let output = Command::new(&custom_output)
        .output()
        .expect("failed to run compiled binary");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Custom output test"));
}

#[test]
fn test_cli_nonexistent_input_fails() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let nonexistent = temp_dir.path().join("does_not_exist.c");

    let status = Command::new(env!("CARGO_BIN_EXE_vcc"))
        .arg(nonexistent.to_str().unwrap())
        .status()
        .expect("failed to run vcc CLI");

    assert!(!status.success());
}

#[test]
fn test_cli_invalid_c_source_fails() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let input_path = temp_dir.path().join("bad.c");

    let source = r#"
        int main() {
            undeclared_var = 123;
            return 0;
        }
    "#;
    fs::write(&input_path, source).expect("failed to write source");

    let status = Command::new(env!("CARGO_BIN_EXE_vcc"))
        .arg(input_path.to_str().unwrap())
        .status()
        .expect("failed to run vcc CLI");

    assert_eq!(status.code(), Some(1));
}

#[test]
fn test_cli_missing_arguments_fails() {
    let status = Command::new(env!("CARGO_BIN_EXE_vcc"))
        .status()
        .expect("failed to run vcc CLI");

    assert!(!status.success());
}

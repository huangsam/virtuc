use std::process::Command;
use tempfile::TempDir;
use virtuc::compile;

pub struct ExecutionResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    #[allow(dead_code)]
    pub stderr: String,
}

/// Compiles C subset source code to a temporary native binary and executes it.
pub fn run_source(source: &str) -> ExecutionResult {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_bin");

    compile(source, &output_path).expect("Compilation failed");

    let output = Command::new(&output_path)
        .output()
        .expect("failed to run generated executable");

    ExecutionResult {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Attempts compilation of C subset source code without running it.
pub fn compile_source(source: &str) -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let output_path = temp_dir.path().join("test_bin");
    compile(source, &output_path)
}

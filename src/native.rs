use std::{
    env,
    fs,
    path::PathBuf,
    process::Command,
};

pub fn generate_executable(ir: &str) -> Result<Vec<u8>, String> {
    // Since we will feed LLVM IR into clang, we need to store the IR in a file
    let workspace = create_workspace()?;
    let ir_path = workspace.join("program.ll");

    #[cfg(windows)]
    let executable_path = workspace.join("program.exe");

    #[cfg(not(windows))]
    let executable_path = workspace.join("program");

    fs::write(&ir_path, ir)
        .map_err(|error| format!("[ERROR] Cannot save LLVM IR in a file: {error}"))?;

    let clang = env::var_os("WISEL_CUSTOM_CLANG")
        .map(PathBuf::from)
        // Fallback to the system-wide 'clang'
        .unwrap_or_else(|| PathBuf::from("clang"));

    // 'clang program.ll -o program'
    let output = Command::new(&clang)
        .arg(&ir_path)
        .arg("-o")
        .arg(&executable_path)
        .output()
        .map_err(|error| {
            format!("[ERROR] Cannot run clang: {error}")
        })?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        return Err(format!("Clang failed:\n{stdout}\n{stderr}"));
    }

    // Executable bytes
    fs::read(&executable_path)
        .map_err(|error| format!("[ERROR] Cannot read executable: {error}"))
}

fn create_workspace() -> Result<PathBuf, String> {
    let process_id = std::process::id();
    let path = env::temp_dir().join(format!("__wisel-output-{process_id}"));

    let _ = fs::remove_dir_all(&path);

    fs::create_dir_all(&path)
        .map_err(|e| format!("Cannot create workspace: {e}"))?;

    Ok(path)
}
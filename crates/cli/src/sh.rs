use std::process::Command;

/// Runs a shell command and waits for completion
/// 
/// The command string is executed via `sh -c` and the function waits
/// for the command to complete. Returns an error if the command fails
/// to execute or returns a non-zero exit status.
/// 
/// # Arguments
/// 
/// * `cmd` - The shell command to execute as a string
/// 
/// # Returns
/// 
/// Returns `Ok(())` if the command executes successfully, or an error
/// if the command fails to execute or returns a non-zero exit status.
pub fn run(cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .status()
        .map_err(|e| format!("Failed to execute command: {}", e))?;
    
    if !status.success() {
        return Err(format!("Command failed with exit code: {:?}", status.code()).into());
    }
    
    Ok(())
}

/// Spawns a shell command without waiting for completion
/// 
/// The command string is executed via `sh -c` and the function spawns
/// the process without waiting. Returns an error if the command fails
/// to spawn.
/// 
/// # Arguments
/// 
/// * `cmd` - The shell command to execute as a string
/// 
/// # Returns
/// 
/// Returns `Ok(())` if the command spawns successfully, or an error
/// if the command fails to spawn.
pub fn spawn(cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;
    
    Ok(())
}

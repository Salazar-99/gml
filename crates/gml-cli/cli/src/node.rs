use chrono::Utc;
use gml_core::{NodeRequest, NodeDetails};
use gml_core::ssh;
use gml_core::state::{GmlState, NodeEntry};
use std::process::{Command, Stdio};
use std::env;
use std::time::Duration;
use std::path::Path;
use std::fs;
use sysinfo::System;
use indicatif::ProgressBar;
use humantime::parse_duration;
use dirs;
use serde_json;

use crate::config;
use crate::providers;
use crate::spinner;
use crate::sh;

pub async fn handle_create_node(provider: String, instance_type: String, timeout: String, region: Option<String>, name: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = spinner::create_spinner();

    ensure_daemon_running(&spinner).await?;

    // Parse config from ~/.gml/config.toml
    let config = config::parse_config()?;

    // Try to get config for the specified provider
    let provider_config = config.get_provider(&provider)
        .ok_or_else(|| format!("Provider '{}' not found in config", provider))?;

    // Use the config to create a provider handle
    let provider_handle = providers::create_provider_handle(
        &provider,
        provider_config,
        region,
        config.ssh_public_key.clone(),
    )
        .await
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    let request = NodeRequest {
        instance_type: instance_type.clone(),
    };

    spinner.set_message(format!("Creating node with provider {}...", provider));
    let details = provider_handle.start_node(request)
        .await
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;
    
    let user = provider_handle.get_user()
        .await
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;
    
    // Parse timeout duration and calculate expiration time
    let timeout_expiration = parse_timeout_duration(&timeout)
        .map(|duration| {
            let expiration = Utc::now() + duration;
            expiration.to_rfc3339()
        });
    
    GmlState::add_node(details, provider.clone(), instance_type.clone(), timeout_expiration, user, name.clone())
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    let created_label = name.as_deref().unwrap_or("Node");
    spinner.finish_with_message(format!("{} created successfully!", created_label));
    Ok(())
}

pub async fn handle_delete_node(id: String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = spinner::create_spinner();

    spinner.set_message("Locating node...");
    
    // Find the node in state
    let node = match GmlState::get_node(&id)? {
        Some(n) => n,
        None => return Err(format!("Node with ID '{}' not found", id).into()),
    };

    spinner.set_message("Parsing configuration...");
    let config = config::parse_config()?;
    let provider_config = config.get_provider(&node.provider)
        .ok_or_else(|| format!("Provider '{}' not found in config", node.provider))?;

    let provider_handle = providers::create_provider_handle(
        &node.provider,
        provider_config,
        None,
        config.ssh_public_key.clone(),
    )
        .await
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    let details = NodeDetails {
        id: node.provider_id.clone(),
        ip: node.ip.clone(),
    };

    spinner.set_message(format!("Stopping node with provider {}...", node.provider));
    provider_handle.stop_node(details)
        .await
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    spinner.set_message("Removing from state...");
    GmlState::remove_node(&id)?;

    spinner.finish_with_message("Node deleted successfully!");
    Ok(())
}

pub fn handle_connect_command(id: String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = spinner::create_spinner();

    spinner.set_message("Locating node...");
    
    // Get node data from state with id
    let node = match GmlState::get_node(&id)? {
        Some(n) => n,
        None => return Err(format!("Node with ID '{}' not found", id).into()),
    };

    spinner.set_message(format!("Copying directory to {}@{}...", node.user, node.ip));
    let remote_dir = sync_cwd_to_node(&node)?;

    // Recompute the cheap locals the git-configuration steps below rely on
    let current_dir = env::current_dir()?;
    let is_git_dir = current_dir.join(".git").exists();
    let ssh_cmd = format!("ssh -o StrictHostKeyChecking=no {}@{}", node.user, node.ip);

    // If in a git directory, copy .git directory and configure git ssh
    if is_git_dir {
        spinner.set_message("Copying .git directory...");
        
        // Create .git directory on remote first
        let mkdir_git_cmd = format!("mkdir -p {}/.git", remote_dir);
        sh::run(&format!("{} '{}'", ssh_cmd, mkdir_git_cmd))
            .map_err(|e| format!("Failed to create remote .git directory: {}", e))?;
        
        // Copy .git directory contents with proper rsync semantics
        // Using trailing slashes to copy contents (not the directory itself)
        // Using --delete to ensure clean sync and remove stale files
        let git_rsync_cmd = format!(
            "rsync -avz --quiet --delete {}/.git/ {}@{}:{}/.git/",
            current_dir.display(), node.user, node.ip, remote_dir
        );

        sh::run(&git_rsync_cmd)
            .map_err(|e| format!("Failed to copy .git directory: {}", e))?;

        spinner.set_message("Configuring Git SSH...");

        let app_config = config::parse_config().map_err(|e| e.to_string())?;
        let key_path = ssh::get_ssh_public_key(app_config.ssh_public_key.as_deref())
            .map_err(|e| e.to_string())?;

        // Copy SSH public key to remote machine's authorized_keys
        let copy_key_cmd = format!(
            "cat {} | {} 'mkdir -p ~/.ssh && cat >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys && chmod 700 ~/.ssh'",
            key_path.display(),
            ssh_cmd
        );

        sh::run(&copy_key_cmd)
            .map_err(|e| format!("Failed to copy SSH key: {}", e))?;

        // Configure git to use SSH for this repository (local config, not global)
        let git_config_cmd = format!(
            "{} 'cd {} && git config --local url.\"git@github.com:\".insteadOf \"https://github.com/\"'",
            ssh_cmd, remote_dir
        );

        sh::run(&git_config_cmd)
            .map_err(|e| format!("Failed to configure git SSH: {}", e))?;

        // Copy local git user identity to remote (user.name and user.email)
        if let Some((name, email)) = get_local_git_identity() {
            spinner.set_message("Configuring Git identity...");
            
            let set_name_cmd = format!(
                "{} 'git config --global user.name \"{}\"'",
                ssh_cmd, name.replace("\"", "\\\"")
            );
            sh::run(&set_name_cmd)
                .map_err(|e| format!("Failed to set git user.name: {}", e))?;
            
            let set_email_cmd = format!(
                "{} 'git config --global user.email \"{}\"'",
                ssh_cmd, email.replace("\"", "\\\"")
            );
            sh::run(&set_email_cmd)
                .map_err(|e| format!("Failed to set git user.email: {}", e))?;
        }

        // Configure LOCAL SSH config to enable agent forwarding when connecting to this host
        // This allows Cursor's SSH connection to forward your local SSH agent
        spinner.set_message("Configuring SSH agent forwarding...");
        let home_dir = dirs::home_dir().ok_or("Unable to determine home directory")?;
        configure_local_ssh_agent_forwarding(&home_dir, &node.ip)?;

        // Add GitHub to known_hosts on remote to avoid host verification prompts
        let add_known_hosts_cmd = format!(
            "{} 'ssh-keyscan -t ed25519,rsa github.com >> ~/.ssh/known_hosts 2>/dev/null || true'",
            ssh_cmd
        );
        sh::run(&add_known_hosts_cmd)
            .map_err(|e| format!("Failed to add GitHub to known_hosts: {}", e))?;

        // Reset the git index to ensure working tree matches
        let git_reset_cmd = format!(
            "{} 'cd {} && git reset --mixed HEAD 2>/dev/null || true'",
            ssh_cmd, remote_dir
        );

        sh::run(&git_reset_cmd)
            .map_err(|e| format!("Failed to reset git index: {}", e))?;
    }

    spinner.set_message("Connecting with Cursor...");
    
    // Run cursor --folder-uri vscode-remote://ssh-remote+<user>@<hostname>/<folder_path>
    let folder_uri = format!("vscode-remote://ssh-remote+{}@{}/{}", node.user, node.ip, remote_dir);
    let cursor_cmd = format!("cursor --folder-uri {}", folder_uri);

    spinner.finish_with_message("Opening Cursor...");
    
    sh::spawn(&cursor_cmd)
        .map_err(|e| format!("Failed to launch Cursor: {}. Make sure Cursor is installed and in your PATH.", e))?;

    Ok(())
}

/// Rsyncs the current working directory to the node and returns the remote
/// directory path (`/home/<user>/<dir-name>`). Excludes `.git` and any patterns
/// found in a local `.gitignore`. Shared by `connect` and `run --sync`.
fn sync_cwd_to_node(node: &NodeEntry) -> Result<String, Box<dyn std::error::Error>> {
    let current_dir = env::current_dir()?;
    let dir_name = current_dir.file_name()
        .ok_or("Failed to get directory name")?
        .to_str()
        .ok_or("Directory name contains invalid UTF-8")?;

    let remote_dir = format!("/home/{}/{}", node.user, dir_name);
    let ssh_cmd = format!("ssh -o StrictHostKeyChecking=no {}@{}", node.user, node.ip);

    // Create remote directory first
    let mkdir_cmd = format!("mkdir -p {}", remote_dir);
    sh::run(&format!("{} '{}'", ssh_cmd, mkdir_cmd))
        .map_err(|e| format!("Failed to create remote directory: {}", e))?;

    // Build rsync exclude patterns from .gitignore
    let mut exclude_patterns = vec!["--exclude".to_string(), ".git".to_string()];
    if let Ok(patterns) = read_gitignore_patterns(&current_dir) {
        for pattern in patterns {
            exclude_patterns.push("--exclude".to_string());
            exclude_patterns.push(pattern);
        }
    }

    // Copy FROM local TO remote
    let exclude_args = exclude_patterns.join(" ");
    let rsync_cmd = format!(
        "rsync -avz --quiet {} {}/ {}@{}:{}/",
        exclude_args, current_dir.display(), node.user, node.ip, remote_dir
    );
    sh::run(&rsync_cmd)
        .map_err(|_| -> Box<dyn std::error::Error> { "Failed to copy directory to remote machine".into() })?;

    Ok(remote_dir)
}

/// Escapes a string for safe embedding inside single quotes in a POSIX shell,
/// using the standard `'\''` idiom.
fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Builds the command string sent to the remote shell over SSH.
///
/// The command is passed through verbatim so the remote shell performs the single,
/// authoritative parse of exactly what the caller quoted (like `ssh host "<cmd>"`).
///
/// - When `remote_dir` is set, the command is prefixed with `cd <dir> && `.
/// - When `detach` is set, the command is wrapped in `nohup sh -c '...' &` and its
///   output redirected to `~/gml-run.log` so it survives disconnect.
fn build_remote_command(remote_dir: Option<&str>, detach: bool, command: &str) -> String {
    let inner = match remote_dir {
        Some(dir) => format!("cd {} && {}", dir, command),
        None => command.to_string(),
    };
    if detach {
        format!("nohup sh -c {} > ~/gml-run.log 2>&1 &", shell_single_quote(&inner))
    } else {
        inner
    }
}

/// Runs a command on a node over SSH (non-interactive). Returns the remote
/// command's exit code so the caller can propagate it.
pub fn handle_run_command(
    id: String,
    sync: bool,
    detach: bool,
    command: String,
) -> Result<i32, Box<dyn std::error::Error>> {
    let node = match GmlState::get_node(&id)? {
        Some(n) => n,
        None => return Err(format!("Node with ID '{}' not found", id).into()),
    };

    let remote_dir = if sync {
        Some(sync_cwd_to_node(&node)?)
    } else {
        None
    };

    let remote_cmd = build_remote_command(remote_dir.as_deref(), detach, &command);

    // Invoke ssh directly (not via `sh -c`) so stdio streams live and the remote
    // exit code is available. `BatchMode=yes` makes auth failures fail fast instead
    // of hanging on a password prompt.
    let status = Command::new("ssh")
        .args(["-o", "StrictHostKeyChecking=no", "-o", "BatchMode=yes"])
        .arg(format!("{}@{}", node.user, node.ip))
        .arg(&remote_cmd)
        .status()
        .map_err(|e| format!("Failed to run ssh: {}", e))?;

    if detach {
        println!("Job started on {}; logs at ~/gml-run.log", node.display_id());
    }

    Ok(status.code().unwrap_or(1))
}

pub fn handle_node_timeout_reset(id: String, duration: String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = spinner::create_spinner();

    spinner.set_message("Locating node...");
    
    // Verify the node exists
    let _node = match GmlState::get_node(&id)? {
        Some(n) => n,
        None => return Err(format!("Node with ID '{}' not found", id).into()),
    };

    spinner.set_message("Parsing timeout duration...");
    // Parse timeout duration and calculate expiration time
    let timeout_expiration = parse_timeout_duration(&duration)
        .map(|duration| {
            let expiration = Utc::now() + duration;
            expiration.to_rfc3339()
        })
        .ok_or_else(|| format!("Invalid duration format: '{}'. Use formats like '1h30m', '2h', '30m'", duration))?;

    spinner.set_message("Updating timeout...");
    GmlState::update_node_timeout(&id, Some(timeout_expiration))
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    spinner.finish_with_message("Timeout reset successfully!");
    Ok(())
}

pub fn handle_node_timeout_remove(id: String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = spinner::create_spinner();

    spinner.set_message("Locating node...");
    
    // Verify the node exists
    let _node = match GmlState::get_node(&id)? {
        Some(n) => n,
        None => return Err(format!("Node with ID '{}' not found", id).into()),
    };

    spinner.set_message("Removing timeout...");
    GmlState::update_node_timeout(&id, None)
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    spinner.finish_with_message("Timeout removed successfully!");
    Ok(())
}

pub async fn handle_list_node_types(provider: String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = spinner::create_spinner();

    spinner.set_message("Parsing configuration...");
    let config = config::parse_config()?;
    let provider_config = config.get_provider(&provider)
        .ok_or_else(|| format!("Provider '{}' not found in config", provider))?;

    spinner.set_message(format!("Fetching node types for {}...", provider));
    let provider_handle = providers::create_provider_handle(
        &provider,
        provider_config,
        None,
        config.ssh_public_key.clone(),
    )
        .await
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    let node_types_json = provider_handle.get_node_types()
        .await
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    spinner.finish_with_message("Node types retrieved successfully!");
    
    // Parse JSON and print with color
    let json_value: serde_json::Value = serde_json::from_str(&node_types_json)
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;
    
    let colored_output = colored_json::to_colored_json_auto(&json_value)
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;
    
    println!("{}", colored_output);
    
    Ok(())
}

async fn ensure_daemon_running(_spinner: &ProgressBar) -> Result<(), Box<dyn std::error::Error>> {
    let mut system = System::new_all();
    system.refresh_all();
    
    let daemon_running = system.processes().values().any(|process| {
        // Check for exact name match or if it contains gmld (handles cases with extensions etc)
        process.name().contains("gmld")
    });

    if !daemon_running {
        let current_exe = env::current_exe()?;
        let daemon_path = current_exe.parent()
            .ok_or("Failed to get parent directory")?
            .join("gmld");
            
        if !daemon_path.exists() {
             return Err(format!("Daemon executable not found at {:?}", daemon_path).into());
        }

        // Suppress daemon output to avoid interfering with spinner
        Command::new(daemon_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start daemon: {}", e))?;
            
        // Give it a moment to start
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    
    Ok(())
}

/// Parse a timeout duration string (e.g., "1h", "30m", "2h 30m") into a chrono::Duration
/// Uses the humantime crate to parse human-readable duration strings
fn parse_timeout_duration(timeout_str: &str) -> Option<chrono::Duration> {
    parse_duration(timeout_str)
        .ok()
        .and_then(|std_duration| chrono::Duration::from_std(std_duration).ok())
}

/// Read and parse .gitignore file, returning a vector of patterns
/// Skips comments (lines starting with #) and empty lines
fn read_gitignore_patterns(dir: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let gitignore_path = dir.join(".gitignore");
    
    if !gitignore_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&gitignore_path)?;
    let mut patterns = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Add the pattern as-is (rsync will handle it)
        patterns.push(line.to_string());
    }

    Ok(patterns)
}

/// Get the local git user identity (name and email) from git config
/// Returns None if either user.name or user.email is not configured
fn get_local_git_identity() -> Option<(String, String)> {
    let name = Command::new("git")
        .args(["config", "--get", "user.name"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())?;
    
    let email = Command::new("git")
        .args(["config", "--get", "user.email"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())?;
    
    if name.is_empty() || email.is_empty() {
        return None;
    }
    
    Some((name, email))
}

/// Configure local SSH config to enable agent forwarding for a specific host
/// This adds a Host entry to ~/.ssh/config with ForwardAgent yes
fn configure_local_ssh_agent_forwarding(home_dir: &Path, host_ip: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ssh_config_path = home_dir.join(".ssh/config");
    
    // Read existing config if it exists
    let existing_config = if ssh_config_path.exists() {
        fs::read_to_string(&ssh_config_path)?
    } else {
        String::new()
    };
    
    // Check if this host already has ForwardAgent configured
    let host_pattern = format!("Host {}", host_ip);
    if existing_config.contains(&host_pattern) {
        // Host entry exists, check if it has ForwardAgent
        // For simplicity, we'll skip if the host is already configured
        return Ok(());
    }
    
    // Create the SSH config directory if it doesn't exist
    let ssh_dir = home_dir.join(".ssh");
    if !ssh_dir.exists() {
        fs::create_dir_all(&ssh_dir)?;
    }
    
    // Append the new host configuration
    let new_config = format!(
        "\n# Added by gml connect for SSH agent forwarding\nHost {}\n  ForwardAgent yes\n  AddKeysToAgent yes\n",
        host_ip
    );
    
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ssh_config_path)?;
    
    use std::io::Write;
    file.write_all(new_config.as_bytes())?;
    
    // Set proper permissions on the config file
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&ssh_config_path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_command_is_passed_through_verbatim() {
        let out = build_remote_command(None, false, "python train.py --epochs 10");
        assert_eq!(out, "python train.py --epochs 10");
    }

    #[test]
    fn quoting_is_preserved_so_the_remote_parses_it() {
        // The caller's quoting survives untouched; the remote shell does the parse.
        let out = build_remote_command(None, false, "sh -c 'exit 42'");
        assert_eq!(out, "sh -c 'exit 42'");
    }

    #[test]
    fn sync_prefixes_cd_into_remote_dir() {
        let out = build_remote_command(Some("/home/ubuntu/proj"), false, "ls -la");
        assert_eq!(out, "cd /home/ubuntu/proj && ls -la");
    }

    #[test]
    fn detach_wraps_in_nohup_and_redirects_logs() {
        let out = build_remote_command(None, true, "python train.py");
        assert_eq!(out, "nohup sh -c 'python train.py' > ~/gml-run.log 2>&1 &");
    }

    #[test]
    fn detach_with_sync_wraps_the_cd_prefixed_command() {
        let out = build_remote_command(Some("/home/ubuntu/proj"), true, "make");
        assert_eq!(out, "nohup sh -c 'cd /home/ubuntu/proj && make' > ~/gml-run.log 2>&1 &");
    }

    #[test]
    fn detach_escapes_single_quotes_in_the_command() {
        let out = build_remote_command(None, true, "echo it's");
        assert_eq!(out, "nohup sh -c 'echo it'\\''s' > ~/gml-run.log 2>&1 &");
    }
}

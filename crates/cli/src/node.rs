use chrono::Utc;
use gml_core::{NodeRequest, NodeDetails};
use gml_core::state::GmlState;
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

pub fn handle_create_node(provider: String, instance_type: String, timeout: String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = spinner::create_spinner();

    ensure_daemon_running(&spinner)?;

    // Parse config from ~/.gml/config.toml
    let config = config::parse_config()?;

    // Try to get config for the specified provider
    let provider_config = config.get_provider(&provider)
        .ok_or_else(|| format!("Provider '{}' not found in config", provider))?;

    // Use the config to create a provider handle
    let provider_handle = providers::create_provider_handle(&provider, provider_config)
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    let request = NodeRequest {
        instance_type: instance_type.clone(),
    };

    spinner.set_message(format!("Creating node with provider {}...", provider));
    let details = provider_handle.start_node(request)
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;
    
    let user = provider_handle.get_user()
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;
    
    // Parse timeout duration and calculate expiration time
    let timeout_expiration = parse_timeout_duration(&timeout)
        .map(|duration| {
            let expiration = Utc::now() + duration;
            expiration.to_rfc3339()
        });
    
    GmlState::add_node(details, provider.clone(), instance_type.clone(), timeout_expiration, user)
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    spinner.finish_with_message("Node created successfully!");
    Ok(())
}

pub fn handle_delete_node(id: String) -> Result<(), Box<dyn std::error::Error>> {
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

    let provider_handle = providers::create_provider_handle(&node.provider, provider_config)
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    let details = NodeDetails {
        id: node.provider_id.clone(),
        ip: node.ip.clone(),
    };

    spinner.set_message(format!("Stopping node with provider {}...", node.provider));
    provider_handle.stop_node(details)
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

    spinner.set_message("Getting current working directory...");
    let current_dir = env::current_dir()?;
    let dir_name = current_dir.file_name()
        .ok_or("Failed to get directory name")?
        .to_str()
        .ok_or("Directory name contains invalid UTF-8")?;

    // Check if in a git directory
    let is_git_dir = current_dir.join(".git").exists();

    spinner.set_message(format!("Copying directory to {}@{}...", node.user, node.ip));
    
    // Create remote directory first
    let remote_dir = format!("/home/{}/{}", node.user, dir_name);
    let ssh_cmd = format!("ssh -o StrictHostKeyChecking=no {}@{}", node.user, node.ip);
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

    // If in a git directory, copy .git directory and configure git ssh
    if is_git_dir {
        spinner.set_message("Copying .git directory...");
        
        // Copy .git directory separately
        let git_rsync_cmd = format!(
            "rsync -avz --quiet {}/.git {}@{}:{}/.git",
            current_dir.display(), node.user, node.ip, remote_dir
        );

        sh::run(&git_rsync_cmd)
            .map_err(|e| format!("Failed to copy .git directory: {}", e))?;

        spinner.set_message("Configuring Git SSH...");
        
        // Find SSH public key (try common locations)
        let home_dir = dirs::home_dir()
            .ok_or("Unable to determine home directory")?;
        let ssh_key_paths = vec![
            home_dir.join(".ssh/id_rsa.pub"),
            home_dir.join(".ssh/id_ed25519.pub"),
            home_dir.join(".ssh/id_ecdsa.pub"),
        ];

        let mut ssh_key_path = None;
        for path in &ssh_key_paths {
            if path.exists() {
                ssh_key_path = Some(path.clone());
                break;
            }
        }

        if let Some(key_path) = ssh_key_path {
            // Copy SSH public key to remote machine's authorized_keys
            let copy_key_cmd = format!(
                "cat {} | {} 'mkdir -p ~/.ssh && cat >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys && chmod 700 ~/.ssh'",
                key_path.display(), ssh_cmd
            );

            sh::run(&copy_key_cmd)
                .map_err(|e| format!("Failed to copy SSH key: {}", e))?;

            // Configure git to use SSH
            let git_config_cmd = format!(
                "{} 'cd {} && git config --global url.\"git@github.com:\".insteadOf \"https://github.com/\" || true'",
                ssh_cmd, remote_dir
            );

            sh::run(&git_config_cmd)
                .map_err(|e| format!("Failed to configure git SSH: {}", e))?;
        }
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

pub fn handle_list_node_types(provider: String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = spinner::create_spinner();

    spinner.set_message("Parsing configuration...");
    let config = config::parse_config()?;
    let provider_config = config.get_provider(&provider)
        .ok_or_else(|| format!("Provider '{}' not found in config", provider))?;

    spinner.set_message(format!("Fetching node types for {}...", provider));
    let provider_handle = providers::create_provider_handle(&provider, provider_config)
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    let node_types_json = provider_handle.get_node_types()
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

fn ensure_daemon_running(_spinner: &ProgressBar) -> Result<(), Box<dyn std::error::Error>> {
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
        std::thread::sleep(Duration::from_secs(1));
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


use chrono::Utc;
use gml_core::{NodeRequest, NodeDetails};
use gml_core::state::GmlState;
use std::process::Command;
use std::env;
use std::time::Duration;
use sysinfo::System;
use indicatif::{ProgressBar, ProgressStyle};
use humantime::parse_duration;

use crate::config;
use crate::providers;

pub fn handle_create_node(provider: String, instance_type: String, timeout: String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

    spinner.set_message("Checking daemon status...");
    ensure_daemon_running(&spinner)?;

    spinner.set_message("Parsing configuration...");
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
    
    // Parse timeout duration and calculate expiration time
    let timeout_expiration = parse_timeout_duration(&timeout)
        .map(|duration| {
            let expiration = Utc::now() + duration;
            expiration.to_rfc3339()
        });
    
    spinner.set_message("Updating state...");
    GmlState::add_node(details, provider.clone(), instance_type.clone(), timeout_expiration)
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    spinner.finish_with_message("Node created successfully!");
    Ok(())
}

pub fn handle_delete_node(id: String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

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

pub fn handle_connect_command(_id: String) {
    // TODO: Implement connect logic
    // scp current working dir to remote machine
    // check if in a git directory, if so
    // get user for provider
    // copy ssh public key to remote machine
    // Configure remote machine to use git ssh
    // Run cursor --folder-uri vscode-remote://ssh-remote+<hostname>/<folder_path> to connect
    // Make sure to update spinner

}

pub fn handle_node_timeout_reset(id: String, duration: String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

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
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

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

fn ensure_daemon_running(spinner: &ProgressBar) -> Result<(), Box<dyn std::error::Error>> {
    let mut system = System::new_all();
    system.refresh_all();
    
    let daemon_running = system.processes().values().any(|process| {
        // Check for exact name match or if it contains gmld (handles cases with extensions etc)
        process.name().contains("gmld")
    });

    if !daemon_running {
        spinner.set_message("Daemon not running, starting gmld...");
        
        let current_exe = env::current_exe()?;
        let daemon_path = current_exe.parent()
            .ok_or("Failed to get parent directory")?
            .join("gmld");
            
        if !daemon_path.exists() {
             return Err(format!("Daemon executable not found at {:?}", daemon_path).into());
        }

        Command::new(daemon_path)
            .spawn()
            .map_err(|e| format!("Failed to start daemon: {}", e))?;
            
        // Give it a moment to start
        std::thread::sleep(Duration::from_secs(1));
        spinner.set_message("Daemon started.");
    } else {
        spinner.set_message("Daemon is already running.");
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


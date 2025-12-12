use gml_core::error::GmlError;
use gml_core::state::{GmlState, NodeEntry, ClusterEntry};
use chrono::{DateTime, Utc};
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::fs::{OpenOptions, create_dir_all, File};
use std::io::Write;
use dirs;

fn open_log_file() -> Result<File, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir()
        .ok_or("Unable to determine home directory")?;
    let log_dir = home_dir.join(".gml");
    let log_file = log_dir.join("gmld.log");
    
    // Create .gml directory if it doesn't exist
    create_dir_all(&log_dir)?;
    
    // Open log file for appending (create if it doesn't exist)
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;
    
    Ok(file)
}

fn log<W: Write>(out: &mut W, message: &str) {
    let _ = writeln!(out, "{}", message);
    let _ = out.flush();
}

fn log_error<W: Write>(out: &mut W, message: &str) {
    log(out, &format!("ERROR: {}", message));
}

fn main() {
    // Initialize logging to ~/.gml/gmld.log
    let mut log_file = match open_log_file() {
        Ok(f) => f,
        Err(e) => {
        eprintln!("Failed to initialize log file: {}", e);
        return;
        }
    };
    
    log(&mut log_file, "GML Daemon starting...");
    
    loop {
        match GmlState::load() {
            Ok(state) => {
                log(&mut log_file, &format!("Read state: {} nodes, {} clusters", 
                    state.nodes.len(), 
                    state.clusters.len()));
                
                // Process node timeouts
                for node_entry in &state.nodes {
                    if let Some(ref timeout) = node_entry.timeout {
                        if let Err(e) = handle_node_timeout(&mut log_file, node_entry, timeout) {
                            log_error(&mut log_file, &format!("Error handling node timeout {}: {}", node_entry.id, e));
                        }
                    }
                }
                
                // Process cluster timeouts
                for cluster_entry in &state.clusters {
                    if let Some(ref timeout) = cluster_entry.timeout {
                        if let Err(e) = handle_cluster_timeout(&mut log_file, cluster_entry, timeout) {
                            log_error(&mut log_file, &format!("Error handling cluster timeout {}: {}", cluster_entry.id, e));
                        }
                    }
                }
            }
            Err(e) => {
                log_error(&mut log_file, &format!("Error reading state file: {}", e));
            }
        }
        
        // Sleep for 1 minute
        thread::sleep(Duration::from_secs(60));
    }
}

/// Handle node timeout - check if expired and stop/remove if needed
fn handle_node_timeout<W: Write>(log_out: &mut W, node_entry: &NodeEntry, timeout: &str) -> Result<(), GmlError> {
    // Parse the timeout timestamp
    let timeout_dt = DateTime::parse_from_rfc3339(timeout)
        .map_err(|e| GmlError::from(format!("Failed to parse timeout for node {}: {}", node_entry.id, e)))?;
    let timeout_utc = timeout_dt.with_timezone(&Utc);
    let now = Utc::now();
    
    // Check if timeout has expired
    if now < timeout_utc {
        // Not expired yet
        return Ok(());
    }
    
    log(log_out, &format!("Node {} has expired (timeout: {}), deleting...", node_entry.id, timeout));
    
    // Call gml node delete command
    let output = Command::new("gml")
        .args(&["node", "delete", &node_entry.id])
        .output()
        .map_err(|e| GmlError::from(format!("Failed to execute gml node delete: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GmlError::from(format!("gml node delete failed: {}", stderr)));
    }
    
    log(log_out, &format!("Successfully deleted node {}", node_entry.id));
    
    Ok(())
}

/// Handle cluster timeout - check if expired and stop/remove if needed
fn handle_cluster_timeout<W: Write>(log_out: &mut W, cluster_entry: &ClusterEntry, timeout: &str) -> Result<(), GmlError> {
    // Parse the timeout timestamp
    let timeout_dt = DateTime::parse_from_rfc3339(timeout)
        .map_err(|e| GmlError::from(format!("Failed to parse timeout for cluster {}: {}", cluster_entry.id, e)))?;
    let timeout_utc = timeout_dt.with_timezone(&Utc);
    let now = Utc::now();
    
    // Check if timeout has expired
    if now < timeout_utc {
        // Not expired yet
        return Ok(());
    }
    
    log(log_out, &format!("Cluster {} has expired (timeout: {}), deleting...", cluster_entry.id, timeout));
    
    // Call gml cluster delete command
    let output = Command::new("gml")
        .args(&["cluster", "delete", &cluster_entry.id])
        .output()
        .map_err(|e| GmlError::from(format!("Failed to execute gml cluster delete: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GmlError::from(format!("gml cluster delete failed: {}", stderr)));
    }
    
    log(log_out, &format!("Successfully deleted cluster {}", cluster_entry.id));
    
    Ok(())
}


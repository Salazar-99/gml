use gml_core::error::GmlError;
use gml_core::state::{GmlState, NodeEntry, ClusterEntry};
use chrono::{DateTime, Utc};
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    println!("GML Daemon starting...");
    
    loop {
        match GmlState::load() {
            Ok(state) => {
                println!("Read state: {} nodes, {} clusters", 
                    state.nodes.len(), 
                    state.clusters.len());
                
                // Process node timeouts
                for node_entry in &state.nodes {
                    if let Some(ref timeout) = node_entry.timeout {
                        if let Err(e) = handle_node_timeout(node_entry, timeout) {
                            eprintln!("Error handling node timeout {}: {}", node_entry.id, e);
                        }
                    }
                }
                
                // Process cluster timeouts
                for cluster_entry in &state.clusters {
                    if let Some(ref timeout) = cluster_entry.timeout {
                        if let Err(e) = handle_cluster_timeout(cluster_entry, timeout) {
                            eprintln!("Error handling cluster timeout {}: {}", cluster_entry.id, e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading state file: {}", e);
            }
        }
        
        // Sleep for 1 minute
        thread::sleep(Duration::from_secs(60));
    }
}

/// Handle node timeout - check if expired and stop/remove if needed
fn handle_node_timeout(node_entry: &NodeEntry, timeout: &str) -> Result<(), GmlError> {
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
    
    println!("Node {} has expired (timeout: {}), deleting...", node_entry.id, timeout);
    
    // Call gml node delete command
    let output = Command::new("gml")
        .args(&["node", "delete", &node_entry.id])
        .output()
        .map_err(|e| GmlError::from(format!("Failed to execute gml node delete: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GmlError::from(format!("gml node delete failed: {}", stderr)));
    }
    
    println!("Successfully deleted node {}", node_entry.id);
    
    Ok(())
}

/// Handle cluster timeout - check if expired and stop/remove if needed
fn handle_cluster_timeout(cluster_entry: &ClusterEntry, timeout: &str) -> Result<(), GmlError> {
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
    
    println!("Cluster {} has expired (timeout: {}), deleting...", cluster_entry.id, timeout);
    
    // Call gml cluster delete command
    let output = Command::new("gml")
        .args(&["cluster", "delete", &cluster_entry.id])
        .output()
        .map_err(|e| GmlError::from(format!("Failed to execute gml cluster delete: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GmlError::from(format!("gml cluster delete failed: {}", stderr)));
    }
    
    println!("Successfully deleted cluster {}", cluster_entry.id);
    
    Ok(())
}


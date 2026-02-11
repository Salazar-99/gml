use chrono::{DateTime, Utc};
use comfy_table::{Cell, Table};
use gml_core::state::GmlState;

pub fn handle_ls_command() {
    // Display nodes
    match GmlState::list_nodes() {
        Ok(nodes) => {
            if nodes.is_empty() {
                println!("No nodes found.");
            } else {
                let mut table = Table::new();
                table.set_header(vec!["ID", "IP", "Provider", "Instance Type", "Time Remaining", "Created At"]);
                
                for node in &nodes {
                    // Format the created_at timestamp to be more readable
                    let created_at = match DateTime::parse_from_rfc3339(&node.created_at) {
                        Ok(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                        Err(_) => node.created_at.clone(),
                    };
                    
                    // Calculate and format time remaining
                    let time_remaining = format_time_remaining(&node.timeout);
                    
                    table.add_row(vec![
                        Cell::new(&node.id),
                        Cell::new(&node.ip),
                        Cell::new(&node.provider),
                        Cell::new(&node.instance_type),
                        Cell::new(time_remaining),
                        Cell::new(created_at),
                    ]);
                }
                
                println!("Nodes");
                println!("{}", table);
            }
        }
        Err(e) => {
            eprintln!("Error listing nodes: {}", e);
            std::process::exit(1);
        }
    }
    
    // Display clusters
    match GmlState::list_clusters() {
        Ok(clusters) => {
            if clusters.is_empty() {
                println!("\nNo clusters found.");
            } else {
                let mut table = Table::new();
                table.set_header(vec!["ID", "Provider", "Node Count", "Timeout", "Created At"]);
                
                for cluster in &clusters {
                    // Format the created_at timestamp to be more readable
                    let created_at = match DateTime::parse_from_rfc3339(&cluster.created_at) {
                        Ok(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                        Err(_) => cluster.created_at.clone(),
                    };
                    
                    // Format timeout - show "None" if not set
                    let timeout_display = cluster.timeout.as_deref().unwrap_or("None");
                    
                    table.add_row(vec![
                        Cell::new(&cluster.id),
                        Cell::new(&cluster.provider),
                        Cell::new(cluster.node_count),
                        Cell::new(timeout_display),
                        Cell::new(created_at),
                    ]);
                }
                
                println!("\nClusters");
                println!("{}", table);
            }
        }
        Err(e) => {
            eprintln!("Error listing clusters: {}", e);
            std::process::exit(1);
        }
    }
}

/// Calculate and format the remaining time until expiration
/// Returns a formatted string like "2h 30m", "Expired", "None", or "Invalid"
fn format_time_remaining(timeout: &Option<String>) -> String {
    match timeout {
        Some(timeout_str) => {
            match DateTime::parse_from_rfc3339(timeout_str) {
                Ok(timeout_dt) => {
                    let timeout_utc = timeout_dt.with_timezone(&Utc);
                    let now = Utc::now();
                    if now >= timeout_utc {
                        "Expired".to_string()
                    } else {
                        let remaining = timeout_utc - now;
                        let total_seconds = remaining.num_seconds();
                        let hours = total_seconds / 3600;
                        let minutes = (total_seconds % 3600) / 60;
                        format!("{}h {}m", hours, minutes)
                    }
                }
                Err(_) => "Invalid".to_string(),
            }
        }
        None => "None".to_string(),
    }
}


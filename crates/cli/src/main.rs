use clap::{Parser, Subcommand};
use comfy_table::{Cell, Table};
use chrono::DateTime;
use gml_core::NodeRequest;
use crate::state::GmlState;

mod config;
mod providers;
mod state;


#[derive(Parser, Debug)]
#[command(name = "gml")]
#[command(about = "GML - A CLI tool for managing GPU nodes and clusters")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    // Manage individual nodes
    Node {
        #[command(subcommand)]
        action: NodeAction,
    },
    // Manage clusters
    Cluster {
        #[command(subcommand)]
        action: ClusterAction,
    },
    /// List all nodes and clusters
    Ls,
}

#[derive(Subcommand, Debug)]
enum NodeAction {
    /// Create a new node
    Create {
        #[arg(short, long)]
        provider: String,
        #[arg(short, long)]
        instance_type: String,
        #[arg(short, long)]
        timeout: String,
    },
    /// Delete a node
    Delete {
        #[arg(short, long)]
        provider: String,
        #[arg(short, long)]
        node_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum ClusterAction {
    /// Create a new cluster
    Create {
        #[arg(short, long)]
        provider: String,
        #[arg(short, long)]
        nodes: Option<i32>,
        #[arg(short, long)]
        timeout: Option<String>,
    },
    /// Delete a cluster
    Delete {
        #[arg(short, long)]
        provider: String,
        #[arg(short, long)]
        cluster_id: Option<String>,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Node { action } => {
            match action {
                NodeAction::Create { provider, instance_type, timeout } => {
                    if let Err(e) = handle_create_node(provider, instance_type, timeout) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                NodeAction::Delete { provider, node_id } => {
                    println!("Deleting node with provider: {} and id: {}", provider, node_id);
                    // TODO: Implement node deletion logic
                }
            }
        }
        Commands::Cluster { action } => {
            match action {
                ClusterAction::Create { provider, nodes, timeout } => {
                    println!("Creating cluster with provider: {} and {:?} nodes", provider, nodes);
                    // TODO: Implement node deletion logic
                }
                ClusterAction::Delete { provider, cluster_id } => {
                    println!("Deleting cluster with provider: {}", provider);
                    // TODO: Implement cluster deletion logic
                }
            }
        }
        Commands::Ls => {
            handle_ls_command();
        }
    }
}

fn handle_create_node(provider: String, instance_type: String, _timeout: String) -> Result<(), Box<dyn std::error::Error>> {
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

    let details = provider_handle.start_node(request)
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;
    
    GmlState::add_node(details, provider.clone(), instance_type.clone())
        .map_err(|e| Box::from(e) as Box<dyn std::error::Error>)?;

    // TODO: Add timeout logic

    Ok(())
}

fn handle_ls_command() {
    // Display nodes
    match state::GmlState::list_nodes() {
        Ok(nodes) => {
            if nodes.is_empty() {
                println!("No nodes found.");
            } else {
                let mut table = Table::new();
                table.set_header(vec!["ID", "IP", "Provider", "Instance Type", "Created At"]);
                
                for node in &nodes {
                    // Format the created_at timestamp to be more readable
                    let created_at = match DateTime::parse_from_rfc3339(&node.created_at) {
                        Ok(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                        Err(_) => node.created_at.clone(),
                    };
                    
                    table.add_row(vec![
                        Cell::new(&node.id),
                        Cell::new(&node.ip),
                        Cell::new(&node.provider),
                        Cell::new(&node.instance_type),
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
    match state::GmlState::list_clusters() {
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

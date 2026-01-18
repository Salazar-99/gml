use clap::{Parser, Subcommand};

mod config;
mod providers;
mod node;
mod cluster;
mod ls;
mod spinner;
mod sh;


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
    /// Connect to a node
    Connect {
        /// The ID of the node
        id: String,
    },
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
        /// The unique ID of the node to delete
        id: String,
    },
    /// Manage node timeouts
    Timeout {
        #[command(subcommand)]
        action: TimeoutAction,
    },
    /// List available node types for a provider
    ListTypes {
        #[arg(short, long)]
        provider: String,
    },
}

#[derive(Subcommand, Debug)]
enum TimeoutAction {
    /// Reset the timeout for a node
    Reset {
        /// The unique ID of the node
        #[arg(short, long)]
        id: String,
        /// The duration for the timeout (e.g., "1h30m", "2h", "30m")
        #[arg(short, long)]
        duration: String,
    },
    /// Remove the timeout for a node
    Remove {
        /// The unique ID of the node
        #[arg(short, long)]
        id: String,
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
                    if let Err(e) = node::handle_create_node(provider, instance_type, timeout) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                NodeAction::Delete { id } => {
                    if let Err(e) = node::handle_delete_node(id) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                NodeAction::Timeout { action } => {
                    match action {
                        TimeoutAction::Reset { id, duration } => {
                            if let Err(e) = node::handle_node_timeout_reset(id, duration) {
                                eprintln!("Error: {}", e);
                                std::process::exit(1);
                            }
                        }
                        TimeoutAction::Remove { id } => {
                            if let Err(e) = node::handle_node_timeout_remove(id) {
                                eprintln!("Error: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
                NodeAction::ListTypes { provider } => {
                    if let Err(e) = node::handle_list_node_types(provider) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Cluster { action } => {
            match action {
                ClusterAction::Create { provider, nodes, timeout } => {
                    if let Err(e) = cluster::handle_create_cluster(provider, nodes, timeout) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                ClusterAction::Delete { provider, cluster_id } => {
                    if let Err(e) = cluster::handle_delete_cluster(provider, cluster_id) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Ls => {
            ls::handle_ls_command();
        }
        Commands::Connect { id } => {
            if let Err(e) = node::handle_connect_command(id) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}


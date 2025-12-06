use clap::{Parser, Subcommand};

mod config;
// mod provider; // TODO: Fix module path when provider is ready
// mod error; // moved to gml-core


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
        #[arg(short, long)]
        provider: String,
        // Add node-specific flags here
        #[arg(short, long)]
        instance_type: Option<String>,
        #[arg(short, long)]
        ssh_key_id: Option<String>,
        #[arg(short, long)]
        timeout: Option<String>
    },
    // Manage clusters
    Cluster {
        #[arg(short, long)]
        provider: String,
        // Add cluster-specific flags here
        #[arg(short, long)]
        nodes: Option<i32>,
        #[arg(short, long)]
        timeout: Option<String>,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Node { provider, instance_type, ssh_key_id, timeout  } => {
            println!("Running node command for provider: {}", provider);
            if let Some(it) = instance_type {
                println!("Instance type: {}", it);
            }
            if let Some(key) = ssh_key_id {
                println!("SSH key ID: {}", key);
            }
            if let Some(timeout) = timeout {
                println!("Timeout: {}", timeout);
            }
            match config::parse_config() {
                Ok(config) => {
                    let providers: Vec<&String> = config.provider_names();
                    println!("Successfully parsed {} provider(s): {:?}", providers.len(), providers);
                }
                Err(e) => {
                    eprintln!("Error parsing config: {}", e);
                    std::process::exit(1);
                }
            }
            // Handle node-specific logic here
        }
        Commands::Cluster { provider, nodes, timeout } => {
            println!("Running cluster command for provider: {}", provider);
            if let Some(n) = nodes {
                println!("Number of nodes: {}", n);
            }
            if let Some(t) = timeout {
                println!("Timeout: {}", t);
            }
            match config::parse_config() {
                Ok(config) => {
                    let providers: Vec<&String> = config.provider_names();
                    println!("Successfully parsed {} provider(s): {:?}", providers.len(), providers);
                }
                Err(e) => {
                    eprintln!("Error parsing config: {}", e);
                    std::process::exit(1);
                }
            }
            // Handle cluster-specific logic here
        }
    }
}

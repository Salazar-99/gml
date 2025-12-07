use gml_core::NodeDetails;
use gml_core::error::GmlError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const STATE_PATH: &str = "~/.gml/state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmlState {
    pub nodes: Vec<NodeEntry>,
    pub clusters: Vec<ClusterEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEntry {
    pub id: String,
    pub ip: String,
    pub provider: String,
    pub created_at: String,
    pub instance_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterEntry {
    pub id: String,
    pub provider: String,
    pub created_at: String,
    pub node_count: usize,
    pub timeout: Option<String>,
}

impl Default for GmlState {
    fn default() -> Self {
        GmlState {
            nodes: Vec::new(),
            clusters: Vec::new(),
        }
    }
}

impl GmlState {
    /// Load state from the JSON file, creating a new state if the file doesn't exist
    pub fn load() -> Result<Self, GmlError> {
        let state_path = expand_path(STATE_PATH)?;
        
        // Create directory if it doesn't exist
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                GmlError::from(format!("Failed to create state directory: {}", e))
            })?;
        }

        // Read and parse the file, or return default if it doesn't exist
        if !state_path.exists() {
            return Ok(GmlState::default());
        }

        let contents = fs::read_to_string(&state_path).map_err(|e| {
            GmlError::from(format!("Failed to read state file: {}", e))
        })?;

        serde_json::from_str(&contents).map_err(|e| {
            GmlError::from(format!("Failed to parse state file: {}", e))
        })
    }

    /// Save state to the JSON file
    pub fn save(&self) -> Result<(), GmlError> {
        let state_path = expand_path(STATE_PATH)?;
        
        // Create directory if it doesn't exist
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                GmlError::from(format!("Failed to create state directory: {}", e))
            })?;
        }

        let json = serde_json::to_string_pretty(self).map_err(|e| {
            GmlError::from(format!("Failed to serialize state: {}", e))
        })?;

        fs::write(&state_path, json).map_err(|e| {
            GmlError::from(format!("Failed to write state file: {}", e))
        })
    }

    /// Add a node entry to the state
    pub fn add_node(
        node_details: NodeDetails,
        provider: String,
        instance_type: String,
    ) -> Result<(), GmlError> {
        let mut state = Self::load()?;
        
        let entry = NodeEntry {
            id: node_details.id.clone(),
            ip: node_details.ip,
            provider,
            created_at: chrono::Utc::now().to_rfc3339(),
            instance_type,
        };

        // Check if node already exists
        if state.nodes.iter().any(|n| n.id == entry.id) {
            return Err(GmlError::from(format!("Node with id '{}' already exists", entry.id)));
        }

        state.nodes.push(entry);
        state.save()
    }

    /// Remove a node entry from the state
    pub fn remove_node(node_id: &str) -> Result<(), GmlError> {
        let mut state = Self::load()?;
        let initial_len = state.nodes.len();
        state.nodes.retain(|n| n.id != node_id);
        
        if state.nodes.len() == initial_len {
            return Err(GmlError::from(format!("Node with id '{}' not found", node_id)));
        }

        state.save()
    }

    /// Get a node entry by ID
    pub fn get_node(node_id: &str) -> Result<Option<NodeEntry>, GmlError> {
        let state = Self::load()?;
        Ok(state.nodes.into_iter().find(|n| n.id == node_id))
    }

    /// List all nodes
    pub fn list_nodes() -> Result<Vec<NodeEntry>, GmlError> {
        let state = Self::load()?;
        Ok(state.nodes)
    }

    /// Add a cluster entry to the state
    pub fn add_cluster(
        cluster_id: String,
        provider: String,
        node_count: usize,
        timeout: Option<String>,
    ) -> Result<(), GmlError> {
        let mut state = Self::load()?;
        
        let entry = ClusterEntry {
            id: cluster_id.clone(),
            provider,
            node_count,
            timeout,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // Check if cluster already exists
        if state.clusters.iter().any(|c| c.id == entry.id) {
            return Err(GmlError::from(format!("Cluster with id '{}' already exists", entry.id)));
        }

        state.clusters.push(entry);
        state.save()
    }

    /// Remove a cluster entry from the state
    pub fn remove_cluster(cluster_id: &str) -> Result<(), GmlError> {
        let mut state = Self::load()?;
        let initial_len = state.clusters.len();
        state.clusters.retain(|c| c.id != cluster_id);
        
        if state.clusters.len() == initial_len {
            return Err(GmlError::from(format!("Cluster with id '{}' not found", cluster_id)));
        }

        state.save()
    }

    /// Get a cluster entry by ID
    pub fn get_cluster(cluster_id: &str) -> Result<Option<ClusterEntry>, GmlError> {
        let state = Self::load()?;
        Ok(state.clusters.into_iter().find(|c| c.id == cluster_id))
    }

    /// List all clusters
    pub fn list_clusters() -> Result<Vec<ClusterEntry>, GmlError> {
        let state = Self::load()?;
        Ok(state.clusters)
    }
}

/// Expand a path that may contain `~` to the user's home directory
fn expand_path(path: &str) -> Result<PathBuf, GmlError> {
    if path.starts_with("~/") {
        let home = dirs::home_dir().ok_or_else(|| {
            GmlError::from("Unable to determine home directory")
        })?;
        Ok(home.join(&path[2..]))
    } else {
        Ok(PathBuf::from(path))
    }
}

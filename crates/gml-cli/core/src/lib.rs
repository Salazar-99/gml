pub mod error;
pub mod ssh;
pub mod state;

use async_trait::async_trait;
use error::GmlError;

#[async_trait]
pub trait NodeProvider: Send + Sync {
    async fn start_node(&self, request: NodeRequest) -> Result<NodeDetails, GmlError>;
    async fn stop_node(&self, details: NodeDetails) -> Result<NodeDetails, GmlError>;
    async fn get_user(&self) -> Result<String, GmlError>;
    async fn get_node_types(&self) -> Result<String, GmlError>;
}

pub struct NodeDetails {
    pub ip: String,
    pub id: String
}

pub struct NodeRequest {
    pub instance_type: String
}

pub trait ClusterProvider {}


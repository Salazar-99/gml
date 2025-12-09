pub mod error;
pub mod state;

use error::GmlError;

pub trait NodeProvider {
    fn start_node(&self, request: NodeRequest) -> Result<NodeDetails, GmlError>;
    fn stop_node(&self, details: NodeDetails) -> Result<NodeDetails, GmlError>;
    fn get_user(&self) -> Result<String, GmlError>;
}

pub struct NodeDetails {
    pub ip: String,
    pub id: String
}

pub struct NodeRequest {
    pub instance_type: String
}

pub trait ClusterProvider {}


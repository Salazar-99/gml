use gml_core::{NodeProvider, NodeRequest, NodeDetails};
use gml_core::error::GmlError;

pub struct Lambda {
    pub api_key: String
}

impl NodeProvider for Lambda {
    fn start_node(&self, _request: NodeRequest) -> Result<NodeDetails, GmlError> {
        // Implementation placeholder
        Ok(NodeDetails {
            ip: "127.0.0.1".to_string(),
            id: "lambda-node-1".to_string(),
        })
    }

    fn stop_node(&self, _details: NodeDetails) -> Result<NodeDetails, GmlError> {
        // Implementation placeholder
        Ok(NodeDetails {
            ip: "127.0.0.1".to_string(),
            id: "lambda-node-1".to_string(),
        })
    }
}


use gml_core::{NodeProvider, NodeRequest, NodeDetails};
use gml_core::error::GmlError;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://cloud.lambda.ai/api/v1/";
pub struct Lambda {
    pub api_key: String,
    pub ssh_key_id: String,
    pub region: String,
}

#[derive(Serialize)]
struct LaunchRequest {
    region_name: String,
    instance_type_name: String,
    ssh_key: String,
}

#[derive(Deserialize)]
struct LaunchResponse {
    data: LaunchResponseData,
}

#[derive(Deserialize)]
struct LaunchResponseData {
    instance_ids: Vec<String>,
}

#[derive(Deserialize)]
struct InfoResponse {
    data: InfoResponseData,
}

#[derive(Deserialize)]
struct InfoResponseData {
    ip: String,
}

#[derive(Serialize)]
struct TerminateRequest {
    instance_id: String,
}

#[derive(Deserialize)]
struct TerminateResponse {
    data: TerminateResponseData,
}

#[derive(Deserialize)]
struct TerminateResponseData {
    terminated_instances: Vec<TerminatedInstance>,
}

#[derive(Deserialize)]
struct TerminatedInstance {
    id: String,
}

impl NodeProvider for Lambda {
    fn start_node(&self, request: NodeRequest) -> Result<NodeDetails, GmlError> {
        let client = reqwest::blocking::Client::new();
        
        let payload = LaunchRequest {
            region_name: self.region.clone(),
            instance_type_name: request.instance_type,
            ssh_key: self.ssh_key_id.clone(),
        };

        let url = BASE_URL.to_owned() + "instance-operations/launch";

        let response = client.post(url)
            .basic_auth(&self.api_key, None::<&str>)
            .header("accept", "application/json")
            .json(&payload)
            .send()
            .map_err(|e| GmlError::from(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            return Err(GmlError::from(format!("API Error ({}): {}", status, text)));
        }

        let launch_response: LaunchResponse = response.json()
            .map_err(|e| GmlError::from(format!("Failed to parse response: {}", e)))?;

        let instance_id = launch_response.data.instance_ids.first()
            .ok_or_else(|| GmlError::from("No instance ID returned"))?
            .clone();

        let ip = self.get_node_ip(&instance_id)?;

        Ok(NodeDetails {
            ip: ip,
            id: instance_id,
        })
    }

    fn stop_node(&self, details: NodeDetails) -> Result<NodeDetails, GmlError> {
        let client = reqwest::blocking::Client::new();

        let payload = TerminateRequest {
            instance_id: details.id.clone(),
        };

        let url = BASE_URL.to_owned() + "instance-operations/terminate";

        let response = client.post(url)
            .basic_auth(&self.api_key, None::<&str>)
            .header("accept", "application/json")
            .json(&payload)
            .send()
            .map_err(|e| GmlError::from(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            return Err(GmlError::from(format!("API Error ({}): {}", status, text)));
        }

        let terminate_response: TerminateResponse = response.json()
            .map_err(|e| GmlError::from(format!("Failed to parse response: {}", e)))?;

        let instance = terminate_response.data.terminated_instances.first()
            .ok_or_else(|| GmlError::from("No terminated instance returned"))?;

        Ok(NodeDetails {
            ip: details.ip,
            id: instance.id.clone(),
        })
    }
}

impl Lambda {
    fn get_node_ip(&self, instance_id: &str) -> Result<String, GmlError> {
        let client = reqwest::blocking::Client::new();

        let url = format!("{}instances/{}", BASE_URL, instance_id);

        let response = client.get(&url)
            .basic_auth(&self.api_key, None::<&str>)
            .header("accept", "application/json")
            .send()
            .map_err(|e| GmlError::from(format!("Request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().unwrap_or_default();
            return Err(GmlError::from(format!("API Error ({}): {}", status, text)));
        }

        let info = response.json::<InfoResponse>()
            .map_err(|e| GmlError::from(format!("Failed to parse response: {}", e)))?;

        Ok(info.data.ip)
    }

    pub fn new(api_key: String, ssh_key_id: String, region: String) -> Lambda {
        Lambda {
            api_key,
            ssh_key_id,
            region
        }
    }
}

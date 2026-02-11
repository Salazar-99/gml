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
    ssh_key_names: Vec<String>,
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
    #[serde(default)]
    ip: Option<String>,
    status: String,
}

#[derive(Serialize)]
struct TerminateRequest {
    instance_ids: Vec<String>,
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
        
        // Create launch request with region_name from CLI flag or config
        let payload = LaunchRequest {
            region_name: self.region.clone(),
            instance_type_name: request.instance_type.clone(),
            ssh_key_names: vec![self.ssh_key_id.clone()],
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

        let response_text = response.text()
            .map_err(|e| GmlError::from(format!("Failed to read response body: {}", e)))?;
        
        let launch_response: LaunchResponse = serde_json::from_str(&response_text)
            .map_err(|e| GmlError::from(format!("Failed to parse response: {} - Response body: {}", e, response_text)))?;

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
            instance_ids: vec![details.id.clone()],
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

        let response_text = response.text()
            .map_err(|e| GmlError::from(format!("Failed to read response body: {}", e)))?;
        
        let terminate_response: TerminateResponse = serde_json::from_str(&response_text)
            .map_err(|e| GmlError::from(format!("Failed to parse response: {} - Response body: {}", e, response_text)))?;

        let instance = terminate_response.data.terminated_instances.first()
            .ok_or_else(|| GmlError::from("No terminated instance returned"))?;

        Ok(NodeDetails {
            ip: details.ip,
            id: instance.id.clone(),
        })
    }

    /// Hardcoded Ubuntu user, works for default Lambda Stack image
    fn get_user(&self) -> Result<String, GmlError> {
        Ok("ubuntu".to_string())
    }

    fn get_node_types(&self) -> Result<String, GmlError> {
        let client = reqwest::blocking::Client::new();
        
        let url = BASE_URL.to_owned() + "instance-types";
        
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
        
        let response_text = response.text()
            .map_err(|e| GmlError::from(format!("Failed to read response body: {}", e)))?;
        
        // Parse JSON and filter out entries with empty regions_with_capacity_available
        let mut json_value: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| GmlError::from(format!("Failed to parse response: {} - Response body: {}", e, response_text)))?;
        
        // Filter out instance types with empty regions_with_capacity_available
        // Structure: { "data": { "instance_type_name": { "regions_with_capacity_available": [...] }, ... } }
        if let Some(serde_json::Value::Object(data_map)) = json_value.get_mut("data") {
            data_map.retain(|_, instance_data| {
                instance_data
                    .get("regions_with_capacity_available")
                    .and_then(|regions| regions.as_array())
                    .map_or(false, |regions_array| !regions_array.is_empty())
            });
        }
        
        let pretty_json = serde_json::to_string_pretty(&json_value)
            .map_err(|e| GmlError::from(format!("Failed to pretty print JSON: {}", e)))?;
        
        Ok(pretty_json)
    }
}

impl Lambda {
    fn get_node_ip(&self, instance_id: &str) -> Result<String, GmlError> {
        const MAX_RETRIES: u32 = 60; // 10 minutes / 10 seconds = 60 attempts
        const RETRY_DELAY_SECS: u64 = 10;
        
        for attempt in 1..=MAX_RETRIES {
            let client = reqwest::blocking::Client::new();

            let url = format!("{}instances/{}", BASE_URL, instance_id);

            let response = client.get(&url)
                .basic_auth(&self.api_key, None::<&str>)
                .header("accept", "application/json")
                .send()
                .map_err(|e| {
                    GmlError::from(format!("Request failed: {}", e))
                })?;
                
            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().unwrap_or_default();
                return Err(GmlError::from(format!("API Error ({}): {}", status, text)));
            }

            let response_text = response.text()
                .map_err(|e| {
                    GmlError::from(format!("Failed to read response body: {}", e))
                })?;
            
            let info: InfoResponse = serde_json::from_str(&response_text)
                .map_err(|e| {
                    GmlError::from(format!("Failed to parse response: {} - Response body: {}", e, response_text))
                })?;

            // Check if both IP is available and status is "active"
            if let Some(ip) = &info.data.ip {
                if info.data.status == "active" {
                    return Ok(ip.clone());
                }
            }
            
            if attempt < MAX_RETRIES {
                std::thread::sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS));
            }
        }

        Err(GmlError::from(format!(
            "Instance {} did not become active with an IP address after {} minutes. Please try again later.",
            instance_id, (MAX_RETRIES as u64 * RETRY_DELAY_SECS) / 60
        )))
    }

    pub fn new(api_key: String, ssh_key_id: String, region: String) -> Lambda {
        Lambda {
            api_key,
            ssh_key_id,
            region
        }
    }
}

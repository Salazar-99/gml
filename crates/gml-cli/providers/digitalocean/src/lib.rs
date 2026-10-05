use async_trait::async_trait;
use gml_core::{NodeProvider, NodeRequest, NodeDetails};
use gml_core::error::GmlError;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.digitalocean.com/v2/";

/// Default droplet image; override with `GML_DIGITALOCEAN_IMAGE` if your chosen
/// size/region needs a different (e.g. GPU-optimized) image slug.
const DEFAULT_IMAGE: &str = "ubuntu-22-04-x64";

/// DigitalOcean boots its stock Linux images with `root` as the only user,
/// provisioned via the `ssh_keys` passed at create time.
const DEFAULT_SSH_USER: &str = "root";

pub struct DigitalOcean {
    pub api_key: String,
    pub ssh_key_id: String,
    pub region: String,
}

#[derive(Serialize)]
struct CreateDropletRequest {
    name: String,
    region: String,
    size: String,
    image: String,
    ssh_keys: Vec<String>,
}

#[derive(Deserialize)]
struct CreateDropletResponse {
    droplet: Droplet,
}

#[derive(Deserialize)]
struct GetDropletResponse {
    droplet: Droplet,
}

#[derive(Deserialize)]
struct Droplet {
    id: u64,
    status: String,
    #[serde(default)]
    networks: Networks,
}

#[derive(Deserialize, Default)]
struct Networks {
    #[serde(default)]
    v4: Vec<NetworkV4>,
}

#[derive(Deserialize)]
struct NetworkV4 {
    ip_address: String,
    #[serde(rename = "type")]
    network_type: String,
}

#[async_trait]
impl NodeProvider for DigitalOcean {
    async fn start_node(&self, request: NodeRequest) -> Result<NodeDetails, GmlError> {
        let client = reqwest::Client::new();

        let payload = CreateDropletRequest {
            name: Self::new_droplet_name(),
            region: self.region.clone(),
            size: request.instance_type.clone(),
            image: Self::image(),
            ssh_keys: vec![self.ssh_key_id.clone()],
        };

        let url = BASE_URL.to_owned() + "droplets";

        let response = client.post(url)
            .bearer_auth(&self.api_key)
            .header("accept", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| GmlError::from(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(GmlError::from(format!("API Error ({}): {}", status, text)));
        }

        let response_text = response.text()
            .await
            .map_err(|e| GmlError::from(format!("Failed to read response body: {}", e)))?;

        let create_response: CreateDropletResponse = serde_json::from_str(&response_text)
            .map_err(|e| GmlError::from(format!("Failed to parse response: {} - Response body: {}", e, response_text)))?;

        self.wait_for_active(create_response.droplet.id).await
    }

    async fn stop_node(&self, details: NodeDetails) -> Result<NodeDetails, GmlError> {
        let client = reqwest::Client::new();

        let url = format!("{}droplets/{}", BASE_URL, details.id);

        let response = client.delete(&url)
            .bearer_auth(&self.api_key)
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| GmlError::from(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(GmlError::from(format!("API Error ({}): {}", status, text)));
        }

        Ok(details)
    }

    /// Hardcoded root user, works for DigitalOcean's stock Linux images.
    async fn get_user(&self) -> Result<String, GmlError> {
        Ok(DEFAULT_SSH_USER.to_string())
    }

    async fn get_node_types(&self) -> Result<String, GmlError> {
        let client = reqwest::Client::new();

        let url = BASE_URL.to_owned() + "sizes?per_page=200";

        let response = client.get(&url)
            .bearer_auth(&self.api_key)
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| GmlError::from(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(GmlError::from(format!("API Error ({}): {}", status, text)));
        }

        let response_text = response.text()
            .await
            .map_err(|e| GmlError::from(format!("Failed to read response body: {}", e)))?;

        let json_value: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| GmlError::from(format!("Failed to parse response: {} - Response body: {}", e, response_text)))?;

        // `gml` only cares about GPU droplet sizes available in the configured region.
        let region = self.region.clone();
        let gpu_sizes: Vec<&serde_json::Value> = json_value
            .get("sizes")
            .and_then(|s| s.as_array())
            .map(|sizes| {
                sizes.iter().filter(|size| {
                    let is_gpu = size.get("slug")
                        .and_then(|s| s.as_str())
                        .map_or(false, |slug| slug.starts_with("gpu-"));
                    let available_here = size.get("regions")
                        .and_then(|r| r.as_array())
                        .map_or(false, |regions| {
                            regions.iter().any(|r| r.as_str() == Some(region.as_str()))
                        });
                    is_gpu && available_here
                }).collect()
            })
            .unwrap_or_default();

        serde_json::to_string_pretty(&gpu_sizes)
            .map_err(|e| GmlError::from(format!("Failed to pretty print JSON: {}", e)))
    }
}

impl DigitalOcean {
    pub fn new(api_key: String, ssh_key_id: String, region: String) -> DigitalOcean {
        DigitalOcean {
            api_key,
            ssh_key_id,
            region,
        }
    }

    fn image() -> String {
        std::env::var("GML_DIGITALOCEAN_IMAGE").unwrap_or_else(|_| DEFAULT_IMAGE.to_string())
    }

    fn new_droplet_name() -> String {
        format!("gml-{}", uuid::Uuid::new_v4().simple())
    }

    async fn wait_for_active(&self, droplet_id: u64) -> Result<NodeDetails, GmlError> {
        const MAX_RETRIES: u32 = 60; // 10 minutes / 10 seconds = 60 attempts
        const RETRY_DELAY_SECS: u64 = 10;

        let client = reqwest::Client::new();
        let url = format!("{}droplets/{}", BASE_URL, droplet_id);

        for attempt in 1..=MAX_RETRIES {
            let response = client.get(&url)
                .bearer_auth(&self.api_key)
                .header("accept", "application/json")
                .send()
                .await
                .map_err(|e| GmlError::from(format!("Request failed: {}", e)))?;

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                return Err(GmlError::from(format!("API Error ({}): {}", status, text)));
            }

            let response_text = response.text()
                .await
                .map_err(|e| GmlError::from(format!("Failed to read response body: {}", e)))?;

            let get_response: GetDropletResponse = serde_json::from_str(&response_text)
                .map_err(|e| GmlError::from(format!("Failed to parse response: {} - Response body: {}", e, response_text)))?;

            let droplet = get_response.droplet;
            if droplet.status == "active" {
                if let Some(public) = droplet.networks.v4.iter().find(|n| n.network_type == "public") {
                    return Ok(NodeDetails {
                        ip: public.ip_address.clone(),
                        id: droplet.id.to_string(),
                    });
                }
            }

            if attempt < MAX_RETRIES {
                tokio::time::sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
            }
        }

        Err(GmlError::from(format!(
            "Droplet {} did not become active with a public IP after {} minutes. Please try again later.",
            droplet_id, (MAX_RETRIES as u64 * RETRY_DELAY_SECS) / 60
        )))
    }
}

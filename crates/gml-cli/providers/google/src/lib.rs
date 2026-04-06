use async_trait::async_trait;
use gml_core::{NodeDetails, NodeProvider, NodeRequest, error::GmlError};
use gml_core::ssh;
use google_cloud_lro::Poller;
use google_cloud_tpu_v2::client::Tpu;
use google_cloud_tpu_v2::model::{NetworkConfig, Node};
use uuid::Uuid;

/// Default TPU VM software image; override with `GML_GOOGLE_TPU_RUNTIME` if your zone needs another version.
const DEFAULT_TPU_RUNTIME_VERSION: &str = "tpu-ubuntu2204-base";

/// SSH login user for Ubuntu-based TPU VM images (e.g. `tpu-ubuntu2204-base`).
const DEFAULT_TPU_SSH_USER: &str = "ubuntu";

const GOOGLE_AUTH_HELP: &str = "Google Cloud permission or auth error. To fix:\n  \
    1. Set 'project' and 'region' in ~/.gml/config.toml under [google]\n  \
    2. Run: gcloud auth application-default login\n  \
    3. Enable TPU API: gcloud services enable tpu.googleapis.com --project=YOUR_PROJECT\n  \
    4. Use zone for region (e.g. us-central1-a) to match: gcloud compute tpus tpu-vm accelerator-types list --zone=...";

fn map_google_error(e: impl std::fmt::Display) -> GmlError {
    let msg = e.to_string();
    if msg.contains("PERMISSION_DENIED")
        || msg.to_lowercase().contains("permission denied")
        || msg.to_lowercase().contains("access denied")
        || msg.to_lowercase().contains("unauthorized")
    {
        GmlError::from(format!("{}\n\n{}", msg, GOOGLE_AUTH_HELP))
    } else {
        GmlError::from(msg)
    }
}

pub struct Google {
    client: Tpu,
    parent: String,
    /// `[gml] ssh-public-key` from config (same resolution as `gml_core::ssh::get_ssh_public_key`).
    gml_ssh_public_key: Option<String>,
}

impl Google {
    pub async fn new(
        project: Option<String>,
        location: Option<String>,
        gml_ssh_public_key: Option<String>,
    ) -> Result<Google, GmlError> {
        let project = project
            .or_else(|| std::env::var("GOOGLE_CLOUD_PROJECT").ok())
            .ok_or_else(|| GmlError::from("project is required for google provider: set in config or GOOGLE_CLOUD_PROJECT env"))?;
        let location = location.unwrap_or_else(|| "us-central1".to_string());
        let parent = format!("projects/{}/locations/{}", project, location);

        let client = Tpu::builder().build().await.map_err(map_google_error)?;

        Ok(Google {
            client,
            parent,
            gml_ssh_public_key,
        })
    }

    /// GCP instance metadata `ssh-keys` value: `username:one-line-public-key` (see Compute Engine docs).
    fn ssh_keys_metadata_value(&self) -> Result<String, GmlError> {
        let path = ssh::get_ssh_public_key(self.gml_ssh_public_key.as_deref())?;
        let line = ssh::read_ssh_public_key_line(&path)?;
        Ok(format!("{}:{}", DEFAULT_TPU_SSH_USER, line))
    }

    /// Full resource name `projects/.../locations/.../nodes/{id}`, or a short node id
    /// combined with this client's `[google] project` + `region`/`location`.
    fn node_resource_name(&self, id_or_segment: &str) -> String {
        if id_or_segment.starts_with("projects/") {
            id_or_segment.to_string()
        } else {
            format!("{}/nodes/{}", self.parent, id_or_segment)
        }
    }

    fn runtime_version() -> String {
        std::env::var("GML_GOOGLE_TPU_RUNTIME").unwrap_or_else(|_| DEFAULT_TPU_RUNTIME_VERSION.to_string())
    }

    fn new_node_id() -> String {
        format!("gml-{}", Uuid::new_v4().simple())
    }
}

fn node_to_details(node: Node) -> NodeDetails {
    let ip = node
        .network_endpoints
        .first()
        .and_then(|ep| {
            ep.access_config.as_ref().and_then(|ac| {
                let s = ac.external_ip.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            })
        })
        .unwrap_or_default();
    NodeDetails {
        ip,
        id: node.name,
    }
}

#[async_trait]
impl NodeProvider for Google {
    async fn get_node_types(&self) -> Result<String, GmlError> {
        let response = self
            .client
            .list_accelerator_types()
            .set_parent(self.parent.clone())
            .send()
            .await
            .map_err(map_google_error)?;

        serde_json::to_string_pretty(&response)
            .map_err(|e| GmlError::from(format!("Failed to serialize: {}", e)))
    }

    async fn start_node(&self, request: NodeRequest) -> Result<NodeDetails, GmlError> {
        if request.instance_type.trim().is_empty() {
            return Err(GmlError::from(
                "instance type (TPU accelerator type) is required for Google, e.g. from `gml node-types --provider google`",
            ));
        }

        let ssh_keys = self.ssh_keys_metadata_value()?;
        // Request a public IP on the default VPC so `gml connect` can SSH without IAP/tunneling.
        let network_config = NetworkConfig::new().set_enable_external_ips(true);
        let node_spec = Node::new()
            .set_accelerator_type(request.instance_type)
            .set_runtime_version(Google::runtime_version())
            .set_network_config(network_config)
            .set_metadata([("ssh-keys", ssh_keys)]);

        let node = self
            .client
            .create_node()
            .set_parent(self.parent.clone())
            .set_node_id(Google::new_node_id())
            .set_node(node_spec)
            .poller()
            .until_done()
            .await
            .map_err(map_google_error)?;
        Ok(node_to_details(node))
    }

    async fn stop_node(&self, details: NodeDetails) -> Result<NodeDetails, GmlError> {
        let name = self.node_resource_name(&details.id);
        self.client
            .delete_node()
            .set_name(name)
            .poller()
            .until_done()
            .await
            .map_err(map_google_error)?;
        Ok(details)
    }

    async fn get_user(&self) -> Result<String, GmlError> {
        Ok(DEFAULT_TPU_SSH_USER.to_string())
    }
}

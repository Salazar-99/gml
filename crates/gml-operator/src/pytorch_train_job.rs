use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use kube::CustomResource;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group="gml.gerardosalazar.com", version = "v1", kind = "PyTorchTrainJob", namespaced)]
pub struct PyTorchTrainJobSpec {
    image: String,
    nodes: i32,
}
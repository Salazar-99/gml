use futures::{StreamExt, TryStreamExt};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{Api, PatchParams, Patch, ResourceExt},
    core::CustomResourceExt,
    Client,
    runtime::{watcher, WatchStreamExt, wait::{conditions, await_condition}},
};

use crate::pytorch_train_job::PyTorchTrainJob;

mod pytorch_train_job;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::try_default().await?;
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());

    crds.patch("pytorchtrainjobs.gml.gerardosalazar.com",
    &PatchParams::apply("manager"),
        &Patch::Apply(PyTorchTrainJob::crd())).await?;

    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        await_condition(crds, "pytorchtrainjobs.gml.gerardosalazar.com", conditions::is_crd_established())
    ).await??;

    let pytorchtrainjobs: Api<PyTorchTrainJob> = Api::default_namespaced(client.clone());
    let wc = watcher::Config::default();
    let mut apply_stream = watcher(pytorchtrainjobs, wc).applied_objects().boxed();
    while let Some(j) = apply_stream.try_next().await? {
        println!("saw apply {}", j.name_any());
    }

    Ok(())
}

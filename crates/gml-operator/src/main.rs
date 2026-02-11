use std::{sync::Arc, time::Duration};
use futures::StreamExt;
use kube::{
    Api, Client, ResourceExt,
    runtime::controller::{Action, Controller}
};
use crate::pytorch_train_job::PyTorchTrainJob;

mod pytorch_train_job;

#[derive(thiserror::Error, Debug)]
pub enum Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() -> Result<(), kube::Error> {
    let client = Client::try_default().await?;
    let pytorch_train_jobs = Api::<PyTorchTrainJob>::all(client);

    Controller::new(pytorch_train_jobs.clone(), Default::default())
        .run(reconcile, error_policy, Arc::new(()))
        .for_each(|_| futures::future::ready(()))
        .await;
    Ok(())
}

async fn reconcile(obj: Arc<PyTorchTrainJob>, ctx: Arc<()>) -> Result<Action> {
    println!("reconcile request: {}", obj.name_any());
    Ok(Action::requeue(Duration::from_secs(3600)))
}

fn error_policy(_object: Arc<PyTorchTrainJob>, _err: &Error, _ctx: Arc<()>) -> Action {
    Action::requeue(Duration::from_secs(5))
}

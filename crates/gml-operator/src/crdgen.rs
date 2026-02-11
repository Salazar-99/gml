use kube::CustomResourceExt;
use serde_yaml;

mod pytorch_train_job;
use pytorch_train_job::PyTorchTrainJob;

fn main() {
    let yaml = serde_yaml::to_string(&PyTorchTrainJob::crd()).unwrap();
    std::fs::write("pytorch_train_job_crd.yaml", yaml).unwrap();
}
pub fn handle_create_cluster(provider: String, nodes: Option<i32>, _timeout: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating cluster with provider: {} and {:?} nodes", provider, nodes);
    // TODO: Implement cluster creation logic
    Ok(())
}

pub fn handle_delete_cluster(provider: String, _cluster_id: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Deleting cluster with provider: {}", provider);
    // TODO: Implement cluster deletion logic
    Ok(())
}


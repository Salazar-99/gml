use gml_core::NodeProvider;
use gml_core::error::GmlError;
use gml_lambda::Lambda;
use crate::config::ProviderConfig;

pub fn create_provider_handle(provider_name: &str, provider_config: &ProviderConfig) -> Result<Box<dyn NodeProvider>, GmlError> {
    match provider_name {
        "lambda" => {
            let api_key = provider_config.api_key
                .as_ref()
                .ok_or_else(|| GmlError::from("api-key is required for lambda provider, set it in your gml config"))?
                .clone();
            let ssh_key_id = provider_config.ssh_key
                .as_ref()
                .ok_or_else(|| GmlError::from("ssh-key is required for lambda provider, set it in your gml config"))?
                .clone();
            let region = provider_config.region
                .as_ref()
                .ok_or_else(|| GmlError::from("region is required for lambda provider, set it in your gml config"))?
                .clone();
            
            Ok(Box::new(Lambda::new(api_key, ssh_key_id, region)))
        }
        _ => Err(GmlError::from(format!("Unimplemented provider: {}", provider_name)))
    }
}
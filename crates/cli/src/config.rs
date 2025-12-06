use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CONFIG_PATH: &str = "~/.gml/config.toml";

#[derive(Debug)]
pub struct Config {
    providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    #[serde(rename = "api-key")]
    pub api_key: Option<String>,
    #[serde(rename = "ssh-key")]
    pub ssh_key: Option<String>,
}

impl Config {
    /// Get a specific provider by name
    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.get(name)
    }

    /// Get all provider names
    pub fn provider_names(&self) -> Vec<&String> {
        self.providers.keys().collect()
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

pub fn parse_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = expand_tilde(CONFIG_PATH);
    let config_content = fs::read_to_string(&config_path)?;
    
    // Parse the entire TOML as a table of tables
    let toml_value: toml::Value = toml::from_str(&config_content)?;
    
    let mut providers = HashMap::new();
    
    // Extract all top-level tables (provider blocks)
    if let toml::Value::Table(root_table) = toml_value {
        for (key, value) in root_table {
            // Try to deserialize each table as a ProviderConfig
            if let toml::Value::Table(table) = value {
                // Create a new TOML value with just this table and deserialize it
                let table_value = toml::Value::Table(table);
                let table_str = toml::to_string(&table_value)?;
                match toml::from_str::<ProviderConfig>(&table_str) {
                    Ok(provider_config) => {
                        providers.insert(key, provider_config);
                    }
                    Err(_) => {
                        // Skip tables that don't match ProviderConfig structure
                        // (e.g., other config sections)
                    }
                }
            }
        }
    }
    
    Ok(Config { providers })
}

pub fn parse_config_for_provider(provider: &str) -> Result<ProviderConfig, Box<dyn std::error::Error>> {
    let config = parse_config()?;
    config
        .get_provider(provider)
        .cloned()
        .ok_or_else(|| format!("Provider '{}' not found in config", provider).into())
}
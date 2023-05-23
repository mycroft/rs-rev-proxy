use std::collections::HashMap;
use serde::Deserialize;

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub aws: HashMap<String, String>,
    pub cache: HashMap<String, String>,
    pub redirect: Vec<HashMap<String, String>>
}

pub fn load_configuration() -> std::result::Result<AppConfig, Box<dyn std::error::Error>> {
    let config_file = std::fs::read_to_string("config.json")?;
    let config: AppConfig = serde_json::from_str(&config_file)?;
    Ok(config)
}

pub fn spawn_config_reloader(config: Arc<RwLock<AppConfig>>) {
    tokio::spawn(async move {
        loop {
            // debug!("Reloading configuration...")

            // TODO(check): if the config file has not been modified, do not reload it.
            let new_config = match load_configuration() {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("Failed to reload configuration: {}", err);
                    continue;
                }
            };
            
            *config.write().await = new_config;
            
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    });
}

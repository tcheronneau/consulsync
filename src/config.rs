use figment::{Figment, providers::{Format, Toml}};
use serde::{Serialize, Deserialize};
use tracing::{info, debug};
use std::path::PathBuf;
use figment::providers::Env;
use std::collections::HashMap;

use crate::consul::{Consul, ServiceCheck};


#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub consul: Consul,
    pub services: Vec<ServiceConfig>,
}

#[derive(Debug, Serialize,Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub kind: String,
    pub port: u16,
    pub address: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub check: Option<ServiceCheck>,
}
impl ServiceConfig {
    // Merge function to merge service type configuration into service configuration
    fn merge_from(&mut self, service_type_config: HashMap<String, serde_yaml::Value>) {
        for (key, value) in service_type_config {
            // Update service configuration with service type configuration
            match key.as_str() {
                "name" | "kind" => continue, // Skip merging name and kind fields
                _ => {
                    self.update_field(key, value); // Update other fields
                }
            }
        }
    }
    fn update_field(&mut self, key: String, value: serde_yaml::Value) {
        match key.as_str() {
            "port" => {
                self.port = value.as_u64().unwrap() as u16;
            }
            "address" => {
                self.address = value.as_str().unwrap().to_string();
            }
            "tags" => {
                self.tags = value.as_sequence().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
            }
            "check" => {
                let check = value.as_mapping().unwrap();
                let tcp = check.get(&serde_yaml::Value::String("tcp".to_string())).unwrap().as_str().unwrap();
                self.check = Some(ServiceCheck::new(tcp));
            }
            _ => {
                panic!("Unknown field: {}", key);
            }
        }
    }
}

pub fn read(config_file: PathBuf) -> anyhow::Result<Config> {
    info!("Reading config file {config_file:?}");

    let mut config: Config = Figment::new()
        .merge(Toml::file(config_file))
        .merge(Env::prefixed("NIXCONSUL_").split("_"))
        .extract()?;


    for i in 0..config.services.len() {
        let service = &config.services[i];
        let service_type = &service.kind;
        let service_type_config_file = format!("config_{}.toml", service_type);
        debug!("Service type is {:?}", service_type_config_file);
        let service_type_config: HashMap<String, serde_yaml::Value> = Figment::new()
            .merge(Toml::file(service_type_config_file))
            .extract()?;
        if let Some(service_config) = config.services.get_mut(i) {
            service_config.merge_from(service_type_config.clone());
        }
    }

    debug!("Read config is {:?}", config);

    Ok(config)
}

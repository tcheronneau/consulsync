use figment::{Figment, providers::{Format, Toml}};
use serde::{Serialize, Deserialize};
use tracing::{info, debug};
use std::path::PathBuf;
use figment::providers::Env;
use std::collections::HashMap;

use crate::consul::{Consul};
use crate::consul::AgentService;


#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub consul: Consul,
    pub services: Vec<ServiceConfig>,
    pub log_level: Option<String>,
}

#[derive(Debug, Serialize,Deserialize, Clone)]
pub struct ServiceConfig {
    pub name: String,
    pub kind: String,
    pub port: u16,
    pub address: String,
    #[serde(default)]
    pub tags: Vec<String>,
}
impl Default for ServiceConfig {
    fn default() -> Self {
        ServiceConfig {
            name: "".to_string(),
            kind: "".to_string(),
            port: 0,
            address: "".to_string(),
            tags: Vec::new(),
        }
    }
}
impl PartialEq<AgentService> for ServiceConfig {
    fn eq(&self, other: &AgentService) -> bool {
        // Check that everything is the same
        if self.name != other.id {
            return false;
        }
        if self.port != other.port {
            return false;
        }
        if self.address != other.address {
            return false;
        }
        let mut tags = other.tags.clone();
        tags.retain(|tag| tag != "nixconsul");
        if self.tags != tags {
            return false;
        }
        if self.kind != other.kind {
            return false;
        }
        true
    }
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
                let new_tags: Vec<String> = value.as_sequence().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
                self.update_tags(new_tags);
                self.replace_service_name();
            }
            _ => {
                panic!("Unknown field: {}", key);
            }
        }
    }
    fn update_tags(&mut self, tags: Vec<String>) {
        let mut result_list: Vec<&str> = Vec::new();
        info!("Checking tags {:?}",tags);
        for tag in &tags {
            if let Some((key2, _)) = extract_key_value(&tag) {
                let mut found = false;
                for strong_tag in &self.tags {
                    if let Some((key1, _)) = extract_key_value(&strong_tag) {
                        if key1 == key2 {
                            found = true;
                        } 
                        result_list.push(strong_tag);
                    } else {
                        result_list.push(strong_tag);
                    }
                }
                if !found {
                    result_list.push(tag);
                }
            } else {
                result_list.push(tag);
            }
        }
        info!("Result list {:?}",result_list);
        self.tags = result_list.iter().map(|s| s.to_string()).collect();
    }
    fn replace_service_name(&mut self) {
        self.tags = self.tags.iter().map(|tag| tag.replace("SERVICE_NAME", &self.name)).collect();
    }
}
fn extract_key_value(input: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = input.trim().splitn(2, '=').collect();
    match parts.as_slice() {
        [key, value] => Some((key.trim(), value.trim())),
        _ => None,
    }
}

pub fn read(config_file: &PathBuf) -> anyhow::Result<Config> {
    info!("Reading config file {config_file:?}");

    let mut config: Config = Figment::new()
        .merge(Toml::file(config_file))
        .merge(Env::prefixed("NIXCONSUL_").split("_"))
        .extract()?;

    debug!("Consul url {}", config.consul.url);
    for i in 0..config.services.len() {
        let service = &config.services[i];
        let service_type = &service.kind;
        let service_type_config_file = format!("config_{}.toml", service_type);
        info!("Service type is {:?}", service_type_config_file);
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

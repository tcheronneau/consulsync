use figment::{Figment, providers::{Format, Toml}};
use serde::{Serialize, Deserialize};
use tracing::{info, debug};
use std::path::PathBuf;
use figment::providers::Env;
use std::collections::HashMap;

use crate::consul::{Consul, RegisterAgentService, ServiceCheck};


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
    pub tags: Vec<String>,
    pub check: Option<ServiceCheck>,
}

pub fn read(config_file: PathBuf) -> Result<Config, figment::Error> {
    info!("Reading config file {config_file:?}");

    let mut figment = Figment::new()
        .merge(Toml::file(config_file));

    let services: Option<Vec<HashMap<String,serde_json::Value>>> = figment.extract_inner("services").unwrap();
    if let Some(services) = services {
        for service_config in services {
            if let Some(service_type) = service_config.get("kind") {
                let service_type_config_file = format!("config_{}.toml", service_type);
                let service_type_config = Toml::file(service_type_config_file);
                figment = figment.merge(service_type_config);
            }
        }
    }
    //for service_config in services {
    //    let service_type_config_file = format!("config_{}.toml", service_config.kind);
    //    let service_type_config = Toml::file(service_type_config_file);
    //    figment = figment.merge(service_type_config);
    //}
    let config = figment.extract()?;

    debug!("Read config is {:?}", config);

    Ok(config)
}

use figment::{Figment, providers::{Format, Toml}};
use serde::Deserialize;

use crate::consul::{Consul, RegisterAgentService};


#[derive(Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub consul: Consul,
    pub services: Vec<RegisterAgentService>,
}

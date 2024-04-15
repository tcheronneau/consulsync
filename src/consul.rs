use tracing::{info,debug,warn};
use reqwest::{Client, header};
use serde_json;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fmt;

use crate::config::ServiceConfig;

#[derive(Debug, Deserialize, Serialize)]
pub struct Service {
    #[serde(flatten)]
    pub data: HashMap<String, Vec<String>>,
}
impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (service, tags) in self.data.iter() {
            write!(f, "Service : {} has tags : \n", service)?;
            for tag in tags {
                write!(f, "{}\n", tag)?;
            }
            writeln!(f, "")?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentServiceResponse {
    #[serde(flatten)]
    pub data: Vec<AgentService>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AgentService {
    pub kind: String,
    #[serde(rename = "ID")]
    pub id: String,
    pub service: String,
    pub tags: Vec<String>,
    pub meta: HashMap<String, String>,
    pub port: u16,
    pub address: String,
    pub tagged_addresses: serde_json::Value, 
    pub weights: HashMap<String, u16>,
    pub enable_tag_override: bool,
    pub datacenter: String,
}
impl PartialEq<ServiceConfig> for AgentService {
    fn eq(&self, other: &ServiceConfig) -> bool {
        // Check that everything is the same
        if self.id != other.name {
            return false;
        }
        if self.port != other.port {
            return false;
        }
        if self.address != other.address {
            return false;
        }
        let mut tags = self.tags.clone();
        tags.retain(|tag| tag != "nixconsul");
        if tags != other.tags {
            return false;
        }
        if self.kind != other.kind {
            return false;
        }
        true
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceCheck {
    #[serde(rename = "TCP")]
    pub tcp: String,
    pub interval: String,
    pub timeout: String,
}
impl ServiceCheck {
    pub fn new(tcp: &str) -> Self {
        ServiceCheck {
            tcp: tcp.to_string(),
            interval: "10s".to_string(),
            timeout: "5s".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct RegisterAgentService {
    pub kind: String,
    pub name: String,
    pub tags: Vec<String>,
    pub meta: HashMap<String, String>,
    pub port: u16,
    pub address: String,
    pub enable_tag_override: bool,
    pub check: Option<ServiceCheck>,
}
impl RegisterAgentService {
    pub fn new(name: &str, kind: &str, port: u16, address: &str, tags: Vec<String>, check: Option<ServiceCheck>) -> Self {
        RegisterAgentService {
            name: name.to_string(),
            kind: kind.to_string(),
            port,
            address: address.to_string(),
            tags,
            meta: HashMap::new(),
            enable_tag_override: true,
            check,
        }
    }
}
impl From<ServiceConfig> for RegisterAgentService {
    fn from(service: ServiceConfig) -> Self {
        RegisterAgentService {
            name: service.name,
            kind: service.kind,
            port: service.port,
            address: service.address,
            tags: service.tags,
            meta: HashMap::new(),
            enable_tag_override: true,
            check: service.check,
        }
    }
}

#[derive(Debug)]
pub struct ClientError {
    pub message: String,
}
impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> Self {
        ClientError {
            message: format!("Request error: {}", e),
        }
    }
}
impl From<serde_json::Error> for ClientError {
    fn from(err: serde_json::Error) -> Self {
        ClientError {
            message: format!("Formating error: {}", err),
        }
    }
}
impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for ClientError {}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct Consul {
    #[serde(skip)]
    client: Client,
    pub url: String,
}
impl Default for Consul {
    fn default() -> Self {
        Self {
            client: Client::new(),
            url: "http://localhost:8500".to_string(),
        }
    }
}

impl Consul {
    pub fn new(url: &str) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json")); 
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        Consul {
            client: client, 
            url: url.to_string(),
        }
    }

    pub async fn get_catalog_services(&self) -> Result<Service, ClientError> {
        let url = format!("{}/v1/catalog/services", self.url);
        let response = self.client.get(&url).send().await?;
        let body = response.text().await?;
        debug!("Body from catalog service {:?}", &body);
        let services: Service = serde_json::from_str(&body)?;
        Ok(services)
    }
    pub async fn get_agent_services(&self) -> Result<Vec<AgentService>, ClientError> {
        let url = format!("{}/v1/agent/services", self.url);
        let response = self.client.get(&url).send().await?;
        let body = response.text().await?;
        debug!("Body from agent service {:?}", &body);
        let services: HashMap<String, AgentService> = serde_json::from_str(&body)?;
        Ok(services.into_iter().map(|(_, v)| v).collect())
    }
    pub async fn register_agent_service(&self, service: &RegisterAgentService) -> Result<(), ClientError> {
        let url = format!("{}/v1/agent/service/register", self.url);
        let mut service = service.clone();
        service.tags.push("nixconsul".to_string());
        let body = serde_json::to_string(&service)?;
        let response = self.client.put(&url).body(body).send().await?;
        debug!("Response from agent service registration {:?}", &response);
        let status = response.status();
        match status {
            reqwest::StatusCode::OK => {
                info!("Service registration successful");
                Ok(())
            },
            _ => Err(ClientError {
                message: format!("Service registration failed with status: {}", status),
            }),
        }
    }

    pub async fn deregister_agent_service(&self, service_id: &str) -> Result<(), ClientError> {
        let url = format!("{}/v1/agent/service/deregister/{}", self.url, service_id);
        let response = self.client.put(&url).send().await?;
        debug!("Response from agent service deregistration {:?}", &response);
        let status = response.status();
        match status {
            reqwest::StatusCode::OK => {
                info!("Service deregistration successful");
                Ok(())
            },
            _ => Err(ClientError {
                message: format!("Service deregistration failed with status: {}", status),
            }),
        }
    }

    pub async fn get_managed_services(&self) -> Result<Vec<AgentService>, ClientError> {
        let services = self.get_agent_services().await;
        match services {
            Ok(services) => {
                let managed_services: Vec<AgentService> = services.into_iter().filter(|service| {
                    service.tags.contains(&"nixconsul".to_string())
                }).collect();
                Ok(managed_services)
            },
            Err(e) => {
                warn!("Error getting managed services: {:?}", e);
                Err(e)
            }
        }
    }

}

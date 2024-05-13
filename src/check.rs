use async_std::net::TcpStream;
use tracing::{info,debug,warn};
use crate::consul::RegisterAgentService;
use crate::consul::Consul;
use rs_consul::Consul as RsConsul;
use bytes::Bytes;
use std::time::Duration;
use gethostname::gethostname;

#[derive(Debug)]
pub struct ExternalCheck {
    pub name: String,
    pub socket: String,
    pub interval: String,
    pub timeout: String,
}

impl From<RegisterAgentService> for ExternalCheck {
    fn from(service: RegisterAgentService) -> Self {
        ExternalCheck {
            name: service.name,
            socket: format!("{}:{}", service.address, service.port),
            interval: service.check.interval,
            timeout: service.check.timeout,
        }
    }
}

impl ExternalCheck {
    pub async fn service_available(&self) -> bool {
        debug!("Checking if service is available on {}", self.socket);
        let stream = TcpStream::connect(self.socket.clone());
        match stream.await {
            Ok(_) => true,
            Err(_) => {
                warn!("Service {} is not available", &self.name);
                false
            } 
        }
    }
}

impl From<Consul> for RsConsul {
    fn from(consul: Consul) -> Self {
        let config = rs_consul::Config {
            address: consul.url,
            token: None,
            hyper_builder: Default::default(),
        };
        rs_consul::Consul::new(config)
    }
}
pub trait RsConsulExt {
    async fn register_unavailable_service(&self, check: &ExternalCheck) -> anyhow::Result<()>;
    async fn deregister_unavailable_service(&self, check: &ExternalCheck) -> anyhow::Result<()>;
    async fn get_unavailable_services(&self) -> anyhow::Result<Vec<String>>;
}

impl RsConsulExt for RsConsul {
    async fn register_unavailable_service(&self, check: &ExternalCheck) -> anyhow::Result<()> {
        let hostname = match gethostname().into_string() {
            Ok(h) => h,
            Err(_) => "unknown".to_string(),
        };
        let key = format!("consulsync/{}/{}", hostname, check.name);
        let value = Bytes::from("unavailable");
        let req = rs_consul::types::CreateOrUpdateKeyRequest {
            key: key.as_str(),
            namespace: "",
            datacenter: "",
            flags: 0,
            check_and_set: None,
            release: "",
            acquire: "",
        };
        match self.create_or_update_key(req, value.to_vec()).await {
            Ok(_) => Ok(()),
            Err(e) => {
                info!("Error registering service: {}", e);
                Ok(())
            }
        }
    }
    async fn deregister_unavailable_service(&self, check: &ExternalCheck) -> anyhow::Result<()> {
        let hostname = match gethostname().into_string() {
            Ok(h) => h,
            Err(_) => "unknown".to_string(),
        };
        let key = format!("consulsync/{}/{}", hostname, check.name);
        let req = rs_consul::types::DeleteKeyRequest {
            key: key.as_str(),
            namespace: "",
            datacenter: "",
            recurse: false,
            check_and_set: 0,
        };
        match self.delete_key(req).await {
            Ok(_) => Ok(()),
            Err(e) => {
                info!("Error deregistering service: {}", e);
                Ok(())
            }
        }
    }
    async fn get_unavailable_services(&self) -> anyhow::Result<Vec<String>> {
        let hostname = match gethostname().into_string() {
            Ok(h) => h,
            Err(_) => "unknown".to_string(),
        };
        let key = format!("consulsync/{}", hostname);
        let req = rs_consul::types::ReadKeyRequest {
            key: key.as_str(),
            namespace: "",
            datacenter: "",
            recurse: true,
            separator: "",
            index: None,
            consistency: rs_consul::ConsistencyMode::Default,
            wait: Duration::from_secs(1),
        };
        let resp = self.read_key(req).await?;
        let unavailable_services: Vec<String> = resp.iter().filter_map(|r| {
            let parts: Vec<&str> = r.key.split('/').collect();
            Some(parts[parts.len()-1].to_string())
        }).collect();
        info!("Unavailable services: {:?}", unavailable_services);
        Ok(unavailable_services)
    }
}

use async_std::net::TcpStream;
use tracing::{info,debug,warn};
use crate::consul::RegisterAgentService;
use crate::consul::Consul;
use rs_consul::Consul as RsConsul;
use bytes::Bytes;

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
trait RsConsulExt {
    async fn register_unavailable_service(&self, check: &ExternalCheck) -> anyhow::Result<()>;
}

impl RsConsulExt for RsConsul {
    async fn register_unavailable_service(&self, check: &ExternalCheck) -> anyhow::Result<()> {
        let key = format!("consulsync/{}", check.name);
        let value = Bytes::from("unavailable");
        let req = rs_consul::types::CreateOrUpdateKeyRequest {
            key: key.as_str(),
            namespace: "",
            datacenter: "mcth",
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
}

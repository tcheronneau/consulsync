use tracing::{info,debug,warn};
use serde_json;
use async_std::net::TcpStream;
use async_std::io;
use serde::{Serialize, Deserialize};
use std::time::Duration;


#[derive(Debug, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub status: String,
    pub address: String,
    pub port: u16,
}

impl Check {
    pub async fn connect(&self) -> io::Result<TcpStream> {
        let socket = format!("{}:{}",self.address, self.port);
        let stream = io::timeout(
            Duration::from_secs(5),
            async move { TcpStream::connect(socket).await },
        )
        .await?;
        Ok(stream)
    }
}

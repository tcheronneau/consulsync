use tracing::{info, Level};
use tracing_subscriber;

mod consul;
mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let collector = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(collector).expect("setting default subscriber failed");
    let config = config::read("config.toml".into())?;
    info!("Config is {:?}", config);
    let client = consul::Consul::new(&config.consul.base_url);
    for service in config.services {
        let new_service: consul::RegisterAgentService = service.into();
        client.register_agent_service(&new_service).await?;
    }
    //let client = consul::Consul::new("http://192.168.10.42:8500");
    //let aservices = client.get_agent_services().await?;
    //info!("{:?}", aservices);
    //let new_service = consul::RegisterAgentService::new(
    //    "nixconsul",
    //    "consulrs",
    //    8080,
    //    "127.0.0.1",
    //    vec!["nixconsul".to_string()],
    //    Some(consul::ServiceCheck::new(
    //        "localhost:8080",
    //    ))
    //);
    //client.register_agent_service(&new_service).await?;
    //client.deregister_agent_service("nixconsul").await?;

    Ok(())

}

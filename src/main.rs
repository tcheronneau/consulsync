use tracing::{info, Level};
use tracing_subscriber;

mod consul;
mod config;

async fn check_services(config: config::Config) -> anyhow::Result<()> {
    let client = consul::Consul::new(&config.consul.url);
    let managed_services = client.get_managed_services().await?;
    let mut uptodate: Vec<String> = Vec::new();
    for service in managed_services {
        if config.services.iter().any(|s| s == &service) {
            info!("Service {} is already up to date", service.id);
            uptodate.push(service.id);
        } else {
            info!("Service {} is not in config deleting it...", service.id);
            client.deregister_agent_service(&service.id).await?;
        }
    }
    for service in config.services {
        if uptodate.contains(&service.name) {
            continue;
        }
        info!("Registering service {}", service.name);
        client.register_agent_service(&service.into()).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let collector = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .finish();

    tracing::subscriber::set_global_default(collector).expect("setting default subscriber failed");
    let config = config::read("config.toml".into())?;
    info!("Config is {:?}", config.consul);
    check_services(config).await?;

    Ok(())
}

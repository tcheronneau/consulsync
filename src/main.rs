use tracing::{info, Level,debug,error};
use tracing_subscriber;
use std::sync::{mpsc,Arc, Mutex};
use std::{time::Duration, thread};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Config as NotifyConfig, EventKind};
use std::path::Path;

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

fn watch_config_file(
    file_path: String,
    sender: mpsc::Sender<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, NotifyConfig::default()).unwrap();
    watcher.watch(Path::new(&file_path), RecursiveMode::Recursive).unwrap();

    loop {
        debug!("watch_config_file waiting for event");
        match rx.recv() {
            Ok(event) => {
                println!("watch event: {:?}", event);
                if event?.kind.is_modify() {
                    sender.send(())?;
                }
                debug!("watch_config_file done");
            },
            Err(e) => error!("watch error: {:?}", e),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let collector = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(collector).expect("setting default subscriber failed");

    let file_path = "config.toml";
    let config = config::read(file_path.into())?;
    info!("Config is {:?}", config);
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        if let Err(err) = watch_config_file(file_path.to_string(), tx) {
            error!("Error monitoring config file changes: {}", err);
        }
    });

    for _event in rx {
        println!("Config file changed, syncing...");
        thread::sleep(Duration::from_secs(1));
        let new_config = config::read(file_path.into())?;
        check_services(new_config).await?;
    }
    debug!("main done");
    Ok(())

}

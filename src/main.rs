use tracing::{info, Level,debug,error,warn};
use tracing_subscriber;
use std::sync::mpsc;
use std::{time::Duration, thread};
use notify::{PollWatcher, RecursiveMode, Watcher, Config as NotifyConfig};

mod consul;
mod config;

const CONFIG_FILE: &str = "config.toml";


async fn check_services(config: config::Config) -> anyhow::Result<()> {
    let client = consul::Consul::new(&config.consul.url);
    let managed_services = client.get_managed_services().await?;
    let mut uptodate: Vec<String> = Vec::new();
    for service in managed_services {
        if config.services.iter().any(|s| s == &service) {
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
    file_path: &str,
    sender: mpsc::Sender<()>,
) -> anyhow::Result<()> {
    let (file_tx, file_rx) = mpsc::channel();
    let mut watcher = PollWatcher::new(file_tx, NotifyConfig::default().with_manual_polling()).unwrap();
    watcher.watch(file_path.as_ref(), RecursiveMode::Recursive).unwrap();

    std::thread::spawn(move || {
        for res in file_rx {
            match res {
                Ok(event) => {
                    debug!("changed: {:?}", event);
                    sender.send(()).unwrap();
                },
                Err(e) => error!("watch error: {:?}", e),
            }
        }
    });
    loop {
        watcher.poll().unwrap();
        thread::sleep(Duration::from_secs(5));
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let config = match config::read(CONFIG_FILE.into()) {
        Ok(config) => config,
        Err(e) => {
            error!("Error reading config file: {}", e);
            return Err(e);
        }
    };
    let log_level = match config.log_level.clone() {
        Some(level) => level,
        None => "info".to_string(),
    };
    let collector = tracing_subscriber::fmt()
        .with_max_level(log_level.parse::<Level>().unwrap_or(Level::INFO))
        .finish();
    tracing::subscriber::set_global_default(collector).expect("setting default subscriber failed");
    debug!("Config is {:?}", config);

    let (tx, rx) = mpsc::channel();
    check_services(config.clone()).await?;
    thread::spawn(move || {
        if let Err(err) = watch_config_file(CONFIG_FILE, tx) {
            error!("Error monitoring config file changes: {}", err);
        }
    });

    loop {
        match rx.recv() {
            Ok(_) => {
                debug!("Config file changed, syncing...");
                thread::sleep(Duration::from_secs(1));
                let new_config = match config::read(CONFIG_FILE.into()) {
                    Ok(config) => config,
                    Err(e) => {
                        error!("Error reading config file: {}", e);
                        warn!("Using old config");
                        config.clone()
                    }
                };
                check_services(new_config).await?;
            }
            Err(e) => error!("watch error: {:?}", e),
        }
    }
}

use tracing::{info, Level,debug,error,warn};
use tracing_subscriber;
use std::sync::mpsc;
use std::{time::Duration, thread};
use notify::{PollWatcher, RecursiveMode, Watcher, Config as NotifyConfig};
use clap::{arg, command, Parser};
use std::path::PathBuf;
use tokio::task;

mod consul;
mod config;
mod check;

use consul::RegisterAgentService;
use crate::check::{RsConsulExt, ExternalCheck};

//const CONFIG_FILE: &str = "config.toml";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: PathBuf,
}

async fn config_services(config: config::Config) -> anyhow::Result<()> {
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

async fn check_services(config: config::Config, sender: mpsc::Sender<()>) -> anyhow::Result<()> {
    let client = consul::Consul::new(&config.consul.url);
    let rs_client: rs_consul::Consul = consul::Consul::new(&config.consul.url).into();
    let unavailable_services = match rs_client.get_unavailable_services().await {
        Ok(services) => services,
        Err(_) => {
            info!("It seems that there is no existing unavailable services");
            Vec::new()
        }
    };
    for service in &config.services {
        let register_service: RegisterAgentService = service.clone().into();
        let service_check: ExternalCheck = register_service.into(); 
        if !service_check.service_available().await {
            warn!("Service {} is not available", service.name);
            match rs_client.register_unavailable_service(&service_check).await {
                Ok(_) => {
                    info!("Service registered as unavailable");
                    warn!("Deregistering service {} since unavailable", service.name);
                    client.deregister_agent_service(&service.name).await?;
                },
                Err(e) => {
                    warn!("Error registering service as unavailable: {:?}", e);
                }
            }
        } else {
            debug!("Service {} is available", service.name);
            sender.send(()).unwrap();
            if unavailable_services.contains(&service.name) {
                match rs_client.deregister_unavailable_service(&service_check).await {
                    Ok(_) => {
                        info!("Service {} was unavailable, now available", service.name);
                    },
                    Err(e) => {
                        warn!("Error deregistering service as available: {:?}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
 

fn watch_config_file(
    file_path: &std::path::Path,
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

async fn loop_check_services(config: config::Config, sender: mpsc::Sender<()>) {
    loop {
        debug!("Checking services...");
        match check_services(config.clone(), sender.clone()).await {
            Ok(_) => (),
            Err(e) => {
                error!("Error checking service: {}", e);
            }
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
        debug!("Checking services...end");
    }
}

async fn loop_config_services(mut config: config::Config,config_file: PathBuf, file_rx: mpsc::Receiver<()>) {
    loop {
        match file_rx.recv() {
            Ok(_) => {
                debug!("Config file changed, syncing...");
                thread::sleep(Duration::from_secs(1));
                let new_config = match config::read(&config_file) {
                    Ok(config) => config,
                    Err(e) => {
                        error!("Error reading config file: {}", e);
                        warn!("Using old config");
                        config.clone()
                    }
                };
                config = new_config.clone();
                match config_services(new_config).await {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Error registering service: {}", e);
                    }
                }
            }
            Err(e) => error!("watch error: {:?}", e),
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
    } 
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config_file = args.config;
    let config_file_clone = config_file.clone();
    let config = match config::read(&config_file) {
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
    info!("Config is {:?}", config);

    let (tx, rx) = mpsc::channel();
    match config_services(config.clone()).await {
        Ok(_) => (),
        Err(e) => {
            error!("Error registering service: {}", e);
        }
    }
    let tx_clone = tx.clone();
    thread::spawn(move || {
        if let Err(err) = watch_config_file(&config_file_clone, tx) {
            error!("Error monitoring config file changes: {}", err);
        }
    });

    let config_clone = config.clone();
    let check_task = task::spawn(async move {
        loop_check_services(config_clone,tx_clone).await;
    });
    let config_task = task::spawn(async move {
        loop_config_services(config, config_file, rx).await;
    });
    tokio::select! {
        _ = check_task => (),
        _ = config_task => (),
    }
    Ok(())
}

use tracing::{info, Level,debug,error};
use tracing_subscriber;
use std::sync::{mpsc,Arc, Mutex};
use std::{time::Duration, thread};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Config as NotifyConfig, EventKind, Event, Error};
use std::path::Path;
use tokio::task;

mod consul;
mod config;

const CONFIG_PATH: &str = "config.toml";

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

    let config = config::read(CONFIG_PATH.into())?;
    info!("Config is {:?}", config);
    let config = Arc::new(Mutex::new(config));
    let cloned_config = Arc::clone(&config);

    let mut watcher =
        // To make sure that the config lives as long as the function
        // we need to move the ownership of the config inside the function
        // To learn more about move please read [Using move Closures with Threads](https://doc.rust-lang.org/book/ch16-01-threads.html?highlight=move#using-move-closures-with-threads)
        RecommendedWatcher::new(move |result: Result<Event, Error>| {
            let event = result.unwrap();

            if event.kind.is_modify() {
                match config::read(CONFIG_PATH.into()) {
                    Ok(new_config) => { 
                        *cloned_config.lock().unwrap() = new_config;
                        futures::executor::block_on( async {
                            check_services(cloned_config
                                .lock()
                                .unwrap()
                                .clone())
                            .await
                            .unwrap();
                        });
                    },
                    Err(error) => println!("Error reloading config: {:?}", error),
                }
            }
        },notify::Config::default())?;

    watcher.watch(Path::new(CONFIG_PATH), RecursiveMode::Recursive)?;
    loop {}

}

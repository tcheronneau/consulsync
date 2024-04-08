use config::{Config as AppConfig, File, Environment, ConfigError};
use notify::{RecommendedWatcher, Watcher, RecursiveMode, Config as NotifyConfig};
use serde::Deserialize;
use std::sync::mpsc::channel;
use std::thread;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Define a struct that represents your application's configuration
#[derive(Debug, Deserialize)]
struct Settings {
    log_level: String,
    server_port: u16,
}

impl Settings {
    // Function to create a new Settings from the configuration sources
    fn new() -> Result<Self, ConfigError> {
        let mut s = AppConfig::new();
        // Start off by merging in the "default" configuration file
        s.merge(File::with_name("Settings"))?;
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_LOG_LEVEL=info` would set the `log_level` key
        s.merge(Environment::with_prefix("APP"))?;
        // You can deserialize (and thus freeze) the entire configuration as
        s.try_into()
    }
}

fn main() -> Result<(), ConfigError> {
    // Attempt to read the configuration
    let settings = Arc::new(Mutex::new(Settings::new()?));
    println!("Current configuration: {:?}", *settings.lock().unwrap());

    // Set up a channel to receive the file watch events
    let (tx, rx) = channel();
    // Create a watcher object, delivering debounced events.
    let mut watcher = RecommendedWatcher::new(tx, NotifyConfig::default()).unwrap();

    // Watch for changes in the configuration file
    watcher.watch(Path::new("Settings.toml"), RecursiveMode::Recursive).unwrap();

    // Clone the Arc to be used inside the thread
    let settings_clone = Arc::clone(&settings);

    // Use a separate thread to handle file changes
    thread::spawn(move || {
        for _ in rx {
            println!("Configuration file changed!");
            match Settings::new() {
                Ok(updated_settings) => {
                    let mut settings = settings_clone.lock().unwrap();
                    *settings = updated_settings;
                    println!("Configuration reloaded: {:?}", *settings);
                }
                Err(e) => println!("Failed to reload configuration: {:?}", e),
            }
        }
    });

    loop {
        // Keep the main thread alive and sleeping
        thread::sleep(Duration::from_secs(1));
    }
}

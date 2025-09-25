mod config;
mod error;

use crate::config::Config;

fn main() {
    match Config::load() {
        Ok(config) => {
            println!("Config loaded successfully: {:#?}", config);
        }
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
        }
    }
}

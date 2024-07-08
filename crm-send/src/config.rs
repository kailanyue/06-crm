use std::{fs::File, io::Read};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub pk: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        if let Ok(reader) = File::open("send.yml") {
            return serde_yaml::from_reader(reader)
                .context("Failed to parse configuration from send.yml");
        }

        if let Ok(reader) = File::open("/etc/config/send.yml") {
            return serde_yaml::from_reader(reader)
                .context("Failed to parse configuration from /etc/config/send.yml");
        }

        if let Ok(path) = std::env::var("SEND_CONFIG") {
            let mut file = File::open(&path).context(format!(
                "Failed to open configuration file from path: {}",
                path
            ))?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .context("Failed to read configuration file contents")?;
            return serde_yaml::from_str(&contents)
                .context("Failed to parse configuration from environment variable metadata");
        }
        bail!("Failed to load configuration");
    }
}

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub pk: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub db_url: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        // read from  ./user_stat.yml, or /etc/config/user_stat.yml, or from env CHAT_CONFIG
        // try to load from "user_stat.yml"
        if let Ok(reader) = File::open("user_stat.yml") {
            return serde_yaml::from_reader(reader)
                .context("Failed to parse configuration from user_stat.yml");
        }

        // try to load from "/etc/config/user_stat.yml"
        if let Ok(reader) = File::open("/etc/config/user_stat.yml") {
            return serde_yaml::from_reader(reader)
                .context("Failed to parse configuration from /etc/config/user_stat.yml");
        }

        // try to load from env CHAT_CONFIG
        if let Ok(path) = std::env::var("CHAT_CONFIG") {
            let mut file = File::open(&path).context(format!(
                "Failed to open configuration file from path: {}",
                path
            ))?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .context("Failed to read configuration file contents")?;
            return serde_yaml::from_str(&contents)
                .context("Failed to parse configuration from environment variable CHAT_CONFIG");
        }
        bail!("Failed to load configuration");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_load_config() -> Result<()> {
        let config = AppConfig::load().unwrap();

        assert_eq!(config.server.port, 50001);
        assert!(config
            .server
            .db_url
            .starts_with("postgres://postgres:postgres@"));

        Ok(())
    }
}

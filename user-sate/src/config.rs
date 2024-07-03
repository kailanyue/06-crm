use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read};

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub db_url: String,
}

impl ServerConfig {
    pub fn load() -> Result<Self> {
        // read from  ./notify.yml, or /etc/config/notify.yml, or from env CHAT_CONFIG
        // try to load from "notify.yml"
        if let Ok(reader) = File::open("app.yml") {
            return serde_yaml::from_reader(reader)
                .context("Failed to parse configuration from notify.yml");
        }

        // try to load from "/etc/config/notify.yml"
        if let Ok(reader) = File::open("/etc/config/app.yml") {
            return serde_yaml::from_reader(reader)
                .context("Failed to parse configuration from /etc/config/notify.yml");
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
        let config = ServerConfig::load().unwrap();

        assert_eq!(config.port, 6688);
        // assert_eq!(
        //     config.db_url,
        //     "postgres://postgres:postgres@192.168.1.9:5432/stats"
        // );

        Ok(())
    }
}

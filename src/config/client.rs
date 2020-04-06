use crate::args::Args;
use crate::config::ConfigFile;
use crate::errors::*;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ClientConfig {
    pub socket: Option<PathBuf>,
    pub proxy: Option<String>,
}

impl ClientConfig {
    pub fn load(args: &Args) -> Result<ClientConfig> {
        let config = ConfigFile::load(args)?;
        Self::build(config, args)
    }

    fn build(config: ConfigFile, _args: &Args) -> Result<ClientConfig> {
        Ok(ClientConfig {
            socket: config.daemon.socket,
            proxy: config.daemon.proxy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_client_config() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
"#).unwrap();
        let config = ClientConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, ClientConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
        });
    }
}

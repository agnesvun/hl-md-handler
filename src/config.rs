use config::{Config, File};
use serde::Deserialize;

const CONFIG_PATH: &str = "config/config.toml";

#[derive(Debug, Deserialize)]
pub struct Cfg {
    pub grpc: Grpc,
    pub stream: Stream,
}

#[derive(Debug, Deserialize)]
pub struct Grpc {
    pub endpoint: String,
    pub auth_token: String,
}

#[derive(Debug, Deserialize)]
pub struct Stream {
    pub coins: Vec<String>,
    pub n_levels: u32,
}

impl Cfg {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config = Config::builder()
            .add_source(File::with_name(CONFIG_PATH))
            .build()?;
        let cfg: Cfg = config.try_deserialize()?;

        Ok(cfg)
    }
}

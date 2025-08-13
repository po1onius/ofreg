use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{fs::read_to_string, path::Path, sync::OnceLock};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub target_dir: String,
    pub ignore_path: Vec<String>,
    pub ignore_cmd: Vec<String>,
}

pub static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn init_config(path: impl AsRef<Path>) -> anyhow::Result<()> {
    let config_str = read_to_string(path)?;
    let config = toml::from_str::<Config>(&config_str)?;
    CONFIG
        .set(config)
        .map_err(|_| anyhow!("config set error"))?;
    Ok(())
}

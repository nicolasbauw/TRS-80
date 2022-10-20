use std::fs;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub screen: ScreenConfig,
    pub memory: MemConfig,
    pub debug: Debug,
}

#[derive(Debug, Deserialize)]
pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct MemConfig {
    pub romfile: String,
    pub ram: u16,
}

#[derive(Debug, Deserialize)]
pub struct Debug {
    pub iodevices: Option<bool>,
}

pub fn load_config_file() -> Result<Config, std::io::Error> {
    let f = "config/config.toml";
    let buf = fs::read_to_string(f)?;
    let config: Config = toml::from_str(&buf)?;
    Ok(config)
}
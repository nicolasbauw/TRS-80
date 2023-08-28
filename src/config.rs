use directories::UserDirs;
use serde_derive::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub display: ScreenConfig,
    pub memory: MemConfig,
    pub storage: StorageConfig,
    pub debug: Debug,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
    pub font: String,
    pub font_size: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MemConfig {
    pub rom: String,
    pub ram: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageConfig {
    pub tape_path: PathBuf,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Debug {
    pub iodevices: Option<bool>,
}

pub fn load_config_file() -> Result<Config, std::io::Error> {
    let user_dirs = UserDirs::new().ok_or(std::io::ErrorKind::Other);
    let mut cfg = user_dirs?.home_dir().to_path_buf();
    cfg.push(".config/trust80/config.toml");
    let buf = fs::read_to_string(cfg)?;
    let config: Config = toml::from_str(&buf).map_err(|_e| std::io::ErrorKind::Other)?;
    Ok(config)
}

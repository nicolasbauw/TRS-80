use std::{fs, path::PathBuf};
use serde_derive::Deserialize;
use directories::UserDirs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub display: ScreenConfig,
    pub memory: MemConfig,
    pub storage: StorageConfig,
    pub keyboard: KeyboardConfig,
    pub debug: Debug,
}

#[derive(Debug, Deserialize)]
pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
    pub font: String,
    pub font_size: u16,
}

#[derive(Debug, Deserialize)]
pub struct MemConfig {
    pub rom: String,
    pub ram: u16,
}

#[derive(Debug, Deserialize)]
pub struct KeyboardConfig {
    pub repeat_delay: u64,
    pub keypress_timeout: u64,
    pub memclear_delay: u64,
}

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    pub tape_path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Debug {
    pub iodevices: Option<bool>,
}

pub fn load_config_file() -> Result<Config, std::io::Error> {
    let user_dirs = UserDirs::new().unwrap();
    let mut cfg = user_dirs.home_dir().to_path_buf();
    cfg.push(".config/trust80/config.toml");
    let buf = fs::read_to_string(cfg)?;
    let config: Config = toml::from_str(&buf)?;
    Ok(config)
}
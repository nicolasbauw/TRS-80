use directories::UserDirs;
use serde_derive::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

use crate::machine::MachineError;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub display: ScreenConfig,
    pub memory: MemConfig,
    pub storage: StorageConfig,
    pub debug: Debug,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
    pub font: String,
    pub font_size: u16,
    /// Zoom level applied at startup (F1-F4 during a session) - "half",
    /// "normal", "x2" or "fullscreen". Absent or unrecognized falls back to
    /// "normal", the same way bytebox's own `default_zoom` does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_zoom: Option<String>,
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

fn config_path() -> Result<PathBuf, MachineError> {
    let user_dirs = UserDirs::new().ok_or(MachineError::ConfigFile)?;
    let mut cfg = user_dirs.home_dir().to_path_buf();
    cfg.push(".config/trust80/config.toml");
    Ok(cfg)
}

pub fn load_config_file() -> Result<Config, MachineError> {
    let buf = fs::read_to_string(config_path()?)?;
    let config: Config = toml::from_str(&buf).map_err(|_e| MachineError::ConfigFileFmt)?;
    Ok(config)
}

/// Rewrites just the `[display]` section of the config file, leaving
/// everything else - including comments - intact. Serializing the whole
/// `Config` would be shorter, but would rewrite the user's file end to end:
/// comments lost, sections reordered, defaults suddenly spelled out. Too
/// high a price for a file people hand-edit.
pub fn save_display_config(display: &ScreenConfig) -> Result<(), MachineError> {
    let body = toml::to_string(display).map_err(|_e| MachineError::ConfigFileFmt)?;
    write_config_section("display", &body)
}

fn write_config_section(section: &str, body: &str) -> Result<(), MachineError> {
    let path = config_path()?;
    // The config directory may not exist yet (fresh checkout, never run) -
    // without this, saving a setting would fail on a completely fresh
    // install, the one time it matters most that it doesn't.
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    // Missing file: start from empty content rather than failing - saving
    // a setting should work even on the very first launch.
    let existing = fs::read_to_string(&path).unwrap_or_default();
    fs::write(&path, replace_section(&existing, section, body))?;
    Ok(())
}

fn replace_section(content: &str, section: &str, body: &str) -> String {
    let header = format!("[{section}]");
    let mut kept: Vec<&str> = Vec::new();
    let mut skipping = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == header {
            skipping = true;
            continue;
        }
        // Any other section header ends the skip: only that section's own
        // body gets dropped.
        if skipping {
            if trimmed.starts_with('[') {
                skipping = false;
            } else {
                continue;
            }
        }
        kept.push(line);
    }
    while kept.last().is_some_and(|l| l.trim().is_empty()) {
        kept.pop();
    }

    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(&header);
    out.push('\n');
    out.push_str(body.trim_end());
    out.push('\n');
    out
}

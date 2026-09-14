use directories::UserDirs;
use serde_derive::{Deserialize, Serialize};
use std::{fmt, fs, path::PathBuf};

#[derive(Debug)]
pub enum ConfigError {
    ConfigFile,
    ConfigFileFmt,
    IOError,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ConfigError::ConfigFile => "Can't load config file",
            ConfigError::ConfigFileFmt => "Bad config file format",
            ConfigError::IOError => "I/O error",
        })
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(_e: std::io::Error) -> ConfigError {
        ConfigError::IOError
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub display: ScreenConfig,
    #[serde(default)]
    pub crt: CrtConfig,
    pub memory: MemConfig,
    pub storage: StorageConfig,
    pub debug: Debug,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ScreenConfig {
    /// Zoom level applied at startup (F1-F4 during a session) - "half",
    /// "normal", "x2" or "fullscreen". Absent or unrecognized falls back to
    /// "normal", the same way bytebox's own `default_zoom` does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_zoom: Option<String>,
}

/// CRT shader tuning (F6 panel), saved as its own `[crt]` section - same
/// idea as bytebox's own `CrtConfig`. Every field is optional: an absent
/// (or partial) `[crt]` section falls back field by field to `Display`'s
/// own tuned defaults, not `zilog_silicon`'s bare ones - see
/// `crt_settings_from_config`.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CrtConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask_cell_px: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask_min: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask_strength: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scanline_beam: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scanline_strength: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beam_bloom: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bright_boost: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizontal_blur: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pixels_per_scanline: Option<f32>,
}

/// Applies the saved `[crt]` section over `defaults`, field by field - a
/// partial (or absent) section stays perfectly valid. `defaults` is
/// `Display`'s own tuned baseline (see its `new()`), not
/// `CrtSettings::default()`: that library default is tuned for bytebox's
/// own 2x buffer oversampling, not our 4.5x/font.
pub fn crt_settings_from_config(
    crt: &CrtConfig,
    defaults: zilog_silicon::renderer::CrtSettings,
) -> zilog_silicon::renderer::CrtSettings {
    zilog_silicon::renderer::CrtSettings {
        mask_cell_px: crt.mask_cell_px.unwrap_or(defaults.mask_cell_px),
        mask_min: crt.mask_min.unwrap_or(defaults.mask_min),
        mask_strength: crt.mask_strength.unwrap_or(defaults.mask_strength),
        scanline_beam: crt.scanline_beam.unwrap_or(defaults.scanline_beam),
        scanline_strength: crt.scanline_strength.unwrap_or(defaults.scanline_strength),
        beam_bloom: crt.beam_bloom.unwrap_or(defaults.beam_bloom),
        bright_boost: crt.bright_boost.unwrap_or(defaults.bright_boost),
        horizontal_blur: crt.horizontal_blur.unwrap_or(defaults.horizontal_blur),
        pixels_per_scanline: crt
            .pixels_per_scanline
            .unwrap_or(defaults.pixels_per_scanline),
    }
}

/// Reciprocal of [`crt_settings_from_config`], for saving: every field is
/// filled in, even ones left at their default - saving freezes a look, so a
/// later change to the tuned defaults shouldn't silently change what the
/// user already chose to keep.
pub fn crt_settings_to_config(settings: zilog_silicon::renderer::CrtSettings) -> CrtConfig {
    CrtConfig {
        mask_cell_px: Some(settings.mask_cell_px),
        mask_min: Some(settings.mask_min),
        mask_strength: Some(settings.mask_strength),
        scanline_beam: Some(settings.scanline_beam),
        scanline_strength: Some(settings.scanline_strength),
        beam_bloom: Some(settings.beam_bloom),
        bright_boost: Some(settings.bright_boost),
        horizontal_blur: Some(settings.horizontal_blur),
        pixels_per_scanline: Some(settings.pixels_per_scanline),
    }
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

fn config_path() -> Result<PathBuf, ConfigError> {
    let user_dirs = UserDirs::new().ok_or(ConfigError::ConfigFile)?;
    let mut cfg = user_dirs.home_dir().to_path_buf();
    cfg.push(".config/trust80/config.toml");
    Ok(cfg)
}

pub fn load_config_file() -> Result<Config, ConfigError> {
    let buf = fs::read_to_string(config_path()?)?;
    let config: Config = toml::from_str(&buf).map_err(|_e| ConfigError::ConfigFileFmt)?;
    Ok(config)
}

/// Rewrites just the `[display]` section of the config file, leaving
/// everything else - including comments - intact. Serializing the whole
/// `Config` would be shorter, but would rewrite the user's file end to end:
/// comments lost, sections reordered, defaults suddenly spelled out. Too
/// high a price for a file people hand-edit.
pub fn save_display_config(display: &ScreenConfig) -> Result<(), ConfigError> {
    let body = toml::to_string(display).map_err(|_e| ConfigError::ConfigFileFmt)?;
    write_config_section("display", &body)
}

/// Same idea as [`save_display_config`], for the `[crt]` section.
pub fn save_crt_config(crt: &CrtConfig) -> Result<(), ConfigError> {
    let body = toml::to_string(crt).map_err(|_e| ConfigError::ConfigFileFmt)?;
    write_config_section("crt", &body)
}

fn write_config_section(section: &str, body: &str) -> Result<(), ConfigError> {
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

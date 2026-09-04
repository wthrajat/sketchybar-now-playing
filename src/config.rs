use crate::error::{Error, Result};
use serde::Deserialize;
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone)]
pub struct Config {
    pub separator: String,
    pub hide_output: bool,
    pub max_chars: usize,
    pub icon_overrides: HashMap<String, String>,
    /// Fixed glyph for all players. Empty counts as unset.
    pub static_icon: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            separator: " - ".to_owned(),
            hide_output: false,
            max_chars: 20,
            icon_overrides: HashMap::new(),
            static_icon: None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    separator: Option<String>,
    hide_output: Option<bool>,
    max_chars: Option<usize>,
    static_icon: Option<String>,
    #[serde(default)]
    icons: HashMap<String, String>,
}

impl Config {
    /// Explicit path, else ~/.config/sketchybar/now-playing.toml, else defaults.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        if let Some(path) = path {
            return Self::load_file(path);
        }
        if let Some(home) = std::env::var_os("HOME") {
            let candidate =
                std::path::PathBuf::from(home).join(".config/sketchybar/now-playing.toml");
            if candidate.is_file() {
                return Self::load_file(&candidate);
            }
        }
        Ok(Self::default())
    }

    fn load_file(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("read {}: {e}", path.display())))?;
        let file: ConfigFile = toml::from_str(&text)
            .map_err(|e| Error::Config(format!("parse {}: {e}", path.display())))?;
        let mut cfg = Self::default();
        if let Some(sep) = file.separator {
            cfg.separator = sep;
        }
        if let Some(hide) = file.hide_output {
            cfg.hide_output = hide;
        }
        if let Some(max) = file.max_chars {
            cfg.max_chars = max;
        }
        if let Some(glyph) = file.static_icon.filter(|s| !s.is_empty()) {
            cfg.static_icon = Some(glyph);
        }
        cfg.icon_overrides = file.icons;
        Ok(cfg)
    }
}

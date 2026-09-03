use crate::error::{Error, Result};
use serde::Deserialize;
use std::{collections::HashMap, path::Path};

/// Runtime config. Scrolling stays native to SketchyBar
/// (`scroll_texts`/`max_chars`), so only playback display knobs live here.
#[derive(Debug, Clone)]
pub struct Config {
    /// Joiner between fields, e.g. `" - "` gives `title - artist`.
    pub separator: String,
    /// Hide the item (`drawing=off` / empty line) when nothing plays.
    pub hide_output: bool,
    /// Hint for pre-sizing the label buffer; bar truncates via `max_chars`.
    pub max_chars: usize,
    pub icon_overrides: HashMap<String, String>,
    /// Fixed glyph for every player. `None` keeps per player icons.
    /// Empty strings count as unset so `static_icon = ""` stays default.
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

/// Partial file form. Every field is optional so sparse TOML just works.
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
    /// Load from `path`, or probe the conventional location, or fall back
    /// to built-in defaults. Probing means `sync` ticks inside SketchyBar
    /// (which cannot receive env config) honor the same file as the daemon
    /// without any extra plumbing. Single allocation per field; called once
    /// at startup, never in the hot loop.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        if let Some(path) = path {
            return Self::load_file(path);
        }
        // Conventional home config. Missing file or HOME is not an error.
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

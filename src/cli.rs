use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Now Playing for SketchyBar on macOS.
#[derive(Debug, Parser)]
#[command(name = "sketchybar-now-playing", version, about)]
pub struct Cli {
    /// Optional config file. Defaults to `config.toml` lookup, else built-ins.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Print current track once (for `update_freq` polling fallback).
    Get {
        /// Emit compact JSON instead of `title - artist` text.
        #[arg(long)]
        json: bool,
    },
    /// Print NDJSON updates to stdout (pipe into `jq` / scripts).
    Stream,
    /// Event-driven loop: trigger SketchyBar only when the track changes.
    Daemon {
        /// Custom event the bar item subscribes to.
        #[arg(long, default_value = "now_playing_change")]
        event: String,
        /// If set, call `sketchybar --set ITEM` directly instead of `--trigger`.
        #[arg(long)]
        set: Option<String>,
    },
    /// Snapshot once and push it into ITEM via `sketchybar --set`.
    /// Powers the `update_freq` fallback so a (re)loaded item converges
    /// label, icon and visibility in one call, with no shell parsing.
    Sync {
        /// Bar item to update.
        item: String,
    },
    /// Media controls (sent to the active Now Playing client).
    Play,
    Pause,
    Toggle,
    Next,
    Prev,
}

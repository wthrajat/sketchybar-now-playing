use crate::{
    config::Config,
    error::{Error, Result},
    icons::icon_for_with_overrides,
    track::Track,
};
use std::process::Command;

/// Fire a custom event, e.g. `sketchybar --trigger now_playing_change ...`.
/// Keys are uppercase by SketchyBar convention (`$NAME`, `$SENDER`), so the
/// plugin reads `$TITLE`, `$LABEL`, `$ICON`, `$PLAYING` and friends.
/// Direct `exec` (no shell): values with spaces stay a single `KEY=value`
/// arg, so track titles cannot inject commands.
pub fn trigger(event: &str, track: Option<&Track>, cfg: &Config) -> Result<()> {
    let mut cmd = Command::new("sketchybar");
    cmd.arg("--trigger").arg(event);
    match track {
        Some(t) => {
            let label = t.label(&cfg.separator);
            let icon = icon_for_with_overrides(&cfg.icon_overrides, &t.bundle_id);
            cmd.arg(format!("TITLE={}", t.title))
                .arg(format!("ARTIST={}", t.artist))
                .arg(format!("ALBUM={}", t.album))
                .arg(format!("BUNDLE={}", t.bundle_id))
                .arg(format!("PLAYING={}", t.playing))
                .arg(format!("LABEL={label}"))
                .arg(format!("ICON={icon}"));
        }
        None => {
            cmd.arg("PLAYING=false").arg("LABEL=").arg("ICON=");
        }
    }
    let status = cmd
        .status()
        .map_err(|e| Error::SketchyBar(format!("spawn sketchybar --trigger: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::SketchyBar(format!("--trigger exited with {status}")))
    }
}

/// Direct `--set` path for `daemon --set ITEM` (skips the event round-trip).
pub fn set(item: &str, track: Option<&Track>, cfg: &Config) -> Result<()> {
    let mut cmd = Command::new("sketchybar");
    cmd.arg("--set").arg(item);
    match track {
        Some(t) => {
            let label = t.label(&cfg.separator);
            let icon = icon_for_with_overrides(&cfg.icon_overrides, &t.bundle_id);
            cmd.arg(format!("label={label}"))
                .arg(format!("icon={icon}"))
                .arg("drawing=on");
        }
        None if cfg.hide_output => {
            cmd.arg("drawing=off");
        }
        None => {
            cmd.arg("label=No player").arg("drawing=on");
        }
    }
    let status = cmd
        .status()
        .map_err(|e| Error::SketchyBar(format!("spawn sketchybar --set: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::SketchyBar(format!("--set exited with {status}")))
    }
}

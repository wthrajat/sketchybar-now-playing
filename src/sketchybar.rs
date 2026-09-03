use crate::{
    config::Config,
    error::{Error, Result},
    icons::{icon_for_with_overrides, toggle_icon, ICON_NEXT, ICON_PREV},
    track::Track,
};
use std::process::Command;

/// Sibling control items created by `items/now_playing.sh` when controls
/// are enabled. Each is a standalone bar item (`<base>.prev`,
/// `<base>.toggle`, `<base>.next`, plus a `<base>.sep` pipe), so every
/// button gets its own click target.
const CONTROL_SUFFIXES: [&str; 4] = [".sep", ".prev", ".toggle", ".next"];

/// Which control `item` is, if it is one. Matches on the trailing suffix
/// only, so any base name works (`now_playing.prev`, `media.toggle`).
#[inline]
fn control_of(item: &str) -> Option<&str> {
    CONTROL_SUFFIXES
        .iter()
        .find(|suffix| item.ends_with(*suffix))
        .copied()
}

/// Resolve the icon: explicit `static_icon` wins, otherwise the per
/// player map. Borrowed either way, so the hot path stays allocation free.
#[inline]
fn resolve_icon<'a>(cfg: &'a Config, bundle_id: &str) -> &'a str {
    cfg.static_icon
        .as_deref()
        .unwrap_or_else(|| icon_for_with_overrides(&cfg.icon_overrides, bundle_id))
}

/// Fire a custom event, e.g. `sketchybar --trigger now_playing_change ...`.
/// Keys are uppercase by SketchyBar convention (`$NAME`, `$SENDER`), so the
/// plugin reads `$TITLE`, `$LABEL`, `$ICON`, `$PLAYING` and friends.
/// Keys are uppercase by SketchyBar convention (`$NAME`, `$SENDER`), so the
/// plugin reads `$TITLE`, `$LABEL`, `$ICON`, `$PLAYING` and friends.
/// `PREV_ICON` / `TOGGLE_ICON` / `NEXT_ICON` feed the optional control
/// items; `TOGGLE_ICON` follows playback (pause while playing). Extra keys
/// are ignored by older plugins, so this stays backward compatible.
/// Direct `exec` (no shell): values with spaces stay a single `KEY=value`
/// arg, so track titles cannot inject commands.
pub fn trigger(event: &str, track: Option<&Track>, cfg: &Config) -> Result<()> {
    let mut cmd = Command::new("sketchybar");
    cmd.arg("--trigger").arg(event);
    match track {
        Some(t) => {
            let label = t.label(&cfg.separator);
            let icon = resolve_icon(cfg, &t.bundle_id);
            let toggle = toggle_icon(t.playing);
            cmd.arg(format!("TITLE={}", t.title))
                .arg(format!("ARTIST={}", t.artist))
                .arg(format!("ALBUM={}", t.album))
                .arg(format!("BUNDLE={}", t.bundle_id))
                .arg(format!("PLAYING={}", t.playing))
                .arg(format!("LABEL={label}"))
                .arg(format!("ICON={icon}"))
                .arg(format!("PREV_ICON={ICON_PREV}"))
                .arg(format!("TOGGLE_ICON={toggle}"))
                .arg(format!("NEXT_ICON={ICON_NEXT}"));
        }
        None => {
            cmd.arg("PLAYING=false")
                .arg("LABEL=")
                .arg("ICON=")
                .arg(format!("PREV_ICON={ICON_PREV}"))
                .arg(format!("TOGGLE_ICON={}", toggle_icon(false)))
                .arg(format!("NEXT_ICON={ICON_NEXT}"));
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
/// Control items (`<base>.prev` / `.toggle` / `.next` / `.sep`) render just
/// their icon or separator, so every subscribed item converges through the
/// same polling fallback (`sync <name>`) with no shell parsing.
pub fn set(item: &str, track: Option<&Track>, cfg: &Config) -> Result<()> {
    if let Some(kind) = control_of(item) {
        return set_control(item, kind, track);
    }
    let mut cmd = Command::new("sketchybar");
    cmd.arg("--set").arg(item);
    match track {
        Some(t) => {
            let label = t.label(&cfg.separator);
            let icon = resolve_icon(cfg, &t.bundle_id);
            // Freeze the scroller while paused so idle text sits still.
            let scroll = if t.playing {
                "scroll_texts=on"
            } else {
                "scroll_texts=off"
            };
            cmd.arg(format!("label={label}"))
                .arg(format!("icon={icon}"))
                .arg(scroll)
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

/// Update one control sibling. Controls are hidden whenever nothing plays,
/// regardless of `hide_output`: a transport button with no player behind
/// it would only ever error.
fn set_control(item: &str, kind: &str, track: Option<&Track>) -> Result<()> {
    let mut cmd = Command::new("sketchybar");
    cmd.arg("--set").arg(item);
    match track {
        Some(t) => {
            // `kind` is one of CONTROL_SUFFIXES; the wildcard keeps this
            // total if the set ever grows (new buttons read as toggle).
            if kind == ".sep" {
                cmd.arg("label=|").arg("icon.drawing=off");
            } else {
                let icon = match kind {
                    ".prev" => ICON_PREV,
                    ".next" => ICON_NEXT,
                    _ => toggle_icon(t.playing),
                };
                cmd.arg(format!("icon={icon}")).arg("label.drawing=off");
            }
            cmd.arg("drawing=on");
        }
        None => {
            cmd.arg("drawing=off");
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

/// `daemon --set` fan-out: update the main item, then push the same
/// snapshot into the control siblings. Missing siblings (controls
/// disabled) fail quietly; only the main item's result is reported.
pub fn set_with_controls(item: &str, track: Option<&Track>, cfg: &Config) -> Result<()> {
    set(item, track, cfg)?;
    for suffix in CONTROL_SUFFIXES {
        let sibling = format!("{item}{suffix}");
        let _ = set(&sibling, track, cfg);
    }
    Ok(())
}

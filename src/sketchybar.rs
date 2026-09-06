use crate::{
    config::Config,
    error::{Error, Result},
    icons::{icon_for_with_overrides, toggle_icon, ICON_NEXT, ICON_PREV},
    track::Track,
};
use std::process::Command;

const CONTROL_SUFFIXES: [&str; 4] = [".sep", ".prev", ".toggle", ".next"];

#[inline]
fn control_of(item: &str) -> Option<&str> {
    CONTROL_SUFFIXES
        .iter()
        .find(|suffix| item.ends_with(*suffix))
        .copied()
}

#[inline]
fn resolve_icon<'a>(cfg: &'a Config, bundle_id: &str) -> &'a str {
    cfg.static_icon
        .as_deref()
        .unwrap_or_else(|| icon_for_with_overrides(&cfg.icon_overrides, bundle_id))
}

/// Fire a custom event. Uppercase keys; direct exec, no shell.
/// Idle (`None`) renders the placeholder pill through the same path, so the
/// bar never shows a stale track and never hides.
pub fn trigger(event: &str, track: Option<&Track>, cfg: &Config) -> Result<()> {
    let placeholder;
    let t = match track {
        Some(t) => t,
        None => {
            placeholder = Track::placeholder();
            &placeholder
        }
    };
    let label = t.label(&cfg.separator);
    let icon = resolve_icon(cfg, &t.bundle_id);
    let toggle = toggle_icon(t.playing);
    let status = Command::new("sketchybar")
        .arg("--trigger")
        .arg(event)
        .arg(format!("TITLE={}", t.title))
        .arg(format!("ARTIST={}", t.artist))
        .arg(format!("ALBUM={}", t.album))
        .arg(format!("BUNDLE={}", t.bundle_id))
        .arg(format!("PLAYING={}", t.playing))
        .arg(format!("LABEL={label}"))
        .arg(format!("ICON={icon}"))
        .arg(format!("PREV_ICON={ICON_PREV}"))
        .arg(format!("TOGGLE_ICON={toggle}"))
        .arg(format!("NEXT_ICON={ICON_NEXT}"))
        .status()
        .map_err(|e| Error::SketchyBar(format!("spawn sketchybar --trigger: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::SketchyBar(format!("--trigger exited with {status}")))
    }
}

#[inline]
fn push_set(cmd: &mut Command, item: &str, track: Option<&Track>, cfg: &Config) {
    if let Some(kind) = control_of(item) {
        push_set_control(cmd, item, kind, track);
        return;
    }
    // Idle renders the placeholder through the same path: standard icon,
    // no scroll, always drawn. Paused tracks keep their real entry.
    let placeholder;
    let t = match track {
        Some(t) => t,
        None => {
            placeholder = Track::placeholder();
            &placeholder
        }
    };
    let label = t.label(&cfg.separator);
    let icon = resolve_icon(cfg, &t.bundle_id);
    let scroll = if t.playing {
        "scroll_texts=on"
    } else {
        "scroll_texts=off"
    };
    cmd.arg("--set")
        .arg(item)
        .arg(format!("label={label}"))
        .arg(format!("icon={icon}"))
        .arg(scroll)
        .arg("drawing=on");
}

#[inline]
fn push_set_control(cmd: &mut Command, item: &str, kind: &str, track: Option<&Track>) {
    // Idle parks the toggle on play through the same path; the whole pill
    // stays drawn, so placeholder and controls are always visible together.
    let placeholder;
    let t = match track {
        Some(t) => t,
        None => {
            placeholder = Track::placeholder();
            &placeholder
        }
    };
    cmd.arg("--set").arg(item);
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

#[inline]
fn spawn(mut cmd: Command, what: &str) -> Result<()> {
    let status = cmd
        .status()
        .map_err(|e| Error::SketchyBar(format!("spawn sketchybar {what}: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::SketchyBar(format!("{what} exited with {status}")))
    }
}

pub fn set_with_controls(item: &str, track: Option<&Track>, cfg: &Config) -> Result<()> {
    // One `sketchybar` spawn covers the whole pill: five separate spawns
    // per `sync` tick (every 10s) and per `--set` daemon event cost five
    // forks plus five transient bar IPC round trips.
    let base = CONTROL_SUFFIXES
        .iter()
        .find_map(|suffix| item.strip_suffix(*suffix))
        .unwrap_or(item);
    let mut cmd = Command::new("sketchybar");
    push_set(&mut cmd, base, track, cfg);
    for suffix in CONTROL_SUFFIXES {
        let sibling = format!("{base}{suffix}");
        push_set_control(&mut cmd, &sibling, suffix, track);
    }
    spawn(cmd, "--set")
}

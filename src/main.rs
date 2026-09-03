mod cli;
mod config;
mod error;
mod icons;
mod media;
mod sketchybar;
mod track;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use error::{Error, Result};
use media::{MediaSource, PerlMedia};
use std::{
    process::ExitCode,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::RecvTimeoutError,
        Arc,
    },
    time::Duration,
};
use track::Track;

/// Budget for the adapter's first payload on one-shot paths.
const FIRST_PAYLOAD_TIMEOUT: Duration = Duration::from_secs(2);
/// Wake interval for the event loops. Carries no work and no allocation;
/// it only lets a SIGTERM/SIGINT break the blocking receive promptly so
/// `Drop` runs and the adapter child is reaped.
const SHUTDOWN_POLL: Duration = Duration::from_millis(200);

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("sketchybar-now-playing: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let cfg = Config::load(cli.config.as_deref())?;
    let media = PerlMedia::new();

    // Graceful SIGTERM/SIGINT: breaking the loops below drops `media`,
    // which reaps the adapter child instead of orphaning it.
    // (ctrlc installs infallibly; the flag is the only fallible path.)
    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = Arc::clone(&stop);
        ctrlc::set_handler(move || stop.store(true, Ordering::Relaxed));
    }

    match cli.command {
        Commands::Get { json } => cmd_get(&media, &cfg, json),
        Commands::Stream => cmd_stream(&media, &stop),
        Commands::Daemon { event, set } => cmd_daemon(&media, &cfg, &event, set.as_deref(), &stop),
        Commands::Sync { item } => {
            let track = media.snapshot_wait(FIRST_PAYLOAD_TIMEOUT);
            sketchybar::set(&item, track.as_ref(), &cfg)
        }
        Commands::Play => control(media.play(), "play"),
        Commands::Pause => control(media.pause(), "pause"),
        Commands::Toggle => control(media.toggle(), "toggle"),
        Commands::Next => control(media.next(), "next"),
        Commands::Prev => control(media.previous(), "previous"),
    }
}

/// Control commands report failure instead of silently succeeding: a `false`
/// from the backend means no active player accepted the command.
#[inline]
fn control(ok: bool, action: &str) -> Result<()> {
    ok.then_some(())
        .ok_or_else(|| Error::Media(format!("{action} failed: no active Now Playing client")))
}
/// One-shot output. Idle + `hide_output` prints an empty line so a polling
/// plugin clears the bar; exit status stays 0 (absence of media is valid).
fn cmd_get(media: &impl MediaSource, cfg: &Config, json: bool) -> Result<()> {
    let track = media.snapshot_wait(FIRST_PAYLOAD_TIMEOUT);
    match (track.as_ref(), json) {
        (Some(t), true) => println!("{}", t.to_json_line()?),
        (Some(t), false) => println!("{}", t.label(&cfg.separator)),
        (None, true) => println!("null"),
        (None, false) if cfg.hide_output => println!(),
        (None, false) => println!("No player available"),
    }
    Ok(())
}

/// NDJSON change feed. O(1) diff in the loop; unchanged callbacks are
/// dropped without printing, keeping downstream `jq` pipes quiet.
fn cmd_stream(media: &impl MediaSource, stop: &AtomicBool) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel::<Option<Track>>();
    let _token = media.on_change(move |t| {
        let _ = tx.send(t); // Receiver gone (shutdown) => drop silently.
    });
    let mut last: Option<Track> = None;
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let next = match rx.recv_timeout(SHUTDOWN_POLL) {
            Ok(next) => next,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let emit = match (&last, &next) {
            (None, None) => false,
            (None, Some(_)) | (Some(_), None) => true,
            (Some(a), Some(b)) => a.changed(b),
        };
        last = next;
        if !emit {
            continue;
        }
        match last.as_ref() {
            Some(t) => println!("{}", t.to_json_line()?),
            None => println!("null"),
        }
    }
    Ok(())
}

/// Event loop: notify SketchyBar only on change. Emits once immediately so
/// a (re)started daemon converges the bar without waiting for input.
fn cmd_daemon(
    media: &impl MediaSource,
    cfg: &Config,
    event: &str,
    set_item: Option<&str>,
    stop: &AtomicBool,
) -> Result<()> {
    let notify = |track: Option<&Track>| -> Result<()> {
        match set_item {
            Some(item) => sketchybar::set(item, track, cfg),
            None => sketchybar::trigger(event, track, cfg),
        }
    };
    // Single wait: `notify` borrows, then `last` takes ownership. No clone.
    let initial = media.snapshot_wait(FIRST_PAYLOAD_TIMEOUT);
    notify(initial.as_ref())?;

    let (tx, rx) = std::sync::mpsc::channel::<Option<Track>>();
    let _token = media.on_change(move |t| {
        let _ = tx.send(t);
    });
    let mut last = initial;
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let next = match rx.recv_timeout(SHUTDOWN_POLL) {
            Ok(next) => next,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let changed = match (&last, &next) {
            (None, None) => false,
            (None, Some(_)) | (Some(_), None) => true,
            (Some(a), Some(b)) => a.changed(b),
        };
        if !changed {
            continue;
        }
        notify(next.as_ref())?;
        last = next;
    }
    Ok(())
}

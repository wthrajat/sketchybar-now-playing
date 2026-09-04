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

const FIRST_PAYLOAD_TIMEOUT: Duration = Duration::from_secs(2);
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
            sketchybar::set_with_controls(&item, track.as_ref(), &cfg)
        }
        Commands::Play => control(media.play(), "play"),
        Commands::Pause => control(media.pause(), "pause"),
        Commands::Toggle => control(media.toggle(), "toggle"),
        Commands::Next => control(media.next(), "next"),
        Commands::Prev => control(media.previous(), "previous"),
    }
}

#[inline]
fn control(ok: bool, action: &str) -> Result<()> {
    ok.then_some(())
        .ok_or_else(|| Error::Media(format!("{action} failed: no active Now Playing client")))
}

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

fn cmd_stream(media: &impl MediaSource, stop: &AtomicBool) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel::<Option<Track>>();
    let _token = media.on_change(move |t| {
        let _ = tx.send(t);
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

fn cmd_daemon(
    media: &impl MediaSource,
    cfg: &Config,
    event: &str,
    set_item: Option<&str>,
    stop: &AtomicBool,
) -> Result<()> {
    let notify = |track: Option<&Track>| -> Result<()> {
        match set_item {
            Some(item) => sketchybar::set_with_controls(item, track, cfg),
            None => sketchybar::trigger(event, track, cfg),
        }
    };
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

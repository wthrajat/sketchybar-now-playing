use crate::track::Track;
use media_remote::{Controller, NowPlayingPerl, Subscription};
use std::time::{Duration, Instant};

/// Poll interval while waiting for the adapter's first payload.
/// The perl child needs ~100-500 ms to emit; 50 ms keeps `get` snappy.
const FIRST_PAYLOAD_POLL: Duration = Duration::from_millis(50);

/// Abstraction over the Now Playing backend. Lets `main.rs` stay backend
/// agnostic (Perl adapter today, JXA fallback tomorrow) and stay testable
/// with a fake without touching MediaRemote.
pub trait MediaSource: Send + Sync {
    /// One-shot snapshot. `None` = idle / no titled media.
    fn snapshot(&self) -> Option<Track>;
    /// Same, but waits up to `timeout` for the adapter's first payload.
    /// `new()` spawns the perl child asynchronously, so an immediate
    /// `snapshot()` right after construction racily returns `None` even
    /// while media plays. One-shot commands (`get`, daemon startup) must
    /// use this; the `stream`/`on_change` path is already event-driven.
    fn snapshot_wait(&self, timeout: Duration) -> Option<Track> {
        let deadline = Instant::now() + timeout;
        loop {
            let track = self.snapshot();
            if track.is_some() || Instant::now() >= deadline {
                return track;
            }
            std::thread::sleep(FIRST_PAYLOAD_POLL);
        }
    }
    /// Subscribe to change notifications. The token must be held by the
    /// caller; dropping it unsubscribes. Callback payloads are `Send`.
    fn on_change<F>(&self, listener: F) -> media_remote::ListenerToken
    where
        F: Fn(Option<Track>) + Send + Sync + 'static;
    fn play(&self) -> bool;
    fn pause(&self) -> bool;
    fn toggle(&self) -> bool;
    fn next(&self) -> bool;
    fn previous(&self) -> bool;
}

/// Tahoe-safe backend: talks to `mediaremoted` through the system `perl`
/// loader (see `ungive/mediaremote-adapter`), so the 15.4+ entitlement
/// gate does not apply. Spawns one background process at construction.
pub struct PerlMedia {
    inner: NowPlayingPerl,
}

impl PerlMedia {
    pub fn new() -> Self {
        Self {
            inner: NowPlayingPerl::new(),
        }
    }
}

impl Drop for PerlMedia {
    /// Reap the adapter child. The crate's own `Drop` only flips its flag
    /// while the reader thread blocks on the child's stdout, so without
    /// this every short lived command (`get`, controls) would orphan one
    /// `/usr/bin/perl` per run.
    fn drop(&mut self) {
        reap_adapter_children();
    }
}

/// Kill leftover adapter helpers that are our own direct children.
/// The PPID check keeps siblings and other apps untouched. Absolute tool
/// paths so this also works under SketchyBar's minimal PATH. Best effort:
/// cleanup must never fail shutdown.
fn reap_adapter_children() {
    let output = std::process::Command::new("/usr/bin/pgrep")
        .args(["-f", "mediaremote-adapter\\.pl .* stream"])
        .output();
    let output = match output {
        Ok(output) => output,
        Err(_) => return,
    };
    let me = std::process::id().to_string();
    let text = String::from_utf8_lossy(&output.stdout);
    for pid in text.split_whitespace() {
        let ppid = std::process::Command::new("/bin/ps")
            .args(["-o", "ppid=", "-p", pid])
            .output();
        let is_ours = match ppid {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == me,
            Err(_) => false,
        };
        if is_ours {
            let _ = std::process::Command::new("/bin/kill").args([pid]).output();
        }
    }
}

/// Convert the guard's borrowed info into an owned `Track`, then drop the
/// guard before doing anything else. This keeps the critical section short.
#[inline]
fn track_from_guard(info: Option<&media_remote::NowPlayingInfo>) -> Option<Track> {
    let info = info?;
    let playing = info
        .is_playing
        .or_else(|| info.playback_rate.map(|r| r > 0.0))
        .unwrap_or(false);
    Track::from_parts(
        info.title.as_deref(),
        info.artist.as_deref(),
        info.album.as_deref(),
        info.bundle_id.as_deref(),
        playing,
    )
}

impl MediaSource for PerlMedia {
    fn snapshot(&self) -> Option<Track> {
        let guard = self.inner.get_info();
        track_from_guard(guard.as_ref())
        // Guard drops here, before the caller formats / prints.
    }

    fn on_change<F>(&self, listener: F) -> media_remote::ListenerToken
    where
        F: Fn(Option<Track>) + Send + Sync + 'static,
    {
        self.inner.subscribe(move |guard| {
            // Clone the minimal snapshot inside the callback, release the
            // lock immediately, then invoke user code lock-free.
            let track = track_from_guard(guard.as_ref());
            listener(track);
        })
    }

    #[inline]
    fn play(&self) -> bool {
        self.inner.play()
    }

    #[inline]
    fn pause(&self) -> bool {
        self.inner.pause()
    }

    #[inline]
    fn toggle(&self) -> bool {
        self.inner.toggle()
    }

    #[inline]
    fn next(&self) -> bool {
        self.inner.next()
    }

    #[inline]
    fn previous(&self) -> bool {
        self.inner.previous()
    }
}

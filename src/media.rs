use crate::track::Track;
use media_remote::{Controller, NowPlayingPerl, Subscription};
use std::time::{Duration, Instant};

const FIRST_PAYLOAD_POLL: Duration = Duration::from_millis(50);

pub trait MediaSource: Send + Sync {
    fn snapshot(&self) -> Option<Track>;
    /// First payload arrives async; immediate reads race startup.
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
    fn on_change<F>(&self, listener: F) -> media_remote::ListenerToken
    where
        F: Fn(Option<Track>) + Send + Sync + 'static;
    fn play(&self) -> bool;
    fn pause(&self) -> bool;
    fn toggle(&self) -> bool;
    fn next(&self) -> bool;
    fn previous(&self) -> bool;
}

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
    /// Reap the adapter child; the crate's Drop can't (blocked reader).
    fn drop(&mut self) {
        reap_adapter_children();
    }
}

/// Kill our own adapter children only (PPID check). Best effort.
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
    }

    fn on_change<F>(&self, listener: F) -> media_remote::ListenerToken
    where
        F: Fn(Option<Track>) + Send + Sync + 'static,
    {
        self.inner.subscribe(move |guard| {
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

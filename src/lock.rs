//! Singleton lock for `daemon` feeds (std only, no new deps).
//!
//! One lock file per feed (`--event` name or `--set` target) in `/tmp`,
//! namespaced by uid. `/tmp` (not `$TMPDIR`) is deliberate: launchd agents
//! and login shells see different `$TMPDIR`s, so a `$TMPDIR` lock would let
//! one daemon per context stack up. A second daemon for the same feed exits
//! immediately instead of doubling steady-state RSS (~33 MB: daemon plus
//! perl helper) and firing every bar trigger twice. Stale locks from dead
//! owners are taken over; the guard removes the file on drop. Short-lived
//! commands (`get`, `sync`, controls) never touch the lock.

use crate::error::{Error, Result};
use std::{io::Write, path::PathBuf};

/// Held for the daemon's lifetime; removes the lock file on drop.
pub struct DaemonLock {
    path: PathBuf,
    pid: u32,
}

/// `Some(guard)` when we own the feed, `None` when a live daemon already
/// does (caller should exit cleanly). The key is the `--set` target when
/// present, else the `--event` name, so distinct feeds stay independent.
pub fn acquire_daemon(event: &str, set: Option<&str>) -> Result<Option<DaemonLock>> {
    let path = lock_path(event, set);
    let me = std::process::id();

    // Fast path: atomic create wins the feed.
    match create_with_pid(&path, me) {
        Ok(()) => {
            return Ok(Some(DaemonLock { path, pid: me }));
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(Error::from(e)),
    }

    // Slow path: lock exists. Duplicate only if the owner is alive and is
    // actually our binary (guards against PID reuse); anything else is a
    // stale lock we take over.
    if owner_is_live_daemon(&path, me) {
        return Ok(None);
    }
    let _ = std::fs::remove_file(&path);
    match create_with_pid(&path, me) {
        Ok(()) => Ok(Some(DaemonLock { path, pid: me })),
        // Lost a startup race to a live newcomer; exiting is the safe side.
        Err(_) => Ok(None),
    }
}

fn lock_path(event: &str, set: Option<&str>) -> PathBuf {
    let mut key = String::with_capacity(32);
    match set {
        Some(item) => {
            key.push_str("set-");
            key.push_str(item);
        }
        None => {
            key.push_str("event-");
            key.push_str(event);
        }
    }
    let safe: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    PathBuf::from(format!("/tmp/sketchybar-now-playing-{}.{safe}.lock", uid()))
}

fn create_with_pid(path: &PathBuf, pid: u32) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(pid.to_string().as_bytes())?;
    Ok(())
}

/// Numeric uid for lock namespacing (one spawn, daemon startup only).
/// Falls back to `unknown` rather than failing startup.
fn uid() -> String {
    std::process::Command::new("/usr/bin/id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn owner_pid(path: &PathBuf) -> Option<u32> {
    std::fs::read_to_string(path)
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()
}

fn owner_is_live_daemon(path: &PathBuf, me: u32) -> bool {
    let owner = match owner_pid(path) {
        Some(pid) => pid,
        None => return false,
    };
    // Already held in this process: a second acquire is a duplicate too.
    if owner == me {
        return true;
    }
    pid_is_ours(&owner.to_string())
}

/// Alive (`kill -0`) and running our binary (full command line contains the
/// binary name, same match the shell autostart guard uses).
fn pid_is_ours(pid: &str) -> bool {
    let alive = std::process::Command::new("/bin/kill")
        .args(["-0", pid])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    if !alive {
        return false;
    }
    std::process::Command::new("/bin/ps")
        .args(["-o", "command=", "-p", pid])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .is_some_and(|cmd| cmd.contains("sketchybar-now-playing"))
}

impl Drop for DaemonLock {
    fn drop(&mut self) {
        // Remove only our own file, so a successor's lock is never deleted.
        let ours = owner_pid(&self.path).is_some_and(|pid| pid == self.pid);
        if ours {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_key(tag: &str) -> String {
        format!("test-{tag}-{}", std::process::id())
    }

    fn lock_path_for(event: &str) -> PathBuf {
        lock_path(event, None)
    }

    #[test]
    fn second_acquire_loses_while_first_is_held() {
        let key = unique_key("cap");
        let first = acquire_daemon(&key, None).expect("first acquires");
        assert!(first.is_some());
        let second = acquire_daemon(&key, None).expect("second reports duplicate");
        assert!(second.is_none());
        drop(first);
        let _ = std::fs::remove_file(lock_path_for(&key));
    }

    #[test]
    fn lock_returns_after_guard_drops() {
        let key = unique_key("drop");
        {
            let _held = acquire_daemon(&key, None).expect("acquires");
        }
        let again = acquire_daemon(&key, None).expect("re-acquires after drop");
        assert!(again.is_some());
        drop(again);
        let _ = std::fs::remove_file(lock_path_for(&key));
    }

    #[test]
    fn stale_lock_is_taken_over() {
        let key = unique_key("stale");
        let path = lock_path_for(&key);
        std::fs::write(&path, "42424242").expect("plant stale pid");
        let guard = acquire_daemon(&key, None).expect("takes over stale lock");
        assert!(guard.is_some());
        drop(guard);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn set_feeds_lock_independently_from_event_feeds() {
        let key = unique_key("kinds");
        let by_set = acquire_daemon("ignored", Some(&key)).expect("set feed acquires");
        assert!(by_set.is_some());
        drop(by_set);
        let _ = std::fs::remove_file(lock_path(&key, Some(&key)));
    }
}

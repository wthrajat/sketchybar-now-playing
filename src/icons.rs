use std::collections::HashMap;

/// Nerd Font glyphs used as per player icons.
/// Unicode escapes keep the source encoding-proof.
pub const ICON_DEFAULT: &str = "\u{f001}"; //  music note
pub const ICON_SPOTIFY: &str = "\u{f1bc}"; //  spotify
pub const ICON_BROWSER: &str = "\u{f269}"; //  browser
pub const ICON_MUSIC: &str = "\u{f001}"; //  music note

/// Transport control glyphs (previous, play, pause, next).
/// Nerd Font glyphs, Unicode escaped to keep the source encoding-proof.
pub const ICON_PREV: &str = "\u{f048}"; //  previous
pub const ICON_PLAY: &str = "\u{f04b}"; //  play
pub const ICON_PAUSE: &str = "\u{f04c}"; //  pause
pub const ICON_NEXT: &str = "\u{f051}"; //  next

/// Play/pause glyph for the toggle button. Shows pause while playing,
/// play while paused.
#[inline]
pub fn toggle_icon(playing: bool) -> &'static str {
    if playing {
        ICON_PAUSE
    } else {
        ICON_PLAY
    }
}

/// Map a client bundle id to its bar glyph. Zero alloc: returns `&'static`.
/// Browsers share one glyph. A video title and a music track both arrive
/// via the browser bundle id when played in a browser tab.
#[inline]
pub fn icon_for(bundle_id: &str) -> &'static str {
    match bundle_id {
        "com.spotify.client" => ICON_SPOTIFY,
        "com.apple.Music" | "com.apple.iTunes" => ICON_MUSIC,
        _ if is_browser(bundle_id) => ICON_BROWSER,
        _ => ICON_DEFAULT,
    }
}

/// User overrides win; falls back to [`icon_for`]. Borrowed return keeps the
/// daemon hot loop allocation-free.
#[inline]
pub fn icon_for_with_overrides<'a>(
    overrides: &'a HashMap<String, String>,
    bundle_id: &str,
) -> &'a str {
    if let Some(custom) = overrides.get(bundle_id) {
        return custom.as_str();
    }
    // Substring pass: `Foo.browser.SpotifyWeb` style helpers still match.
    for (key, glyph) in overrides.iter() {
        if !key.is_empty() && bundle_id.contains(key.as_str()) {
            return glyph.as_str();
        }
    }
    icon_for(bundle_id)
}

/// O(1) lookup over a tiny constant set, called only on track change
/// (never per tick: the daemon diffs first, so this never spins).
#[inline]
fn is_browser(bundle_id: &str) -> bool {
    const BROWSERS: [&str; 8] = [
        "com.apple.Safari",
        "com.google.Chrome",
        "org.mozilla.firefox",
        "company.thebrowser.Browser", // Arc
        "com.brave.Browser",
        "com.microsoft.edgemac",
        "com.mighty.app", // Mighty / Orion variants report under app ids
        "org.chromium.Chromium",
    ];
    BROWSERS.contains(&bundle_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_players_map() {
        assert_eq!(icon_for("com.spotify.client"), ICON_SPOTIFY);
        assert_eq!(icon_for("com.apple.Safari"), ICON_BROWSER);
        assert_eq!(icon_for("com.apple.Music"), ICON_MUSIC);
        assert_eq!(icon_for("com.unknown.app"), ICON_DEFAULT);
    }

    #[test]
    fn toggle_shows_pause_while_playing() {
        assert_eq!(toggle_icon(true), ICON_PAUSE);
        assert_eq!(toggle_icon(false), ICON_PLAY);
    }
}

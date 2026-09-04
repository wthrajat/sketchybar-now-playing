use std::collections::HashMap;

pub const ICON_DEFAULT: &str = "\u{f001}"; //  music note
pub const ICON_SPOTIFY: &str = "\u{f1bc}"; //  spotify
pub const ICON_BROWSER: &str = "\u{f269}"; //  browser
pub const ICON_MUSIC: &str = "\u{f001}"; //  music note
pub const ICON_PREV: &str = "\u{f048}"; //  previous
pub const ICON_PLAY: &str = "\u{f04b}"; //  play
pub const ICON_PAUSE: &str = "\u{f04c}"; //  pause
pub const ICON_NEXT: &str = "\u{f051}"; //  next

#[inline]
pub fn toggle_icon(playing: bool) -> &'static str {
    if playing {
        ICON_PAUSE
    } else {
        ICON_PLAY
    }
}

#[inline]
pub fn icon_for(bundle_id: &str) -> &'static str {
    match bundle_id {
        "com.spotify.client" => ICON_SPOTIFY,
        "com.apple.Music" | "com.apple.iTunes" => ICON_MUSIC,
        _ if is_browser(bundle_id) => ICON_BROWSER,
        _ => ICON_DEFAULT,
    }
}

#[inline]
pub fn icon_for_with_overrides<'a>(
    overrides: &'a HashMap<String, String>,
    bundle_id: &str,
) -> &'a str {
    if let Some(custom) = overrides.get(bundle_id) {
        return custom.as_str();
    }
    for (key, glyph) in overrides.iter() {
        if !key.is_empty() && bundle_id.contains(key.as_str()) {
            return glyph.as_str();
        }
    }
    icon_for(bundle_id)
}

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

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Track {
    pub title: String,
    pub artist: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub album: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bundle_id: String,
    pub playing: bool,
}

impl Track {
    /// None when title is missing.
    pub fn from_parts(
        title: Option<&str>,
        artist: Option<&str>,
        album: Option<&str>,
        bundle_id: Option<&str>,
        playing: bool,
    ) -> Option<Self> {
        let title = title.map(str::trim).filter(|t| !t.is_empty())?;
        Some(Self {
            title: title.to_owned(),
            artist: artist.map(str::trim).unwrap_or("").to_owned(),
            album: album.map(str::trim).unwrap_or("").to_owned(),
            bundle_id: bundle_id.unwrap_or("").to_owned(),
            playing,
        })
    }

    #[inline]
    pub fn changed(&self, other: &Self) -> bool {
        self != other
    }

    #[inline]
    pub fn label(&self, sep: &str) -> String {
        if self.artist.is_empty() {
            return self.title.clone();
        }
        let mut out = String::with_capacity(self.title.len() + sep.len() + self.artist.len());
        out.push_str(&self.title);
        out.push_str(sep);
        out.push_str(&self.artist);
        out
    }

    pub fn to_json_line(&self) -> crate::error::Result<String> {
        serde_json::to_string(self).map_err(crate::error::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_joins_title_and_artist_once() {
        let t = Track::from_parts(Some("Song"), Some("Band"), None, None, true).expect("track");
        assert_eq!(t.label(" - "), "Song - Band");
    }

    #[test]
    fn label_without_artist_is_bare_title() {
        let t = Track::from_parts(Some("Trailer"), Some("  "), None, None, true).expect("track");
        assert_eq!(t.label(" - "), "Trailer");
    }

    #[test]
    fn title_is_mandatory() {
        assert!(Track::from_parts(None, Some("Band"), None, None, true).is_none());
        assert!(Track::from_parts(Some("   "), Some("Band"), None, None, true).is_none());
    }
}

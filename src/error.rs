use std::fmt;

#[derive(Debug)]
pub enum Error {
    Config(String),
    Media(String),
    SketchyBar(String),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(msg) | Self::Media(msg) | Self::SketchyBar(msg) => f.write_str(msg),
            Self::Io(err) => write!(f, "io: {err}"),
            Self::Json(err) => write!(f, "json: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Json(err) => Some(err),
            Self::Config(_) | Self::Media(_) | Self::SketchyBar(_) => None,
        }
    }
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    #[inline]
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

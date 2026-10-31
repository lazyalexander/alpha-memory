use std::path::PathBuf;

/// Failures a caller can correct by changing arguments or session state.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("workspace does not exist: {0}")]
    MissingWorkspace(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn message(text: impl Into<String>) -> Error {
    Error::Message(text.into())
}

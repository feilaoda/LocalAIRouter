use thiserror::Error;

pub type Result<T> = std::result::Result<T, LocalAIRouterError>;

#[derive(Debug, Error)]
pub enum LocalAIRouterError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("vault is locked")]
    Locked,
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("database error: {0}")]
    Sqlite(String),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("http error: {0}")]
    Http(String),
    #[error("{0}")]
    Message(String),
}

impl From<std::io::Error> for LocalAIRouterError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<serde_json::Error> for LocalAIRouterError {
    fn from(value: serde_json::Error) -> Self {
        Self::Message(value.to_string())
    }
}

impl From<std::num::ParseIntError> for LocalAIRouterError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::Validation(value.to_string())
    }
}

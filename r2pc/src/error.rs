#[derive(thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum Error {
    #[error("serde error: {0}")]
    SerdeError(String),
    #[error("socket error: {0}")]
    SocketError(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("invalid msg: {0}")]
    InvalidMsg(String),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl From<rmp_serde::decode::Error> for Error {
    fn from(value: rmp_serde::decode::Error) -> Self {
        Error::SerdeError(value.to_string())
    }
}

impl From<rmpv::ext::Error> for Error {
    fn from(value: rmpv::ext::Error) -> Self {
        Error::SerdeError(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

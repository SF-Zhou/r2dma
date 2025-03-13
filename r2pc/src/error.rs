#[derive(thiserror::Error, derse::Serialize, derse::Deserialize)]
pub enum Error {
    #[error("serialization error: {0}")]
    DerseError(#[from] derse::Error),
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

pub type Result<T> = std::result::Result<T, Error>;

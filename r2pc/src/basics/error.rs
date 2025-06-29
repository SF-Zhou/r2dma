use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Timeout,
    InvalidArgument,
    SerializeFailed,
    DeserializeFailed,
    TcpConnectFailed,
    TcpAddSocketFailed,
    TcpSendMsgFailed,
    TcpParseMsgFailed,
    TcpSendFailed,
    TcpRecvFailed,
    WaitMsgFailed,
    #[cfg(feature = "rdma")]
    RdmaError(r2dma::ErrorKind),
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub msg: Option<String>,
}

impl Error {
    pub fn new(kind: ErrorKind, msg: String) -> Self {
        Self {
            kind,
            msg: Some(msg),
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self { kind, msg: None }
    }
}

#[cfg(feature = "rdma")]
impl From<r2dma::Error> for Error {
    fn from(e: r2dma::Error) -> Self {
        Self {
            kind: ErrorKind::RdmaError(e.kind),
            msg: e.msg,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.msg {
            Some(msg) => write!(f, "{:?}: {}", self.kind, msg),
            None => write!(f, "{:?}", self.kind),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_kind() {
        let kind = ErrorKind::Timeout;
        let error: Error = kind.into();
        println!("{:?}", error);
    }
}

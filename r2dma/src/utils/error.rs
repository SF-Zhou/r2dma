#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    IOError,
    IBGetDeviceListFail,
    IBDeviceNotFound,
    IBOpenDeviceFail,
    IBQueryGidFail,
    IBQueryPortFail,
    IBCreateCompChannelFail,
    IBCreateCQFail,
    IBReqNotifyCQFail,
    IBPollCQFail,
    IBAllocPDFail,
    IBCreateQPFail,
    IBModifyQPFail,
    IBGetCQEventFail,
    IBRegMRFail,
    AllocateBufferFail,
    PollCompChannelFailed,
    SetNonBlockFail,
}

#[derive(Clone)]
pub struct Error(ErrorKind, Option<String>);

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self(kind, None)
    }

    pub fn with_errno(kind: ErrorKind) -> Self {
        Self(
            kind,
            Some(format!("errno: {}", std::io::Error::last_os_error())),
        )
    }

    pub fn with_msg(kind: ErrorKind, msg: String) -> Self {
        Self(kind, Some(msg))
    }
}

impl From<nix::Error> for Error {
    fn from(error: nix::Error) -> Self {
        Self(ErrorKind::IOError, Some(format!("errno: {}", error)))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.1 {
            f.debug_struct("Error")
                .field("kind", &self.0)
                .field("msg", msg)
                .finish()
        } else {
            f.debug_struct("Error").field("kind", &self.0).finish()
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error() {
        println!("{:?}", Error::new(ErrorKind::IBGetDeviceListFail));
        println!("{:?}", Error::with_errno(ErrorKind::IBDeviceNotFound));
        println!(
            "{:?}",
            Error::with_msg(ErrorKind::IBGetDeviceListFail, "not found!".into())
        );
        println!("{:?}", Error::from(nix::Error::EAGAIN));
    }
}

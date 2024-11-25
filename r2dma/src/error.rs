#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("ib get deivce list fail")]
    IBGetDeviceListFail,
    #[error("ib device is not found")]
    IBDeviceNotFound,
    #[error("ib open device fail")]
    IBOpenDeviceFail,
    #[error("ib query device fail")]
    IBQueryGidFail,
    #[error("ib query port fail")]
    IBQueryPortFail,
    #[error("ib allocate protection domain fail")]
    IBAllocPDFail,
}

pub type Result<T> = std::result::Result<T, Error>;

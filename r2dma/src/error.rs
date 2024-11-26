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
    #[error("ib create completion channel fail")]
    IBCreateCompChannelFail,
    #[error("ib get cq event fail")]
    IBGetCQEventFail,
    #[error("ib create CQ fail")]
    IBCreateCQFail,
    #[error("ib req notify CQ fail")]
    IBReqNotifyCQFail,
    #[error("ib poll CQ fail")]
    IBPollCQFail,
    #[error("ib register memory region fail")]
    IBRegMRFail,
    #[error("ib create queue pair fail")]
    IBCreateQPFail,
    #[error("ib modify queue pair fail")]
    IBModifyQPFail,
    #[error("set fd non-block fail")]
    SetNonBlockFail,
}

pub type Result<T> = std::result::Result<T, Error>;

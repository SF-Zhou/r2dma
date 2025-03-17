#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("alloc memory failed")]
    AllocMemoryFailed,
    #[error("ib get deivce list fail: {0}")]
    IBGetDeviceListFail(#[source] std::io::Error),
    #[error("ib device is not found")]
    IBDeviceNotFound,
    #[error("ib open device fail: {0}")]
    IBOpenDeviceFail(#[source] std::io::Error),
    #[error("ib query device fail: {0}")]
    IBQueryDeviceFail(#[source] std::io::Error),
    #[error("ib query gid fail: {0}")]
    IBQueryGidFail(#[source] std::io::Error),
    #[error("ib query gid type fail: {0}")]
    IBQueryGidTypeFail(#[source] std::io::Error),
    #[error("ib query port fail: {0}")]
    IBQueryPortFail(#[source] std::io::Error),
    #[error("ib allocate protection domain fail: {0}")]
    IBAllocPDFail(#[source] std::io::Error),
    #[error("ib create completion channel fail: {0}")]
    IBCreateCompChannelFail(#[source] std::io::Error),
    #[error("ib set completion channel non-block fail: {0}")]
    IBSetCompChannelNonBlockFail(#[source] std::io::Error),
    #[error("ib get comp queue event fail: {0}")]
    IBGetCompQueueEventFail(#[source] std::io::Error),
    #[error("ib create comp queue fail: {0}")]
    IBCreateCompQueueFail(#[source] std::io::Error),
    #[error("ib req notify comp queue fail: {0}")]
    IBReqNotifyCompQueueFail(#[source] std::io::Error),
    #[error("ib poll comp queue fail: {0}")]
    IBPollCompQueueFail(#[source] std::io::Error),
    #[error("ib register memory region fail: {0}")]
    IBRegMemoryRegionFail(#[source] std::io::Error),
    #[error("ib create queue pair fail: {0}")]
    IBCreateQueuePairFail(#[source] std::io::Error),
    #[error("ib modify queue pair fail: {0}")]
    IBModifyQueuePairFail(#[source] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

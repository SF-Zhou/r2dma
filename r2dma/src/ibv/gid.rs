#[derive(Debug, PartialEq, Eq, Hash)]
pub enum GidType {
    IB,
    RoCEv1,
    RoCEv2,
    Other(String),
}

use super::verbs::ibv_gid;
use derse::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Endpoint {
    pub qp_num: u32,
    pub lid: u16,
    pub gid: ibv_gid,
}

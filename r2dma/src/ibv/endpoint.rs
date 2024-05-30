use super::Gid;

#[derive(Debug, derse::Derse)]
pub struct Endpoint {
    pub qp_num: u32,
    pub lid: u16,
    pub gid: Gid,
}

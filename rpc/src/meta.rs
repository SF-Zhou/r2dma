#[derive(derse::Serialize, derse::Deserialize, Clone)]
pub struct Meta {
    pub msg_id: u64,
    pub method: String,
    pub flags: u32,
}

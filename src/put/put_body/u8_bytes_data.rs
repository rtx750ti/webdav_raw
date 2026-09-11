pub type BytesDataId = Option<String>;

pub struct U8BytesData {
    data: Vec<u8>,
    length: u64,
    id: BytesDataId, // 给个id好搜索
}

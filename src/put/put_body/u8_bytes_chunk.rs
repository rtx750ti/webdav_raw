use crate::put::put_body::u8_bytes_data::U8BytesData;

pub struct U8BytesChunk {
    data: U8BytesData,
    create_time: Option<String>,
    range_start: Option<u64>,
    range_end: Option<u64>,
    id: Option<String>,
}

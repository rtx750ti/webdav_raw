use crate::put::put_body::u8_bytes_data::U8BytesData;

pub struct U8Metadata {
    name: String,
    last_modified: Option<String>,
    create_time: Option<String>,
    update_time: Option<String>,
    content_type: String,
    etag: Option<String>,
}

pub struct U8Bytes {
    data: U8BytesData,
    metadata: U8Metadata,
    id: Option<String>,
}

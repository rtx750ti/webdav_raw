pub mod file_handle;
pub mod u8_bytes;
pub mod u8_bytes_chunk;
pub mod u8_bytes_data;

use crate::put::put_body::{
    file_handle::FileHandle, u8_bytes::U8Bytes, u8_bytes_chunk::U8BytesChunk,
};

pub enum PutData {
    /// 文件句柄
    File(FileHandle),
    /// 单文件二进制
    U8Bytes(U8Bytes),
    /// 二进制分片，用于处理单个文件分片
    U8BytesChunk(U8BytesChunk),
}

/// 上传文件请求体
///
/// 一次请求只处理一件事情，本库当中仅做基础单元操作，不提供高级功能
pub struct PutBody {
    data: PutData,
}

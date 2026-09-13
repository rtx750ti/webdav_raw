//! 一次 PUT 的请求体数据模型。
//!
//! 数据来源只有三种：异步文件句柄、完整内存二进制、单个业务分片。
//! 不接任意 `AsyncRead`，理由见 [`file_handle`]。
//!
//! 各类型的字段全部公开，调用方可以直接构造和改写；同时提供少量构造方法用于
//! 带默认推断或带校验的常见场景。

pub mod file_handle;
pub mod u8_bytes;
pub mod u8_bytes_chunk;
pub mod u8_bytes_data;

use crate::put::put_body::{
    file_handle::FileHandle, u8_bytes::U8Bytes, u8_bytes_chunk::U8BytesChunk,
};

/// 一次 PUT 的数据源，三选一。
///
/// 三者的区别只在"数据从哪来"和"带不带位置信息"：
///
/// - [`U8Bytes`]：完整内存二进制，长度由字节算出。
/// - [`U8BytesChunk`]：单个业务分片，在完整二进制之外附带范围。
/// - [`FileHandle`]：异步文件句柄，长度在构建请求时读取。
#[derive(Debug)]
pub enum PutData {
    /// 异步文件句柄。
    File(FileHandle),
    /// 完整内存二进制。
    U8Bytes(U8Bytes),
    /// 单个业务分片，用于处理一个文件的某一段。
    U8BytesChunk(U8BytesChunk),
}

impl PutData {}

/// 一次 PUT 的请求体，只承载一个数据源。
///
/// 本库不维护分片集合，也不持有上传任务状态：分几片、按什么顺序发、失败后
/// 怎么重试，全部由调用方决定。
#[derive(Debug)]
pub struct PutBody {
    pub data: PutData,
}

impl PutBody {
    /// 使用任意一种数据源创建请求体。
    pub fn new(data: PutData) -> Self {
        Self { data }
    }

    /// 使用异步文件句柄创建请求体。
    pub fn from_file(file: FileHandle) -> Self {
        Self::new(PutData::File(file))
    }

    /// 使用完整内存二进制创建请求体。
    pub fn from_bytes(bytes: U8Bytes) -> Self {
        Self::new(PutData::U8Bytes(bytes))
    }

    /// 使用单个业务分片创建请求体。
    pub fn from_chunk(chunk: U8BytesChunk) -> Self {
        Self::new(PutData::U8BytesChunk(chunk))
    }
}

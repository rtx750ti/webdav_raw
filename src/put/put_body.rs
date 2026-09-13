//! 一次 PUT 的请求体数据模型。
//!
//! 数据来源只有三种：异步文件句柄、完整内存二进制、单个业务分片。
//! 不接任意 `AsyncRead`，理由见 [`file_handle`]。
//!
//! # 公开、可组合、可检查
//!
//! 本模块没有私有字段，也没有"只能通过方法访问"的信息：
//!
//! - **可组合**：每个类型都有公开构造方法，也能用结构体字面量直接拼装。
//! - **可检查**：字段全部 `pub`，调用方直接读字段即可，本库不再另外铺一层
//!   只返回相同值的 getter。需要读字节内容这种「不该随手复制」的信息，
//!   才由 [`U8BytesData`](u8_bytes_data::U8BytesData) 单独控制。
//!
//! 构造方法只负责"常见场景的默认值"和"构造点校验"；字段公开意味着调用方
//! 也可以在构造之后改写，此时本库不再复查。

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

/// 一次 PUT 的请求体，只承载一个数据源。
///
/// 本库不维护分片集合，也不持有上传任务状态：分几片、按什么顺序发、失败后
/// 怎么重试，全部由调用方决定。
#[derive(Debug)]
pub struct PutBody {
    /// 本次请求唯一的数据源。
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

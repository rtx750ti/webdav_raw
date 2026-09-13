//! PUT 领域复用的测试夹具与构造工具。
//!
//! 按 `docs/测试/测试规范.md`：同一领域反复使用的构造放入该领域的 `support`，
//! `tests/common` 只保留通用 HTTP 能力。本模块不引用 PROPFIND 固件。
//!
//! 本领域不依赖仓库自带的二进制夹具：文件源测试在系统临时目录现场造文件。

use std::path::{Path, PathBuf};

use webdav_core::{PutBody, U8Bytes, U8BytesData, U8Metadata};

/// 创建带指定内容类型的元数据。
///
/// 这里用结构体字面量直接拼装，因此传入非法内容类型不会被拦下：`U8Metadata`
/// 的字段是公开的，校验只发生在 `from_name` / `validate`。
pub fn metadata(content_type: &str) -> U8Metadata {
    U8Metadata {
        name: "test.bin".to_owned(),
        content_type: content_type.to_owned(),
        ..U8Metadata::default()
    }
}

/// 创建带默认内容类型的元数据。
pub fn default_metadata() -> U8Metadata {
    metadata("application/octet-stream")
}

/// 构造一个完整内存二进制请求体，长度由字节算出。
pub fn bytes_body(data: impl Into<Vec<u8>>) -> PutBody {
    let data = U8BytesData::new(data.into(), None).expect("测试数据必须能构造成功");

    PutBody::from_bytes(U8Bytes::new(data, default_metadata()))
}

/// 构造一个带指定内容类型的内存二进制请求体。
pub fn bytes_body_with_content_type(data: impl Into<Vec<u8>>, content_type: &str) -> PutBody {
    let data = U8BytesData::new(data.into(), None).expect("测试数据必须能构造成功");

    PutBody::from_bytes(U8Bytes::new(data, metadata(content_type)))
}

/// 在系统临时目录写入测试文件。
///
/// 文件名带上 `tag` 与进程号，保证并行执行的测试之间互不冲突。
pub async fn write_temp_file(tag: &str, content: &[u8]) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("webdav_core_put_{}_{tag}.bin", std::process::id()));

    tokio::fs::write(&path, content)
        .await
        .expect("临时测试文件必须可写入");

    path
}

/// 删除 [`write_temp_file`] 创建的临时文件。
pub async fn remove_temp_file(path: &Path) {
    let _ = tokio::fs::remove_file(path).await;
}

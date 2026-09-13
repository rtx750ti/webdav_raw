//! 只在本机构建 PUT 请求，不连接任何服务端，也不需要账号。
//!
//! 运行方式：
//!
//! ```text
//! cargo run --example put_local
//! ```
//!
//! 三种数据源各构建一次请求，打印请求方法、目标地址、本库负责的请求头和字节
//! 长度。这里只调用 `build()`，不调用 `send()`：示例不发送任何请求，也不修改
//! 任何远端资源，因此可以直接跑。
//!
//! 示例用的是不带认证头的普通 `Client`，打印请求时不会带出任何凭据。

use std::env;

use webdav_core::{
    Client, FileHandle, PutBody, PutBuilder, Request, U8Bytes, U8BytesChunk, U8BytesData,
    U8Metadata, Url,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 认证对象会在这里注入 Authorization 并复用配置好的 Client；示例只演示
    // 请求构建，所以用一个普通 Client 指向示例地址。
    let base_url = Url::parse("https://example.com/dav/")?;
    let builder = || PutBuilder::new(Client::new(), base_url.clone());

    // ---------- 1. 完整内存二进制 ----------
    // 长度由 U8BytesData 从字节算出；内容类型交给文件名推断。
    let data = U8BytesData::new(b"report-body".to_vec(), Some("op-1".to_owned()))?;
    let metadata = U8Metadata::from_name("report.txt".to_owned())?;
    let request = builder()
        .relative_path("报告/report.txt")?
        .body(PutBody::from_bytes(U8Bytes::new(data, metadata)))
        .build()
        .await?;
    print_request("1. 完整内存二进制", &request);

    // ---------- 2. 单个业务分片 ----------
    // 范围是调用方侧的本地信息，本库默认不写成 Content-Range；需要发送时由
    // 调用方显式设置 header("content-range", &chunk.to_content_range()?)。
    let data = U8BytesData::new(vec![0xAB; 4], None)?;
    let chunk = U8BytesChunk::new(data, Some(0), Some(3), Some(8), None)?;
    let request = builder()
        .relative_path("chunk.bin")?
        .body(PutBody::from_chunk(chunk))
        .build()
        .await?;
    print_request("2. 单个业务分片", &request);

    // ---------- 3. 异步文件句柄 ----------
    // 句柄必须处于文件起始位置，长度由本库在 build() 时读取；字节在发送阶段
    // 才流式读出，不会整体进入内存。
    let path = env::temp_dir().join("webdav_core_put_example.bin");
    tokio::fs::write(&path, b"file-body").await?;
    let file = tokio::fs::File::open(&path).await?;
    let request = builder()
        .relative_path("archive/upload.bin")?
        .header("x-note", "from put_local example")?
        .body(PutBody::from_file(FileHandle::new(file, None)))
        .build()
        .await?;
    print_request("3. 异步文件句柄", &request);
    tokio::fs::remove_file(&path).await?;

    Ok(())
}

/// 打印一个已经构建好的请求：方法、地址、请求头和请求体形态。
fn print_request(title: &str, request: &Request) {
    println!("=== {title} ===");
    println!("方法: {}", request.method());
    println!("地址: {}", request.url());

    for (name, value) in request.headers() {
        // 只打印本库负责的头和示例自己设的头；真实调用方通常不会整体打印请求头。
        println!(
            "请求头: {name}: {}",
            value.to_str().unwrap_or("<非文本取值>")
        );
    }

    match request.body().and_then(|body| body.as_bytes()) {
        Some(bytes) => println!("请求体: {} 字节，已在内存", bytes.len()),
        None => println!("请求体: 流式发送，长度见 Content-Length"),
    }

    println!();
}

//! 只在本机构建 HEAD 请求，不连接任何服务端，也不需要账号。
//!
//! 运行方式：
//!
//! ```text
//! cargo run --example head_local
//! ```
//!
//! HEAD 与 GET 的路径语义完全一致，区别只有一个：**响应没有响应体**。因此它是
//! 「先问元数据、不下载内容」的手段——`Content-Length`、`Content-Type`、
//! `Last-Modified`、`ETag` 这些信息仍然在响应头里。
//!
//! 这里只调用 `build()`，不调用 `send()`：示例不发送任何请求，可以直接跑。
//! 要看真实的响应头读取，请参考 `tests/head/local/integration/metadata.rs` 与
//! `tests/head/network/read_only_contract.rs`。

use webdav_raw::{Client, HeadBuilder, Request, Url};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 真实调用方用 auth.head() 取 Builder，Authorization 会自动复用。
    let base_url = Url::parse("https://dav.example.com/remote.php/dav/files/alice/")?;
    let builder = || HeadBuilder::new(Client::new(), base_url.clone());

    // ---------- 1. 查询单个文件的元数据 ----------
    let request = builder().target_path("Documents/report.pdf")?.build()?;
    print_request("1. 查询文件元数据", &request);

    // ---------- 2. 查询集合（目录）本身 ----------
    // 注意末尾斜杠：WebDAV 里带斜杠表示集合。
    let request = builder().target_path("Documents/")?.build()?;
    print_request("2. 查询集合本身", &request);

    // ---------- 3. 条件请求：只在内容变化时才想知道新元数据 ----------
    // 服务端可以回 304 Not Modified，调用方按状态码处理。
    let request = builder()
        .target_path("Documents/report.pdf")?
        .header("if-none-match", "\"v1\"")?
        .build()?;
    print_request("3. 带 if-none-match 的条件 HEAD", &request);

    // ---------- 4. 中文与空格路径会被自动编码 ----------
    let request = builder().target_path("我的 文档/报告.pdf")?.build()?;
    print_request("4. 含空格与中文的路径", &request);

    println!("服务端应答后，响应头照常读取（示例不发送请求，这里只说明读法）：");
    println!("  response.headers().get(\"content-length\")");
    println!("  response.headers().get(\"content-type\")");
    println!("  response.headers().get(\"etag\")");
    println!("而 response.text() 一定是空字符串——HEAD 按协议不返回响应体。");

    Ok(())
}

/// 打印一个已经构建好的请求：方法、地址与请求头。
fn print_request(title: &str, request: &Request) {
    println!("=== {title} ===");
    println!("方法: {}", request.method());
    println!("地址: {}", request.url());

    for (name, value) in request.headers() {
        println!(
            "请求头: {name}: {}",
            value.to_str().unwrap_or("<非文本取值>")
        );
    }

    println!("请求体: 无（HEAD 没有请求体）");
    println!();
}

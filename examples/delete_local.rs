//! 只在本机构建 DELETE 请求，不连接任何服务端，也不需要账号。
//!
//! 运行方式：
//!
//! ```text
//! cargo run --example delete_local
//! ```
//!
//! 演示 `DeleteBuilder` 的默认值、`Depth` 两个合法取值、路径编码，以及删除集合
//! 时「递归」与「只删自身」的区别。这里只调用 `build()`，不调用 `send()`：示例
//! 不发送任何请求，也不删除任何远端资源，因此可以直接跑。
//!
//! 删除集合时 `Depth` 的两个取值语义完全不同：
//!
//! - `Depth::Infinity`（默认）：连同全部成员一起删掉。
//! - `Depth::Zero`：只删集合自身；非空集合会被服务端拒绝（通常 400 或 207）。
//!
//! 返回的状态码本库原样透传，`204` / `404` / `423` / `207` 都要由调用方自行判断。

use webdav_core::{Client, DeleteBuilder, DeleteDepth, Request, Url};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 真实调用方用 auth.delete() 取 Builder，Authorization 会自动复用。
    let base_url = Url::parse("https://dav.example.com/remote.php/dav/files/alice/")?;
    let builder = || DeleteBuilder::new(Client::new(), base_url.clone());

    // ---------- 1. 删除单个文件（默认递归深度） ----------
    let request = builder().target_path("staging/old.txt")?.build()?;
    print_request("1. 删除文件（默认 Depth）", &request);

    // ---------- 2. 递归删除非空集合 ----------
    // 非空文件夹必须用 Infinity 才能整体删掉。这是最常用的形态。
    let request = builder()
        .target_path("staging/")?
        .depth(DeleteDepth::Infinity)
        .build()?;
    print_request("2. 递归删除集合", &request);

    // ---------- 3. 只删集合自身 ----------
    // 空集合用 Zero 可以删掉；非空集合会被服务端拒绝，调用方按状态码处理。
    let request = builder()
        .target_path("empty-dir/")?
        .depth(DeleteDepth::Zero)
        .build()?;
    print_request("3. 只删集合自身", &request);

    // ---------- 4. 中文与空格路径会被自动编码 ----------
    let request = builder()
        .target_path("我的 目录/报告 2026.pdf")?
        .build()?;
    print_request("4. 含空格与中文的路径", &request);

    // ---------- 5. 条件删除 ----------
    // 调用方可以自己加请求头；本库只负责写 Depth 与调用方给的头。
    let request = builder()
        .target_path("guarded.txt")?
        .header("if-match", "\"v1\"")?
        .build()?;
    print_request("5. 带 if-match 的条件删除", &request);

    println!("注意：Depth 只有 0 与 infinity 两个合法取值，类型上不存在 One——");
    println!("DELETE 的 Depth: 1 是协议非法组合，本库用枚举把这种组合挡在编译期。");

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

    println!("请求体: 无（DELETE 没有请求体）");
    println!();
}

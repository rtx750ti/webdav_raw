//! 只在本机构建 COPY 请求，不连接任何服务端，也不需要账号。
//!
//! 运行方式：
//!
//! ```text
//! cargo run --example copy_local
//! ```
//!
//! COPY 有两个地址：**源**（请求行里的 URI，用 `source_path` / `source_url` 设置）
//! 和**目标**（`Destination` 请求头，用 `target_path` / `target_url` 设置）。
//!
//! # `Destination` 头由本库从 URL 生成
//!
//! 这是 COPY 最值得注意的一条：写进请求头的**不是**调用方传的原始字符串，而是已
//! 解析 URL 的重新序列化结果。因此空格与中文自动百分号编码，取值永远带 scheme 与
//! authority（满足 RFC 4918 对 Destination 必须是完整 URI 的要求）。第 3 组示例打
//! 印出来的 destination 就能直接看到这个效果。
//!
//! # 两个协议值的默认行为
//!
//! - `Overwrite` 默认 `True`，此时**不发送** `Overwrite` 头（RFC 的默认值就是 T）。
//!   只有显式设 `False` 才会写出 `Overwrite: F`。
//! - `CopyDepth` 默认 `Infinity`（递归复制）。
//!
//! 忘记设目标时 `build()` 会返回 `CopyError::MissingTarget`，不会发出一个没有
//! `Destination` 的请求。
//!
//! 这里只调用 `build()`，不调用 `send()`：示例不发送任何请求，也不修改任何远端
//! 资源，因此可以直接跑。

use webdav_core::{Client, CopyBuilder, CopyDepth, Overwrite, Request, Url};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 真实调用方用 auth.copy() 取 Builder，Authorization 会自动复用。
    let base_url = Url::parse("https://dav.example.com/remote.php/dav/files/alice/")?;
    let builder = || CopyBuilder::new(Client::new(), base_url.clone());

    // ---------- 1. 最简形态：默认允许覆盖、递归复制 ----------
    // 注意这里没有 Overwrite 头——默认值就是「按 RFC 默认处理」。
    let request = builder()
        .source_path("staging/report.pdf")?
        .target_path("release/report.pdf")?
        .build()?;
    print_request("1. 默认覆盖行为", &request);

    // ---------- 2. 明确禁止覆盖 ----------
    // 目标已存在时让请求失败（服务端通常回 412），避免手滑毁掉已有内容。
    let request = builder()
        .source_path("staging/report.pdf")?
        .target_path("release/report.pdf")?
        .overwrite(Overwrite::False)
        .build()?;
    print_request("2. 禁止覆盖", &request);

    // ---------- 3. 中文与空格目标：destination 自动百分号编码 ----------
    // 调用方写原始中文即可，不需要自己编码。
    let request = builder()
        .source_path("staging/report.pdf")?
        .target_path("备份/报告 2026.pdf")?
        .build()?;
    print_request("3. 中文与空格目标（观察 destination）", &request);

    // ---------- 4. 只复制集合本身，不含成员 ----------
    let request = builder()
        .source_path("staging/")?
        .target_path("release/")?
        .depth(CopyDepth::Zero)
        .build()?;
    print_request("4. Depth: 0 只复制集合自身", &request);

    // ---------- 5. 复制直接成员，不含更深层 ----------
    let request = builder()
        .source_path("staging/")?
        .target_path("release/")?
        .depth(CopyDepth::One)
        .build()?;
    print_request("5. Depth: 1 复制直接成员", &request);

    // ---------- 6. 目标用完整 URL ----------
    // 也支持跨主机目标；服务端是否接受由服务端决定（多数会拒绝）。
    let request = builder()
        .source_path("report.pdf")?
        .target_url("https://other.example.com/archive/report.pdf")?
        .build()?;
    print_request("6. 目标用完整 URL", &request);

    println!("状态的 201 / 204 / 207 / 403 / 409 / 412 / 423 都原样透传给调用方。");
    println!("只有集合被部分复制时服务端才回 207，想解析它用 send_and_deserialize()。");

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

    println!("请求体: 无（COPY 没有请求体）");
    println!();
}

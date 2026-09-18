//! 只在本机构建 MOVE 请求，不连接任何服务端，也不需要账号。
//!
//! 运行方式：
//!
//! ```text
//! cargo run --example move_local
//! ```
//!
//! MOVE 的语义是「COPY + 删源」：请求成功后**源资源不再存在**，目标位置出现同一份
//! 资源。重命名（同一集合内改名字）也是 MOVE 的一种用法。
//!
//! # 入口名为什么是 mv
//!
//! 真实调用方写 `auth.mv()`。`move` 是 Rust 关键字，作为方法名只能写成 `r#move()`，
//! 调用点很难看；模块名 `r#move` 只是实现细节，不出现在调用方代码里。
//!
//! # 与 COPY 的关系
//!
//! 源/目标方法名与 COPY 逐字一致（`source_path` / `source_url` / `target_path` /
//! `target_url`），并且复用同一个 `Overwrite` 类型——两种方法对「目标已存在时怎么办」
//! 的回答完全一致。
//!
//! # 怎么确认移动真的成功
//!
//! 本库只透传状态码。要确认结果就用 `head()` 或 `propfind()` 回查：源应该查不到，
//! 目标应该查得到。
//!
//! 这里只调用 `build()`，不调用 `send()`：示例不发送任何请求，也不移动任何远端
//! 资源，因此可以直接跑。

use webdav_raw::{Client, MoveBuilder, MoveDepth, Overwrite, Request, Url};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 真实调用方用 auth.mv() 取 Builder，Authorization 会自动复用。
    let base_url = Url::parse("https://dav.example.com/remote.php/dav/files/alice/")?;
    let builder = || MoveBuilder::new(Client::new(), base_url.clone());

    // ---------- 1. 最简形态：默认允许覆盖、递归移动 ----------
    let request = builder()
        .move_from_path("staging/report.pdf")?
        .target_path("archive/report.pdf")?
        .build()?;
    print_request("1. 归档一个文件", &request);

    // ---------- 2. 重命名：同一目录内换名字 ----------
    let request = builder()
        .move_from_path("draft.txt")?
        .target_path("final.txt")?
        .build()?;
    print_request("2. 重命名", &request);

    // ---------- 3. 禁止覆盖已有的目标 ----------
    let request = builder()
        .move_from_path("staging/report.pdf")?
        .target_path("archive/report.pdf")?
        .overwrite(Overwrite::False)
        .build()?;
    print_request("3. 禁止覆盖", &request);

    // ---------- 4. 中文与空格路径 ----------
    // 源会被编码进请求行，目标会被编码进 destination 头。
    let request = builder()
        .move_from_path("待归档/报告 2026.pdf")?
        .target_path("归档/报告 2026.pdf")?
        .build()?;
    print_request("4. 中文与空格路径", &request);

    // ---------- 5. 只移动集合自身 ----------
    let request = builder()
        .move_from_path("staging/")?
        .target_path("archive/")?
        .depth(MoveDepth::Zero)
        .build()?;
    print_request("5. Depth: 0 只移动集合自身", &request);

    // ---------- 6. 源用完整 URL ----------
    let request = builder()
        .source_url("https://other.example.com/old/report.pdf")?
        .target_path("archive/report.pdf")?
        .build()?;
    print_request("6. 源用完整 URL", &request);

    println!("move_from_path 与 source_path 完全等价，前者在调用点提醒「源会消失」。");
    println!("确认结果请回查：");
    println!("  auth.head().target_path(\"staging/report.pdf\")  // 应查不到");
    println!("  auth.head().target_path(\"archive/report.pdf\")  // 应查得到");
    println!("状态码 201 / 204 / 207 / 403 / 409 / 412 / 423 都原样透传给调用方。");

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

    println!("请求体: 无（MOVE 没有请求体）");
    println!();
}

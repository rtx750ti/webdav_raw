//! 只在本机构建 6 个 WebDAV 领域的请求，不连接任何服务端，也不需要账号。
//!
//! 运行方式：
//!
//! ```text
//! cargo run --example local_workflow
//! ```
//!
//! 这个示例演示的是「一个典型的发布流程」会依次用到哪些 Builder，以及每个
//! Builder 负责写哪些请求头。**它只调用 `build()`，不调用 `send()`**，因此不会
//! 向任何服务端发出请求，也不会修改任何远端资源，可以直接跑。
//!
//! 示例用的是不带认证头的普通 `Client`，打印请求时不会带出任何凭据。
//!
//! 要看真实的完整链路（含响应解析），请参考各领域的集成测试与 `tests/*/network/`。

use webdav_core::{
    Client, CopyDepth, DeleteDepth, MoveDepth, Overwrite, Request, Url,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 真实调用方在这里用 WebdavAuth::new(账号, 密码, 根地址)，然后 auth.options()、
    // auth.mkcol()、auth.mv() … 逐个取 Builder，Authorization 会自动复用。
    // 示例只演示请求构建，所以用一个普通 Client 指向示例地址。
    let base_url = Url::parse("https://dav.example.com/remote.php/dav/files/alice/")?;

    println!("根地址: {base_url}\n");

    // ---------- 1. OPTIONS：先问服务端支持什么 ----------
    // 只需要读响应头，没有请求体。`send_capabilities()` 会把 DAV 与 Allow
    // 解析成 OptionsCapabilities。
    let request = webdav_core::OptionsBuilder::new(Client::new(), base_url.clone()).build()?;
    print_request("1. OPTIONS 探测能力", &request);
    println!("服务端应答后可用 <Builder>::send_capabilities() 取到 dav_levels 与 allowed_methods。\n");

    // ---------- 2. MKCOL：建目标目录 ----------
    // 不设 body 时不发 Content-Type；MKCOL 不递归，多层目录要按层级依次调用。
    let request = webdav_core::MkcolBuilder::new(Client::new(), base_url.clone())
        .target_path("release/2026-Q1/")?
        .build()?;
    print_request("2. MKCOL 建目录", &request);
    println!("405 表示已存在，409 表示父目录不存在——状态码原样返回，由调用方判断。\n");

    // ---------- 3. HEAD：确认源文件在、有多大 ----------
    // 没有响应体，但 Content-Length / Content-Type / ETag 都在响应头里。
    let request = webdav_core::HeadBuilder::new(Client::new(), base_url.clone())
        .target_path("staging/report.pdf")?
        .build()?;
    print_request("3. HEAD 查元数据", &request);

    // ---------- 4. COPY：先留一份副本，禁止覆盖已有的正式版 ----------
    // Destination 由本库从目标 URL 生成：中文与空格自动百分号编码，取值永远带
    // scheme 与 authority。Overwrite::True（默认）时不发该头。
    let request = webdav_core::CopyBuilder::new(Client::new(), base_url.clone())
        .source_path("staging/report.pdf")?
        .target_path("release/2026-Q1/报告 2026.pdf")?
        .overwrite(Overwrite::False)
        .depth(CopyDepth::Infinity)
        .build()?;
    print_request("4. COPY 留副本", &request);
    println!("注意 destination 里的中文与空格已被百分号编码，调用方不需要自己编码。\n");

    // ---------- 5. MOVE：把 staging 那份归档，源随之消失 ----------
    // 入口名是 mv()（move 是 Rust 关键字）。MOVE 成功后源不再存在，要确认结果
    // 就用 head() 或 propfind() 回查。
    let request = webdav_core::MoveBuilder::new(Client::new(), base_url.clone())
        .move_from_path("staging/report.pdf")?
        .target_path("archive/2026-Q1/report.pdf")?
        .overwrite(Overwrite::False)
        .depth(MoveDepth::Infinity)
        .build()?;
    print_request("5. MOVE 归档", &request);
    println!("move_from_path 与 source_path 等价，前者在调用点提醒「源会消失」。\n");

    // ---------- 6. DELETE：清理 ----------
    // Depth 只有 0 与 infinity 两个合法取值，类型上就没有 One。
    let request = webdav_core::DeleteBuilder::new(Client::new(), base_url.clone())
        .target_path("release/2026-Q1/报告 2026.pdf")?
        .depth(DeleteDepth::Zero)
        .build()?;
    print_request("6. DELETE 清理", &request);

    // ---------- 7. PROPFIND：用服务端自己的记录核对最终状态 ----------
    // 这是已实现领域，用它核对上面几步的结果，而不是相信状态码。
    let request = webdav_core::PropFindBuilder::new(Client::new(), base_url.clone())
        .path("release/2026-Q1/")
        .depth(webdav_core::Depth::One)
        .props([
            webdav_core::FindProp::Resourcetype,
            webdav_core::FindProp::Getcontenttype,
            webdav_core::FindProp::Getetag,
        ])
        .build()?;
    print_request("7. PROPFIND 回查", &request);
    println!("PROPFIND 的 207 响应可用 send_and_deserialize() 解析成 MultiStatus。\n");

    println!("以上 6 个 Builder 的公开手感一致：relative/absolute 地址用 target_path/target_url，");
    println!("请求头用 header/headers，发送用 send()，状态码一律原样透传。");

    Ok(())
}

/// 打印一个已经构建好的请求：方法、地址、请求头和请求体形态。
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

    match request.body().and_then(|body| body.as_bytes()) {
        Some(bytes) => {
            if bytes.is_empty() {
                println!("请求体: 空");
            } else {
                println!("请求体: {} 字节，随请求发送", bytes.len());
            }
        }
        None => println!("请求体: 无（这些方法没有请求体）"),
    }

    println!();
}

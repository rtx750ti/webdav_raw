//! 只在本机构建 MKCOL 请求，不连接任何服务端，也不需要账号。
//!
//! 运行方式：
//!
//! ```text
//! cargo run --example mkcol_local
//! ```
//!
//! MKCOL 创建集合（目录），**不递归**：父集合不存在时服务端返回 409 Conflict，
//! 需要多层就按层级依次调用。
//!
//! 请求体默认是空的。按 RFC 4918 §9.3.1，MKCOL 可以带请求体且必须是 XML，但标准
//! 没有定义任何有用的 body 内容，所以本库默认不发。显式设置 body 时会补上
//! `Content-Type: application/xml; charset=utf-8`，但**不校验** body 是否为合法
//! XML——理由与 PUT 不拦 `ftp://` 相同：不和服务端造两套规则。
//!
//! 这里只调用 `build()`，不调用 `send()`：示例不发送任何请求，也不创建任何远端
//! 目录，因此可以直接跑。

use webdav_core::{Client, MkcolBuilder, Request, Url};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 真实调用方用 auth.mkcol() 取 Builder，Authorization 会自动复用。
    let base_url = Url::parse("https://dav.example.com/remote.php/dav/files/alice/")?;
    let builder = || MkcolBuilder::new(Client::new(), base_url.clone());

    // ---------- 1. 最简形态：不带请求体 ----------
    // 不发 Content-Type。这是绝大多数场景需要的形态。
    let request = builder().target_path("release/")?.build()?;
    print_request("1. 建目录（无请求体）", &request);

    // ---------- 2. 多层目录要逐层建 ----------
    // MKCOL 不递归：先建 release/，再建 release/2026-Q1/。
    let request = builder().target_path("release/2026-Q1/")?.build()?;
    print_request("2. 逐层建多层目录", &request);

    // ---------- 3. 带扩展 XML 请求体 ----------
    // 只有服务端明确支持时才用得上；设置了 body 就会带 XML 内容类型。
    let request = builder()
        .target_path("extended/")?
        .body(
            r#"<D:mkcol xmlns:D="DAV:"><D:set><D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop></D:set></D:mkcol>"#,
        )
        .build()?;
    print_request("3. 带 XML 请求体", &request);

    // ---------- 4. 调用方覆盖内容类型 ----------
    // 显式设置的值优先于本库为 body 补的默认值。
    let request = builder()
        .target_path("typed/")?
        .header("content-type", "text/xml")?
        .body("<D:mkcol xmlns:D=\"DAV:\"/>")
        .build()?;
    print_request("4. 调用方指定内容类型", &request);

    // ---------- 5. 中文与空格路径会被自动编码 ----------
    let request = builder().target_path("新建 目录/")?.build()?;
    print_request("5. 含空格与中文的路径", &request);

    println!("常见状态码（本库原样透传，不做判断）：");
    println!("  201 Created            创建成功");
    println!("  405 Method Not Allowed 资源已存在");
    println!("  409 Conflict           父集合不存在");
    println!("  415 Unsupported Media  服务端不接受请求体");

    Ok(())
}

/// 打印一个已经构建好的请求：方法、地址、请求头与请求体形态。
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
                println!("请求体: {} 字节", bytes.len());
            }
        }
        None => println!("请求体: 无"),
    }

    println!();
}

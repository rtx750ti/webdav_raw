//! 只在本机构建 OPTIONS 请求，不连接任何服务端，也不需要账号。
//!
//! 运行方式：
//!
//! ```text
//! cargo run --example options_local
//! ```
//!
//! OPTIONS 不返回资源内容，只回答「这台服务器允许做什么」。WebDAV 服务端把答案
//! 写在两个响应头里：
//!
//! - `DAV`：支持的 WebDAV 合规级别，例如 `1, 2, 3`。
//! - `Allow`：允许的 HTTP 方法列表。
//!
//! 本库提供两个发送入口：`send()` 交回原始响应，`send_capabilities()` 额外把上面
//! 两个头解析成 `OptionsCapabilities`。**本库只解析、不判定**——要不要因为缺少某个
//! 方法而跳过后续步骤，由调用方自己决定。
//!
//! 这里只调用 `build()`，不调用 `send()`：示例不发送任何请求，可以直接跑。

use webdav_raw::{Client, HeaderMap, HeaderValue, OptionsBuilder, OptionsCapabilities, Request, Url};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 真实调用方用 auth.options() 取 Builder，Authorization 会自动复用。
    let base_url = Url::parse("https://dav.example.com/remote.php/dav/files/alice/")?;
    let builder = || OptionsBuilder::new(Client::new(), base_url.clone());

    // ---------- 1. 探测认证根地址的能力 ----------
    // 不设路径时目标就是根地址本身。
    let request = builder().build()?;
    print_request("1. 探测根地址能力", &request);

    // ---------- 2. 探测某个具体集合的能力 ----------
    // 有些服务端对根目录与子目录声明不同的能力。
    let request = builder().target_path("Documents/")?.build()?;
    print_request("2. 探测子集合能力", &request);

    // ---------- 3. 带自定义请求头 ----------
    let request = builder().header("x-probe", "capabilities")?.build()?;
    print_request("3. 带自定义请求头", &request);

    // ---------- 4. 中文与空格路径会被自动编码 ----------
    let request = builder().target_path("我的 目录/")?.build()?;
    print_request("4. 含空格与中文的路径", &request);

    // ---------- 5. 能力解析是纯函数，可以脱网预览解析行为 ----------
    // `send_capabilities()` 内部就是拿响应头调它，所以这里能直接演示。
    let mut headers = HeaderMap::new();
    headers.insert("dav", HeaderValue::from_static("1, 2, 3"));
    headers.insert(
        "allow",
        HeaderValue::from_static("OPTIONS, GET, HEAD, DELETE, PROPFIND, PUT, COPY, MOVE, MKCOL"),
    );
    let caps = OptionsCapabilities::from_headers(&headers);
    println!("=== 5. 能力解析预览（本例唯一会实际执行的一步）===");
    println!("DAV 级别:     {:?}", caps.dav_levels);
    println!("允许的方法:   {:?}", caps.allowed_methods);
    println!();

    println!("调用方的典型用法：");
    println!("  let (response, caps) = auth.options().send_capabilities().await?;");
    println!("  if !caps.allowed_methods.iter().any(|m| m == \"MOVE\") {{");
    println!("      // 服务端不支持 MOVE，自己决定跳过还是报错");
    println!("  }}");
    println!("原始响应仍然拿得到，两个入口不是二选一。");

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

    println!("请求体: 无（OPTIONS 没有请求体）");
    println!();
}

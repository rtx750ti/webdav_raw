//! 目标地址到 `Destination` 请求头的转换。
//!
//! 这是 COPY 最核心的一条语义：请求头里的取值**不是**调用方传来的原字符串，
//! 而是已解析 URL 的重新序列化结果。用例逐个钉住这个区别带来的行为。

use webdav_raw::{Client, CopyBuilder, Url};

use crate::support::fixtures::{base_url, client};

/// 相对目标被补全成带 scheme 与 authority 的完整 URI。
#[test]
fn relative_target_becomes_absolute_uri() {
    let request = CopyBuilder::new(client(), base_url())
        .source_path("a.txt")
        .expect("合法相对路径应被接受")
        .target_path("b.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    let destination = request.headers().get("destination").unwrap();
    let parsed = Url::parse(destination.to_str().unwrap()).expect("Destination 应是完整 URI");

    assert_eq!(parsed.scheme(), "https");
    assert_eq!(parsed.host_str(), Some("example.com"));
    assert_eq!(parsed.path(), "/dav/b.txt");
}

/// 空格与中文被百分号编码，调用方不必自己编码。
#[test]
fn unicode_and_spaces_are_percent_encoded() {
    let request = CopyBuilder::new(client(), base_url())
        .source_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path("备份/报告 2026.pdf")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/dav/%E5%A4%87%E4%BB%BD/%E6%8A%A5%E5%91%8A%202026.pdf"
    );
}

/// 已经编码过的输入不会被二次编码。
#[test]
fn already_encoded_input_is_not_double_encoded() {
    let target = "%E5%A4%87%E4%BB%BD.txt";

    let request = CopyBuilder::new(client(), base_url())
        .source_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path(target)
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    let destination = request.headers().get("destination").unwrap().to_str().unwrap();
    assert!(
        !destination.contains("%25"),
        "不应出现二次编码的 %25，实际: {destination}"
    );
}

/// 调用方自己设置的 `Destination` 会被目标地址生成的值覆盖。
///
/// 目标地址只有一个可信来源：能设目标的地方是 `target_path` / `target_url`，
/// 不提供从请求头绕过的入口。
#[test]
fn caller_destination_header_is_overridden() {
    let request = CopyBuilder::new(client(), base_url())
        .source_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path("real-target.txt")
        .expect("合法相对路径应被接受")
        .header("destination", "https://evil.example.com/other.txt")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/dav/real-target.txt"
    );
    assert_eq!(request.headers().get_all("destination").iter().count(), 1);
}

/// 批量请求头里的 `Destination` 同样会被覆盖。
#[test]
fn batch_destination_header_is_overridden() {
    let mut headers = webdav_raw::HeaderMap::new();
    headers.insert(
        "destination",
        webdav_raw::HeaderValue::from_static("https://evil.example.com/other.txt"),
    );

    let request = CopyBuilder::new(client(), base_url())
        .target_path("real-target.txt")
        .expect("合法相对路径应被接受")
        .headers(headers)
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/dav/real-target.txt"
    );
}

/// 点路径与重复斜杠遵循 `Url` 的既有语义。
#[test]
fn target_normalizes_dot_segments_and_keeps_repeated_slashes() {
    let normalized = CopyBuilder::new(client(), base_url())
        .target_path("a/./b/../c.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");
    let repeated = CopyBuilder::new(client(), base_url())
        .target_path("a//b///c.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        normalized.headers().get("destination").unwrap(),
        "https://example.com/dav/a/c.txt"
    );
    assert_eq!(
        repeated.headers().get("destination").unwrap(),
        "https://example.com/dav/a//b///c.txt"
    );
}

/// 源与目标相同时本库不拦，照常构建（服务端可能回 403）。
#[test]
fn identical_source_and_target_are_not_blocked() {
    let request = CopyBuilder::new(Client::new(), base_url())
        .source_path("same.txt")
        .expect("合法相对路径应被接受")
        .target_path("same.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("本库不替服务端判定自复制");

    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/dav/same.txt"
    );
}

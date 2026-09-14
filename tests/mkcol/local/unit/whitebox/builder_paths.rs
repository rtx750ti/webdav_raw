//! Builder 的默认值、覆盖语义与请求装配。

use webdav_core::{MkcolBuilder, HeaderMap, HeaderValue};

use crate::support::fixtures::{base_url, client};

/// 未设置目标地址时使用传入的认证根地址，方法是 MKCOL。
#[test]
fn default_target_uses_auth_root_and_mkcol_method() {
    let request = MkcolBuilder::new(client(), base_url())
        .build()
        .expect("默认请求应构建成功");

    assert_eq!(request.method().as_str(), "MKCOL");
    assert_eq!(request.url().as_str(), "https://example.com/dav/");
}

/// 后设置的地址覆盖先设置的地址。
#[test]
fn later_target_setting_overrides_previous_setting() {
    let request = MkcolBuilder::new(client(), base_url())
        .target_path("first/")
        .expect("合法相对路径应被接受")
        .target_url("https://example.com/second/")
        .expect("合法绝对地址应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/second/");
}

/// 自定义请求头写入请求。
#[test]
fn custom_header_is_written_to_request() {
    let request = MkcolBuilder::new(client(), base_url())
        .header("x-note", "建目录")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-note").unwrap(), "建目录");
}

/// 同名请求头是替换语义，不追加第二个。
#[test]
fn later_header_replaces_earlier_one() {
    let request = MkcolBuilder::new(client(), base_url())
        .header("x-note", "first")
        .expect("合法请求头应被接受")
        .header("x-note", "second")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-note").unwrap(), "second");
    assert_eq!(request.headers().get_all("x-note").iter().count(), 1);
}

/// 批量请求头生效，并能覆盖单个请求头设过的同名值。
#[test]
fn batch_headers_are_applied_and_replace_single_value() {
    let mut headers = HeaderMap::new();
    headers.insert("x-note", HeaderValue::from_static("batch"));

    let request = MkcolBuilder::new(client(), base_url())
        .header("x-note", "single")
        .expect("合法请求头应被接受")
        .headers(headers)
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-note").unwrap(), "batch");
    assert_eq!(request.headers().get_all("x-note").iter().count(), 1);
}

/// 单个请求头也能覆盖批量请求头设过的同名值。
#[test]
fn single_header_replaces_batch_header_value() {
    let mut headers = HeaderMap::new();
    headers.insert("x-note", HeaderValue::from_static("batch"));

    let request = MkcolBuilder::new(client(), base_url())
        .headers(headers)
        .header("x-note", "single")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-note").unwrap(), "single");
}

/// `Method::from_bytes` 能构造出 MKCOL，`build()` 不因自定义方法失败。
#[test]
fn custom_method_is_accepted_by_request_builder() {
    let request = MkcolBuilder::new(client(), base_url())
        .build()
        .expect("自定义方法应能构建请求");

    assert_eq!(request.method().as_str(), "MKCOL");
    assert_ne!(request.method(), reqwest::Method::GET);
}

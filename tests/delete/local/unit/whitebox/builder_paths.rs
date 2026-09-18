//! Builder 的默认值、覆盖语义与请求装配。

use webdav_raw::{DeleteBuilder, DeleteDepth, HeaderMap, HeaderValue};

use crate::support::fixtures::{base_url, client};

/// 未设置目标地址时使用传入的认证根地址，并默认递归删除。
#[test]
fn default_target_and_depth() {
    let request = DeleteBuilder::new(client(), base_url())
        .build()
        .expect("默认请求应构建成功");

    assert_eq!(request.method().as_str(), "DELETE");
    assert_eq!(request.url().as_str(), "https://example.com/dav/");
    assert_eq!(request.headers().get("depth").unwrap(), "infinity");
}

/// 后设置的地址覆盖先设置的地址。
#[test]
fn later_target_setting_overrides_previous_setting() {
    let request = DeleteBuilder::new(client(), base_url())
        .target_path("first.txt")
        .expect("合法相对路径应被接受")
        .target_url("https://example.com/second.txt")
        .expect("合法绝对地址应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/second.txt");
}

/// `depth` 会覆盖默认值。
#[test]
fn depth_overrides_default() {
    let request = DeleteBuilder::new(client(), base_url())
        .depth(DeleteDepth::Zero)
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("depth").unwrap(), "0");
}

/// 自定义请求头写入请求。
#[test]
fn custom_header_is_written_to_request() {
    let request = DeleteBuilder::new(client(), base_url())
        .header("if-match", "\"v1\"")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("if-match").unwrap(), "\"v1\"");
}

/// 同名请求头是替换语义，不追加第二个。
#[test]
fn later_header_replaces_earlier_one() {
    let request = DeleteBuilder::new(client(), base_url())
        .header("x-trace", "first")
        .expect("合法请求头应被接受")
        .header("x-trace", "second")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-trace").unwrap(), "second");
    assert_eq!(request.headers().get_all("x-trace").iter().count(), 1);
}

/// 批量请求头生效。
#[test]
fn batch_headers_are_applied() {
    let mut headers = HeaderMap::new();
    headers.insert("x-trace", HeaderValue::from_static("batch"));

    let request = DeleteBuilder::new(client(), base_url())
        .headers(headers)
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-trace").unwrap(), "batch");
}

/// 批量请求头能覆盖单个请求头设过的同名值。
#[test]
fn batch_headers_replace_single_header_value() {
    let mut headers = HeaderMap::new();
    headers.insert("x-trace", HeaderValue::from_static("batch"));

    let request = DeleteBuilder::new(client(), base_url())
        .header("x-trace", "single")
        .expect("合法请求头应被接受")
        .headers(headers)
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-trace").unwrap(), "batch");
}

/// 单个请求头也能覆盖批量请求头设过的同名值。
#[test]
fn single_header_replaces_batch_header_value() {
    let mut headers = HeaderMap::new();
    headers.insert("x-trace", HeaderValue::from_static("batch"));

    let request = DeleteBuilder::new(client(), base_url())
        .headers(headers)
        .header("x-trace", "single")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-trace").unwrap(), "single");
}

/// 调用方设置的 `Depth` 请求头会被 Builder 的 `depth` 取值覆盖。
#[test]
fn caller_depth_header_is_overridden() {
    let request = DeleteBuilder::new(client(), base_url())
        .header("depth", "0")
        .expect("合法请求头应被接受")
        .depth(DeleteDepth::Infinity)
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("depth").unwrap(), "infinity");
    assert_eq!(request.headers().get_all("depth").iter().count(), 1);
}

/// 删除请求没有请求体。
#[test]
fn request_has_no_body() {
    let request = DeleteBuilder::new(client(), base_url())
        .build()
        .expect("请求应构建成功");

    assert!(request.body().is_none());
}

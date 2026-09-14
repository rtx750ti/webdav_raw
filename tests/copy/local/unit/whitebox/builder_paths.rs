//! Builder 的默认值、覆盖语义与请求装配。

use webdav_core::{CopyBuilder, CopyDepth, HeaderMap, HeaderValue, Overwrite};

use crate::support::fixtures::{base_url, client};

/// 未设置任何地址时，源是认证根地址；方法为 COPY。
#[test]
fn default_source_is_auth_root_and_method_is_copy() {
    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.method().as_str(), "COPY");
    assert_eq!(request.url().as_str(), "https://example.com/dav/");
}

/// 默认 `Depth` 是 infinity，随请求发出。
#[test]
fn default_depth_header_is_infinity() {
    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("depth").unwrap(), "infinity");
}

/// `CopyDepth::Zero` 与 `One` 如实写进 `Depth` 头。
#[test]
fn explicit_depths_reach_the_header() {
    let zero = CopyBuilder::new(client(), base_url())
        .target_path("t.txt")
        .expect("合法相对路径应被接受")
        .depth(CopyDepth::Zero)
        .build()
        .expect("请求应构建成功");
    let one = CopyBuilder::new(client(), base_url())
        .target_path("t.txt")
        .expect("合法相对路径应被接受")
        .depth(CopyDepth::One)
        .build()
        .expect("请求应构建成功");

    assert_eq!(zero.headers().get("depth").unwrap(), "0");
    assert_eq!(one.headers().get("depth").unwrap(), "1");
}

/// 默认 `Overwrite::True` 时**不发送** `Overwrite` 头。
#[test]
fn default_overwrite_header_is_absent() {
    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert!(
        request.headers().get("overwrite").is_none(),
        "默认应按 RFC 默认值处理，不发该头"
    );
}

/// 显式 `Overwrite::True` 同样不发该头。
#[test]
fn explicit_overwrite_true_still_omits_the_header() {
    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .overwrite(Overwrite::True)
        .build()
        .expect("请求应构建成功");

    assert!(request.headers().get("overwrite").is_none());
}

/// `Overwrite::False` 时写出 `Overwrite: F`。
#[test]
fn overwrite_false_writes_the_header() {
    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .overwrite(Overwrite::False)
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("overwrite").unwrap(), "F");
}

/// 先设 `False` 再设回 `True`，该头会消失。
#[test]
fn overwrite_can_be_switched_back_to_true() {
    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .overwrite(Overwrite::False)
        .overwrite(Overwrite::True)
        .build()
        .expect("请求应构建成功");

    assert!(request.headers().get("overwrite").is_none());
}

/// 调用方设置的 `Depth` 会被 Builder 的 `depth` 取值覆盖。
#[test]
fn caller_depth_header_is_overridden() {
    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .header("depth", "0")
        .expect("合法请求头应被接受")
        .depth(CopyDepth::Infinity)
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("depth").unwrap(), "infinity");
    assert_eq!(request.headers().get_all("depth").iter().count(), 1);
}

/// 自定义请求头写入请求。
#[test]
fn custom_header_is_written_to_request() {
    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .header("if-match", "\"v1\"")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("if-match").unwrap(), "\"v1\"");
}

/// 同名自定义请求头是替换语义，不追加第二个。
#[test]
fn later_header_replaces_earlier_one() {
    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .header("x-trace", "first")
        .expect("合法请求头应被接受")
        .header("x-trace", "second")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-trace").unwrap(), "second");
    assert_eq!(request.headers().get_all("x-trace").iter().count(), 1);
}

/// 批量请求头生效，并能覆盖单个请求头设过的同名值。
#[test]
fn batch_headers_are_applied_and_replace_single_value() {
    let mut headers = HeaderMap::new();
    headers.insert("x-trace", HeaderValue::from_static("batch"));

    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .header("x-trace", "single")
        .expect("合法请求头应被接受")
        .headers(headers)
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-trace").unwrap(), "batch");
}

/// COPY 请求没有请求体。
#[test]
fn request_has_no_body() {
    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert!(request.body().is_none());
}

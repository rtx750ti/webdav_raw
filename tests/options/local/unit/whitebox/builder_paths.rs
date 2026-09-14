//! Builder 的默认值、覆盖语义与请求装配。

use webdav_core::{HeaderMap, HeaderValue, OptionsBuilder};

use crate::support::fixtures::{base_url, client};

/// 未设置目标地址时使用传入的认证根地址，方法是 OPTIONS。
#[test]
fn default_target_uses_auth_root_and_options_method() {
    let request = OptionsBuilder::new(client(), base_url())
        .build()
        .expect("默认请求应构建成功");

    assert_eq!(request.method().as_str(), "OPTIONS");
    assert_eq!(request.url().as_str(), "https://example.com/dav/");
    assert!(request.body().is_none());
}

/// 后设置的地址覆盖先设置的地址。
#[test]
fn later_target_setting_overrides_previous_setting() {
    let request = OptionsBuilder::new(client(), base_url())
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
    let request = OptionsBuilder::new(client(), base_url())
        .header("x-probe", "capabilities")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-probe").unwrap(), "capabilities");
}

/// 同名请求头是替换语义，不追加第二个。
#[test]
fn later_header_replaces_earlier_one() {
    let request = OptionsBuilder::new(client(), base_url())
        .header("x-probe", "first")
        .expect("合法请求头应被接受")
        .header("x-probe", "second")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-probe").unwrap(), "second");
    assert_eq!(request.headers().get_all("x-probe").iter().count(), 1);
}

/// 批量请求头生效，并能覆盖单个请求头设过的同名值。
#[test]
fn batch_headers_are_applied_and_replace_single_value() {
    let mut headers = HeaderMap::new();
    headers.insert("x-probe", HeaderValue::from_static("batch"));

    let request = OptionsBuilder::new(client(), base_url())
        .header("x-probe", "single")
        .expect("合法请求头应被接受")
        .headers(headers)
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-probe").unwrap(), "batch");
    assert_eq!(request.headers().get_all("x-probe").iter().count(), 1);
}

/// 单个请求头也能覆盖批量请求头设过的同名值。
#[test]
fn single_header_replaces_batch_header_value() {
    let mut headers = HeaderMap::new();
    headers.insert("x-probe", HeaderValue::from_static("batch"));

    let request = OptionsBuilder::new(client(), base_url())
        .headers(headers)
        .header("x-probe", "single")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("x-probe").unwrap(), "single");
}

/// OPTIONS 请求没有请求体。
#[test]
fn request_has_no_body() {
    let request = OptionsBuilder::new(client(), base_url())
        .target_path("dir/")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert!(request.body().is_none());
}

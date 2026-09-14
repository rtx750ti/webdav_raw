//! Builder 的错误返回分支。

use webdav_core::{CopyBuilder, CopyError};

use crate::support::fixtures::{base_url, client};

/// 非法请求头名称返回 `HeaderName` 错误。
#[test]
fn invalid_header_name_returns_header_name_error() {
    let error = CopyBuilder::new(client(), base_url())
        .header("bad header", "value")
        .expect_err("非法请求头名称应被拒绝");

    assert!(matches!(error, CopyError::HeaderName(_)));
}

/// 非法请求头取值返回 `HeaderValue` 错误。
#[test]
fn invalid_header_value_returns_header_value_error() {
    let error = CopyBuilder::new(client(), base_url())
        .header("x-trace", "bad\nvalue")
        .expect_err("非法请求头取值应被拒绝");

    assert!(matches!(error, CopyError::HeaderValue(_)));
}

/// 未设置目标时既报 `MissingTarget`，也不发出任何请求。
#[tokio::test]
async fn missing_target_fails_before_any_request_is_sent() {
    let error = CopyBuilder::new(client(), base_url())
        .source_path("source.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect_err("缺目标时应报错");

    assert!(matches!(error, CopyError::MissingTarget));
}

/// 服务器不可达时 `send()` 返回 `Request` 错误。
#[tokio::test]
async fn send_returns_request_error_when_server_is_unreachable() {
    let target = webdav_core::Url::parse("http://127.0.0.1:1/dav/").expect("地址必须合法");

    let error = CopyBuilder::new(client(), target)
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect_err("不可达服务器应返回错误");

    assert!(matches!(error, CopyError::Request(_)));
}

/// 不可达时 `send_and_deserialize()` 同样报 `Request` 错误。
#[tokio::test]
async fn send_and_deserialize_reports_send_error() {
    let target = webdav_core::Url::parse("http://127.0.0.1:1/dav/").expect("地址必须合法");

    let error = CopyBuilder::new(client(), target)
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .send_and_deserialize()
        .await
        .expect_err("不可达服务器应返回错误");

    assert!(matches!(error, CopyError::Request(_)));
}

/// 不支持的协议在发送阶段报错，`build()` 不拦。
#[tokio::test]
async fn unsupported_scheme_is_reported_at_send_not_build() {
    let request = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .source_url("ftp://example.com/source.txt")
        .expect("语法合法就应通过")
        .build()
        .expect("构建阶段不检查协议");

    assert_eq!(request.url().scheme(), "ftp");

    let error = CopyBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .source_url("ftp://example.com/source.txt")
        .expect("语法合法就应通过")
        .send()
        .await
        .expect_err("发送阶段应报协议错误");

    assert!(matches!(error, CopyError::Request(_)));
}

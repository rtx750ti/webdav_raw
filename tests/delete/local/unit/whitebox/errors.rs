//! Builder 的错误返回分支。

use webdav_core::{DeleteBuilder, DeleteError};

use crate::support::fixtures::{base_url, client};

/// 非法请求头名称返回 `HeaderName` 错误。
#[test]
fn invalid_header_name_returns_header_name_error() {
    let error = DeleteBuilder::new(client(), base_url())
        .header("bad header", "value")
        .expect_err("非法请求头名称应被拒绝");

    assert!(matches!(error, DeleteError::HeaderName(_)));
}

/// 非法请求头取值返回 `HeaderValue` 错误。
#[test]
fn invalid_header_value_returns_header_value_error() {
    let error = DeleteBuilder::new(client(), base_url())
        .header("x-trace", "bad\nvalue")
        .expect_err("非法请求头取值应被拒绝");

    assert!(matches!(error, DeleteError::HeaderValue(_)));
}

/// 服务器不可达时 `send()` 返回 `Request` 错误。
#[tokio::test]
async fn send_returns_request_error_when_server_is_unreachable() {
    // 回环地址的保留端口上没有监听者。
    let target = webdav_core::Url::parse("http://127.0.0.1:1/dav/").expect("地址必须合法");

    let error = DeleteBuilder::new(client(), target)
        .send()
        .await
        .expect_err("不可达服务器应返回错误");

    assert!(matches!(error, DeleteError::Request(_)));
}

/// 不支持的协议在发送阶段报错，`build()` 不拦。
#[tokio::test]
async fn unsupported_scheme_is_reported_at_send_not_build() {
    let request = DeleteBuilder::new(client(), base_url())
        .target_url("ftp://example.com/file.txt")
        .expect("语法合法就应通过")
        .build()
        .expect("构建阶段不检查协议");

    assert_eq!(request.url().scheme(), "ftp");

    let error = DeleteBuilder::new(client(), base_url())
        .target_url("ftp://example.com/file.txt")
        .expect("语法合法就应通过")
        .send()
        .await
        .expect_err("发送阶段应报协议错误");

    assert!(matches!(error, DeleteError::Request(_)));
}

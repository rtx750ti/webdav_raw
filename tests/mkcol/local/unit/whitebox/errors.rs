//! Builder 的错误返回分支。

use webdav_raw::{MkcolBuilder, MkcolError};

use crate::support::fixtures::{base_url, client};

/// 服务器不可达时 `send()` 返回 `Request` 错误。
#[tokio::test]
async fn send_returns_request_error_when_server_is_unreachable() {
    let target = webdav_raw::Url::parse("http://127.0.0.1:1/dav/").expect("地址必须合法");

    let error = MkcolBuilder::new(client(), target)
        .send()
        .await
        .expect_err("不可达服务器应返回错误");

    assert!(matches!(error, MkcolError::Request(_)));
}

/// 不支持的协议在发送阶段报错，`build()` 不拦。
#[tokio::test]
async fn unsupported_scheme_is_reported_at_send_not_build() {
    let request = MkcolBuilder::new(client(), base_url())
        .target_url("ftp://example.com/dir/")
        .expect("语法合法就应通过")
        .build()
        .expect("构建阶段不检查协议");

    assert_eq!(request.url().scheme(), "ftp");

    let error = MkcolBuilder::new(client(), base_url())
        .target_url("ftp://example.com/dir/")
        .expect("语法合法就应通过")
        .send()
        .await
        .expect_err("发送阶段应报协议错误");

    assert!(matches!(error, MkcolError::Request(_)));
}

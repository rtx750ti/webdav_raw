//! 请求体的边界：不设置、空字符串、带内容与非 ASCII 内容。

use webdav_core::{MkcolBuilder, MkcolError};

use crate::support::fixtures::{base_url, client};

/// 不设置 body 时请求体为空，且不写 `Content-Type`。
#[test]
fn missing_body_sends_empty_body_without_content_type() {
    let request = MkcolBuilder::new(client(), base_url())
        .target_path("plain/")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert!(request.headers().get("content-type").is_none());
    assert_eq!(
        request.body().and_then(|body| body.as_bytes()),
        Some(&[][..])
    );
}

/// 空字符串等价于不设置 body。
#[test]
fn empty_body_string_equals_missing_body() {
    let request = MkcolBuilder::new(client(), base_url())
        .body("")
        .build()
        .expect("请求应构建成功");

    assert!(request.headers().get("content-type").is_none());
    assert_eq!(
        request.body().and_then(|body| body.as_bytes()),
        Some(&[][..])
    );
}

/// 非空 body 补上 XML 内容类型。
#[test]
fn non_empty_body_adds_xml_content_type() {
    let request = MkcolBuilder::new(client(), base_url())
        .body("<D:mkcol xmlns:D=\"DAV:\"/>")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.headers().get("content-type").unwrap(),
        "application/xml; charset=utf-8"
    );
}

/// 后设置 body 会覆盖先设置的 body。
#[test]
fn later_body_overrides_earlier_body() {
    let request = MkcolBuilder::new(client(), base_url())
        .body("first")
        .body("second")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.body().and_then(|body| body.as_bytes()),
        Some(&b"second"[..])
    );
}

/// 设置 body 后再清空成空字符串，会回到「无 body」状态。
#[test]
fn body_can_be_cleared_back_to_empty() {
    let request = MkcolBuilder::new(client(), base_url())
        .body("first")
        .body("")
        .build()
        .expect("请求应构建成功");

    assert!(request.headers().get("content-type").is_none());
    assert_eq!(
        request.body().and_then(|body| body.as_bytes()),
        Some(&[][..])
    );
}

/// 非法请求头名称返回 `HeaderName` 错误。
#[test]
fn invalid_header_name_returns_header_name_error() {
    let error = MkcolBuilder::new(client(), base_url())
        .header("bad header", "value")
        .expect_err("非法请求头名称应被拒绝");

    assert!(matches!(error, MkcolError::HeaderName(_)));
}

/// 非法请求头取值返回 `HeaderValue` 错误。
#[test]
fn invalid_header_value_returns_header_value_error() {
    let error = MkcolBuilder::new(client(), base_url())
        .header("x-note", "bad\nvalue")
        .expect_err("非法请求头取值应被拒绝");

    assert!(matches!(error, MkcolError::HeaderValue(_)));
}

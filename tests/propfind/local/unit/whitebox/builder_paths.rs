use webdav_core::propfind::builder::{Depth, PropFindBuilder, PropFindError};
use webdav_core::{Client, Url};

/// 验证 Builder 将路径、Depth、固定请求头和 PROPFIND 方法组装为请求。
#[test]
fn builder_builds_request_with_protocol_headers() {
    let base_url = Url::parse("https://example.com/webdav/").expect("固定地址必须有效");
    let request = PropFindBuilder::new(Client::new(), base_url)
        .path("folder")
        .depth(Depth::One)
        .allprop()
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.method().as_str(), "PROPFIND");
    assert_eq!(request.url().as_str(), "https://example.com/webdav/folder");
    assert_eq!(request.headers().get("Depth").expect("Depth 应存在"), "1");
    assert_eq!(
        request
            .headers()
            .get("Content-Type")
            .expect("Content-Type 应存在"),
        "application/xml; charset=utf-8"
    );
    assert_eq!(
        request.headers().get("Accept").expect("Accept 应存在"),
        "application/xml, text/xml, */*"
    );
}

/// 验证不能拼接相对路径的基础 URL 会返回 URL 错误。
#[test]
fn non_base_url_returns_url_error() {
    let invalid_base = Url::parse("data:text/plain,hello").expect("data URL 应可解析");
    let result = PropFindBuilder::new(Client::new(), invalid_base)
        .path("folder")
        .build();

    assert!(matches!(result, Err(PropFindError::Url(_))));
}

//! 能力解析在完整 HTTP 链路里的行为。
//!
//! 单元测试用的是手工构造的 `HeaderMap`；这里验证的是「服务端真的发了这两个头」
//! 时，解析结果与原始响应同时可用。

use webdav_core::WebdavAuth;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 典型的 WebDAV 服务端响应：`DAV` 与 `Allow` 都能解析出来。
#[tokio::test]
async fn typical_capabilities_are_parsed_from_live_response() {
    let server = local_http::start_server().await;
    Mock::given(method("OPTIONS"))
        .and(path("/dav/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("dav", "1, 2, 3")
                .insert_header(
                    "allow",
                    "OPTIONS, GET, HEAD, DELETE, PROPFIND, PUT, PROPPATCH, COPY, MOVE, MKCOL",
                ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let (response, caps) = auth
        .options()
        .send_capabilities()
        .await
        .expect("OPTIONS 应发送成功");

    assert_eq!(response.status(), 200);
    assert_eq!(caps.dav_levels, ["1", "2", "3"]);
    assert_eq!(
        caps.allowed_methods,
        [
            "OPTIONS",
            "GET",
            "HEAD",
            "DELETE",
            "PROPFIND",
            "PUT",
            "PROPPATCH",
            "COPY",
            "MOVE",
            "MKCOL"
        ]
    );
}

/// 服务端只声明 `DAV` 时，`Allow` 为空列表。
#[tokio::test]
async fn missing_allow_yields_empty_methods() {
    let server = local_http::start_server().await;
    Mock::given(method("OPTIONS"))
        .and(path("/dav/"))
        .respond_with(ResponseTemplate::new(200).insert_header("dav", "1, 2"))
        .expect(1)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let (_, caps) = auth
        .options()
        .send_capabilities()
        .await
        .expect("OPTIONS 应发送成功");

    assert_eq!(caps.dav_levels, ["1", "2"]);
    assert!(caps.allowed_methods.is_empty());
}

/// 服务端一个能力头都不给时，返回两个空列表而不是报错。
#[tokio::test]
async fn response_without_capability_headers_yields_empty_lists() {
    let server = local_http::start_server().await;
    Mock::given(method("OPTIONS"))
        .and(path("/dav/"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let (response, caps) = auth
        .options()
        .send_capabilities()
        .await
        .expect("OPTIONS 应发送成功");

    assert_eq!(response.status(), 200);
    assert!(caps.dav_levels.is_empty());
    assert!(caps.allowed_methods.is_empty());
}

/// `send()` 与 `send_capabilities()` 对同一个响应给出一致的状态码与原始头。
#[tokio::test]
async fn raw_send_and_capabilities_agree_on_the_response() {
    let server = local_http::start_server().await;
    Mock::given(method("OPTIONS"))
        .and(path("/dav/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("dav", "1, 2")
                .insert_header("x-server", "mock"),
        )
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let raw = auth
        .options()
        .send()
        .await
        .expect("原始入口应发送成功");
    let (structured, caps) = auth
        .options()
        .send_capabilities()
        .await
        .expect("结构化入口应发送成功");

    assert_eq!(raw.status(), structured.status());
    assert_eq!(
        raw.headers().get("dav"),
        structured.headers().get("dav"),
        "两个入口拿到的是同一个响应头"
    );
    assert_eq!(
        structured.headers().get("x-server").unwrap(),
        "mock",
        "结构化入口不吞掉其余响应头"
    );
    assert_eq!(caps.dav_levels, ["1", "2"]);
}

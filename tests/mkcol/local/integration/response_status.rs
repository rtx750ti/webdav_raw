//! 状态码行为：MKCOL 的几个「常态错误码」原样透传。
//!
//! MKCOL 的特殊之处在于失败码本身就是正常业务信号，调用方需要自己判断：
//!
//! - `405 Method Not Allowed`：资源已存在
//! - `409 Conflict`：父集合不存在
//! - `415 Unsupported Media Type`：服务端不接受请求体
//!
//! 本库不做这些判断，只保证状态码、响应头和响应体都能拿到。

use webdav_raw::WebdavAuth;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 逐码验证 MKCOL 的成功与失败状态都原样返回。
#[tokio::test]
async fn mkcol_status_codes_are_passed_through_unchanged() {
    let server = local_http::start_server().await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let codes = [200u16, 201, 204, 400, 401, 403, 405, 409, 415, 423, 500, 507];

    for code in codes {
        let target = format!("code-{code}/");
        Mock::given(method("MKCOL"))
            .and(path(format!("/dav/{target}")))
            .respond_with(ResponseTemplate::new(code))
            .expect(1)
            .mount(&server)
            .await;

        let response = auth
            .mkcol()
            .target_path(&target)
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("MKCOL 应发送成功");

        assert_eq!(response.status().as_u16(), code, "{code} 应原样返回");
    }
}

/// 405（已存在）时响应头仍然可读，调用方可以据此区分「已存在」和其他失败。
#[tokio::test]
async fn already_exists_keeps_response_headers() {
    let server = local_http::start_server().await;
    Mock::given(method("MKCOL"))
        .and(path("/dav/existing/"))
        .respond_with(ResponseTemplate::new(405).insert_header("allow", "PROPFIND, DELETE"))
        .expect(1)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let response = auth
        .mkcol()
        .target_path("existing/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功");

    assert_eq!(response.status(), 405);
    assert_eq!(
        response.headers().get("allow").unwrap(),
        "PROPFIND, DELETE"
    );
}

/// 409（父集合不存在）的响应体原样保留。
#[tokio::test]
async fn missing_parent_keeps_response_body() {
    let server = local_http::start_server().await;
    Mock::given(method("MKCOL"))
        .and(path("/dav/a/b/c/"))
        .respond_with(
            ResponseTemplate::new(409).set_body_string("<D:error xmlns:D=\"DAV:\"/>"),
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

    let response = auth
        .mkcol()
        .target_path("a/b/c/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功");

    assert_eq!(response.status(), 409);
    let body = response.text().await.expect("响应体应可读");
    assert!(body.contains("D:error"), "响应体应原样保留，实际: {body}");
}

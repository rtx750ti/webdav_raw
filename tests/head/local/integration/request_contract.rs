//! HEAD 请求契约：方法、目标地址与请求头。

use webdav_raw::WebdavAuth;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 认证对象创建的 HEAD 请求：方法、目标路径与默认 Authorization 都正确。
#[tokio::test]
async fn authenticated_head_uses_auth_root_and_method() {
    let server = local_http::start_server().await;
    Mock::given(method("HEAD"))
        .and(path("/dav/Documents/report.pdf"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(200).insert_header("content-length", "11"))
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
        .head()
        .target_path("Documents/report.pdf")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");

    assert_eq!(response.status(), 200);
}

/// 默认目标指向认证根地址本身。
#[tokio::test]
async fn default_target_is_auth_root() {
    let server = local_http::start_server().await;
    Mock::given(method("HEAD"))
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

    let response = auth.head().send().await.expect("HEAD 应发送成功");

    assert_eq!(response.status(), 200);
}

/// 中文与空格路径在真实请求里被正确编码。
#[tokio::test]
async fn encoded_path_reaches_server() {
    let server = local_http::start_server().await;
    Mock::given(method("HEAD"))
        .and(path("/dav/%E6%88%91%E7%9A%84%20%E7%9B%AE%E5%BD%95/%E6%8A%A5%E5%91%8A.pdf"))
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

    let response = auth
        .head()
        .target_path("我的 目录/报告.pdf")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");

    assert_eq!(response.status(), 200);
}

/// 调用方设置的条件请求头原样到达服务端。
#[tokio::test]
async fn caller_set_conditional_header_reaches_server() {
    let server = local_http::start_server().await;
    Mock::given(method("HEAD"))
        .and(path("/dav/guarded.txt"))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
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
        .head()
        .target_path("guarded.txt")
        .expect("合法相对路径应被接受")
        .header("if-none-match", "\"v1\"")
        .expect("合法请求头应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");

    assert_eq!(response.status(), 304);
}

/// `target_url` 换的是地址，不是 Client：默认 Authorization 也跟着走。
#[tokio::test]
async fn target_url_sends_default_authorization_to_another_origin() {
    let auth_server = local_http::start_server().await;
    let other_server = local_http::start_server().await;

    Mock::given(method("HEAD"))
        .and(path("/file.txt"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&other_server)
        .await;

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&auth_server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let response = auth
        .head()
        .target_url(&format!("{}/file.txt", other_server.uri()))
        .expect("合法绝对地址应被接受")
        .send()
        .await
        .expect("跨服务 HEAD 应发送成功");

    assert_eq!(response.status(), 200);
}

/// 以斜杠开头的相对路径覆盖认证根地址中已有的路径前缀。
#[tokio::test]
async fn leading_slash_path_replaces_auth_root_prefix() {
    let server = local_http::start_server().await;
    Mock::given(method("HEAD"))
        .and(path("/other/file.txt"))
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

    let response = auth
        .head()
        .target_path("/other/file.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");

    assert_eq!(response.status(), 200);
}

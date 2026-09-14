//! MKCOL 请求契约：方法、目标地址、请求体与内容类型。

use webdav_core::WebdavAuth;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 认证对象创建的 MKCOL 请求：方法、目标路径与默认 Authorization 都正确。
#[tokio::test]
async fn authenticated_mkcol_uses_auth_root_and_method() {
    let server = local_http::start_server().await;
    Mock::given(method("MKCOL"))
        .and(path("/dav/release/"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(201))
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
        .target_path("release/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 默认目标指向认证根地址本身。
#[tokio::test]
async fn default_target_is_auth_root() {
    let server = local_http::start_server().await;
    Mock::given(method("MKCOL"))
        .and(path("/dav/"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let response = auth.mkcol().send().await.expect("MKCOL 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 不设置 body 时服务端收到零长度请求体，且没有 `Content-Type`。
#[tokio::test]
async fn missing_body_reaches_server_as_empty_without_content_type() {
    let server = local_http::start_server().await;
    Mock::given(method("MKCOL"))
        .and(path("/dav/plain/"))
        .and(body_string(""))
        .respond_with(ResponseTemplate::new(201))
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
        .target_path("plain/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 设置了 body 时，字节与 XML 内容类型都到达服务端。
#[tokio::test]
async fn body_and_xml_content_type_reach_server() {
    let server = local_http::start_server().await;
    let body = r#"<D:mkcol xmlns:D="DAV:"><D:set><D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop></D:set></D:mkcol>"#;
    Mock::given(method("MKCOL"))
        .and(path("/dav/extended/"))
        .and(header("content-type", "application/xml; charset=utf-8"))
        .and(body_string(body))
        .respond_with(ResponseTemplate::new(201))
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
        .target_path("extended/")
        .expect("合法相对路径应被接受")
        .body(body)
        .send()
        .await
        .expect("MKCOL 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 调用方显式设置的内容类型覆盖本库为 body 补的 XML 默认值。
#[tokio::test]
async fn caller_content_type_overrides_default() {
    let server = local_http::start_server().await;
    Mock::given(method("MKCOL"))
        .and(path("/dav/typed/"))
        .and(header("content-type", "text/xml"))
        .respond_with(ResponseTemplate::new(201))
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
        .target_path("typed/")
        .expect("合法相对路径应被接受")
        .header("content-type", "text/xml")
        .expect("合法请求头应被接受")
        .body("<D:mkcol xmlns:D=\"DAV:\"/>")
        .send()
        .await
        .expect("MKCOL 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 中文与空格路径在真实请求里被正确编码。
#[tokio::test]
async fn encoded_path_reaches_server() {
    let server = local_http::start_server().await;
    Mock::given(method("MKCOL"))
        .and(path("/dav/%E6%96%B0%E5%BB%BA%20%E7%9B%AE%E5%BD%95/"))
        .respond_with(ResponseTemplate::new(201))
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
        .target_path("新建 目录/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功");

    assert_eq!(response.status(), 201);
}

/// `target_url` 换的是地址，不是 Client：默认 Authorization 也跟着走。
#[tokio::test]
async fn target_url_sends_default_authorization_to_another_origin() {
    let auth_server = local_http::start_server().await;
    let other_server = local_http::start_server().await;

    Mock::given(method("MKCOL"))
        .and(path("/dir/"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(201))
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
        .mkcol()
        .target_url(&format!("{}/dir/", other_server.uri()))
        .expect("合法绝对地址应被接受")
        .send()
        .await
        .expect("跨服务 MKCOL 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 以斜杠开头的相对路径覆盖认证根地址中已有的路径前缀。
#[tokio::test]
async fn leading_slash_path_replaces_auth_root_prefix() {
    let server = local_http::start_server().await;
    Mock::given(method("MKCOL"))
        .and(path("/other/dir/"))
        .respond_with(ResponseTemplate::new(201))
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
        .target_path("/other/dir/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功");

    assert_eq!(response.status(), 201);
}

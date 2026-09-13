#[path = "../../../common/local_http.rs"]
mod local_http;

use webdav_core::WebdavAuth;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::support::{bytes_body, bytes_body_with_content_type};

/// 认证对象创建 PUT 请求：方法、目标路径与默认 Authorization 都正确。
#[tokio::test]
async fn authenticated_put_uses_auth_root_and_put_method() {
    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/dav/file.txt"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .and(body_string("content"))
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
        .put()
        .relative_path("file.txt")
        .expect("相对路径必须有效")
        .body(bytes_body_with_content_type(b"content".to_vec(), "text/plain"))
        .send()
        .await
        .expect("PUT 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 默认路径指向认证根地址本身。
#[tokio::test]
async fn default_put_targets_auth_root() {
    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
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

    let response = auth.put().send().await.expect("空 body 的 PUT 应发送成功");

    assert_eq!(response.status(), 200);
}

/// 不设置数据源时发送零长度请求体。
#[tokio::test]
async fn missing_body_sends_zero_length_request() {
    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/dav/empty.txt"))
        .and(header("content-length", "0"))
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
        .put()
        .relative_path("empty.txt")
        .expect("相对路径必须有效")
        .send()
        .await
        .expect("PUT 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 调用方设置的 `Content-Type` 优先于内存源元数据里的值。
#[tokio::test]
async fn caller_content_type_wins_over_metadata() {
    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/dav/typed.bin"))
        .and(header("content-type", "application/x-custom"))
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
        .put()
        .relative_path("typed.bin")
        .expect("相对路径必须有效")
        .header("content-type", "application/x-custom")
        .expect("合法请求头应被接受")
        .body(bytes_body_with_content_type(vec![1], "text/plain"))
        .send()
        .await
        .expect("PUT 应发送成功");

    assert_eq!(response.status(), 201);
}

/// `Content-Length` 始终由数据源长度决定，调用方设的值不生效。
#[tokio::test]
async fn caller_content_length_is_ignored() {
    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/dav/len.bin"))
        .and(header("content-length", "1"))
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
        .put()
        .relative_path("len.bin")
        .expect("相对路径必须有效")
        .header("content-length", "999")
        .expect("合法请求头名应被接受")
        .body(bytes_body(vec![7]))
        .send()
        .await
        .expect("PUT 应发送成功");

    assert_eq!(response.status(), 201, "服务端应收到实际长度 1");
}

/// 响应头与响应体原样交给调用方。
#[tokio::test]
async fn response_headers_and_body_are_preserved() {
    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/dav/kept.txt"))
        .respond_with(
            ResponseTemplate::new(201)
                .insert_header("etag", "\"v1\"")
                .set_body_string("stored"),
        )
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let response = auth
        .put()
        .relative_path("kept.txt")
        .expect("相对路径必须有效")
        .body(bytes_body(b"x".to_vec()))
        .send()
        .await
        .expect("PUT 应发送成功");

    assert_eq!(response.status(), 201);
    assert_eq!(response.headers().get("etag").unwrap(), "\"v1\"");
    assert_eq!(response.text().await.unwrap(), "stored");
}

/// 以 `/` 开头的相对路径会覆盖认证根地址中已有的路径前缀。
#[tokio::test]
async fn leading_slash_path_replaces_auth_root_prefix() {
    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/other/file.txt"))
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
        .put()
        .relative_path("/other/file.txt")
        .expect("相对路径必须有效")
        .body(bytes_body(b"x".to_vec()))
        .send()
        .await
        .expect("PUT 应发送成功");

    assert_eq!(response.status(), 201);
}

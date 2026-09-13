//! PUT 请求契约：方法、目标地址、请求头与请求体。
//!
//! 全部经本地 MockServer 发出真实 HTTP 请求，验证公开调用链而不只是请求对象。

use webdav_core::WebdavAuth;
use webdav_core::{PutBody, U8BytesChunk, U8BytesData};
use wiremock::matchers::{body_bytes, body_string, header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;
use crate::support::fixtures::{bytes_body, bytes_body_with_content_type};

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
        .body(bytes_body_with_content_type(
            b"content".to_vec(),
            "text/plain",
        ))
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

/// 分片范围只有在调用方显式设置时才成为请求头，服务端能原样收到。
///
/// 本库默认不发 `Content-Range`：标准 WebDAV 的 PUT 是单次资源写入，服务端忽略
/// 该头时通常回 200/201，与"支持部分上传"无法区分。这里验证的是"要发就能发"。
#[tokio::test]
async fn explicit_content_range_header_reaches_server() {
    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/dav/part.bin"))
        .and(header("content-range", "bytes 0-3/8"))
        .and(header("content-length", "4"))
        .and(body_bytes(vec![0xAB; 4]))
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

    let data = U8BytesData::new(vec![0xAB; 4], None).expect("应构造成功");
    let chunk = U8BytesChunk::new(data, Some(0), Some(3), Some(8), None).expect("应构造成功");
    let range = chunk.to_content_range().expect("分片带范围");

    let response = auth
        .put()
        .relative_path("part.bin")
        .expect("相对路径必须有效")
        .body(PutBody::from_chunk(chunk))
        .header("content-range", &range)
        .expect("合法请求头应被接受")
        .send()
        .await
        .expect("分片 PUT 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 条件请求头由调用方自己设置，本库不会从元数据里自动生成。
#[tokio::test]
async fn caller_set_conditional_header_reaches_server() {
    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/dav/guarded.bin"))
        .and(header("if-match", "\"v1\""))
        .and(header("if-none-match", "*"))
        .respond_with(ResponseTemplate::new(204))
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
        .relative_path("guarded.bin")
        .expect("相对路径必须有效")
        .header("if-match", "\"v1\"")
        .expect("合法请求头应被接受")
        .header("if-none-match", "*")
        .expect("合法请求头应被接受")
        .body(bytes_body(b"x".to_vec()))
        .send()
        .await
        .expect("PUT 应发送成功");

    assert_eq!(response.status(), 204);
}

/// `absolute_path` 换的是地址，不是 Client：默认 Authorization 也跟着走。
///
/// 认证对象注入的 `Authorization` 是 Client 级默认头，对本 Client 的每个请求都
/// 生效。本测试用两个回环端口模拟两个服务，固定这条既有行为：需要跨服务上传时
/// 靠它，不需要时应当用 `relative_path`，否则凭据会发给无关服务。
#[tokio::test]
async fn absolute_path_sends_default_authorization_to_another_origin() {
    let auth_server = local_http::start_server().await;
    let upload_server = local_http::start_server().await;

    Mock::given(method("PUT"))
        .and(path("/file.bin"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&upload_server)
        .await;

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&auth_server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let response = auth
        .put()
        .absolute_path(&format!("{}/file.bin", upload_server.uri()))
        .expect("合法绝对地址应被接受")
        .body(bytes_body(b"x".to_vec()))
        .send()
        .await
        .expect("跨服务 PUT 应发送成功");

    assert_eq!(response.status(), 201);
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

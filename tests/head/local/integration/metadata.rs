//! 元数据行为：HEAD 没有响应体，但响应头全部保留。
//!
//! 这是 HEAD 唯一区别于 GET 的地方，也是它存在的意义：先问元数据，不下载内容。
//! 这些用例把「响应头能读到」和「响应体确实是空的」两件事分别钉住。

use webdav_raw::WebdavAuth;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// `Content-Length`、`Content-Type`、`ETag`、`Last-Modified` 都能从响应头读到。
#[tokio::test]
async fn metadata_headers_are_all_readable() {
    let server = local_http::start_server().await;
    Mock::given(method("HEAD"))
        .and(path("/dav/meta.bin"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-length", "4096")
                .insert_header("content-type", "application/octet-stream")
                .insert_header("etag", "\"abc123\"")
                .insert_header("last-modified", "Sat, 16 Aug 2025 10:46:36 GMT"),
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
        .head()
        .target_path("meta.bin")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");

    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-length").unwrap(), "4096");
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );
    assert_eq!(response.headers().get("etag").unwrap(), "\"abc123\"");
    assert_eq!(
        response.headers().get("last-modified").unwrap(),
        "Sat, 16 Aug 2025 10:46:36 GMT"
    );
}

/// HEAD 的响应体为空；服务端即使带了 body，读取结果也应是空的。
#[tokio::test]
async fn head_response_has_empty_body() {
    let server = local_http::start_server().await;
    Mock::given(method("HEAD"))
        .and(path("/dav/with-body.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("should not be delivered"))
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
        .target_path("with-body.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");

    let body = response.text().await.expect("响应体应可读");
    assert!(body.is_empty(), "HEAD 响应体必须为空，实际: {body:?}");
}

/// HEAD 与 GET 走同一路径语义：同一资源的响应头一致，而 GET 才有响应体。
#[tokio::test]
async fn head_and_get_agree_on_headers_but_differ_on_body() {
    let server = local_http::start_server().await;

    // GET 会带 body；HEAD 由 wiremock 自动省略 body。
    Mock::given(method("HEAD"))
        .and(path("/dav/same.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-length", "7")
                .insert_header("content-type", "text/plain"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/dav/same.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-length", "7")
                .insert_header("content-type", "text/plain")
                .set_body_string("content"),
        )
        .mount(&server)
        .await;

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let head_response = auth
        .head()
        .target_path("same.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");
    let head_status = head_response.status();
    let head_type = head_response.headers().get("content-type").cloned();
    let head_body = head_response.text().await.expect("HEAD 响应体应可读");

    let get_response = auth
        .get()
        .relative_path("same.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("GET 应发送成功");
    let get_status = get_response.status();
    let get_type = get_response.headers().get("content-type").cloned();
    let get_body = get_response.text().await.expect("GET 响应体应可读");

    assert_eq!(head_status, get_status, "两者状态码应一致");
    assert_eq!(head_type, get_type, "两者内容类型应一致");
    assert!(head_body.is_empty(), "HEAD 不应有响应体");
    assert_eq!(get_body, "content", "GET 才有响应体");
}

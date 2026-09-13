use webdav_core::WebdavAuth;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 验证成功响应的状态、响应头和响应体会原样交给调用方。
#[tokio::test]
async fn success_response_preserves_status_headers_and_body() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    Mock::given(method("GET"))
        .and(path("/dav/file.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-webdav-test", "present")
                .set_body_string("file-content"),
        )
        .mount(&server)
        .await;
    let auth =
        WebdavAuth::new("alice", "password", base_url.as_str()).expect("回环地址应创建认证对象");

    let response = auth
        .get()
        .relative_url("file.txt".to_owned())
        .send()
        .await
        .expect("请求应发送成功");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get("x-webdav-test")
            .expect("响应头应存在"),
        "present"
    );
    assert_eq!(
        response.text().await.expect("响应体应可读取"),
        "file-content"
    );
}

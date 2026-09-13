use webdav_core::WebdavAuth;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 验证默认 GET 请求使用认证根地址和 GET 方法。
#[tokio::test]
async fn default_get_uses_auth_root_and_get_method() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    Mock::given(method("GET"))
        .and(path("/dav/"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let auth =
        WebdavAuth::new("alice", "password", base_url.as_str()).expect("回环地址应创建认证对象");

    let response = auth.get().send().await.expect("请求应发送成功");

    assert_eq!(response.status(), 200);
}

/// 验证相对路径在实际 GET 请求中保留认证根路径。
#[tokio::test]
async fn relative_get_preserves_auth_root_path() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    Mock::given(method("GET"))
        .and(path("/dav/Documents/report.txt"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let auth =
        WebdavAuth::new("alice", "password", base_url.as_str()).expect("回环地址应创建认证对象");

    let response = auth
        .get()
        .relative_url("Documents/report.txt".to_owned())
        .send()
        .await
        .expect("请求应发送成功");

    assert_eq!(response.status(), 200);
}

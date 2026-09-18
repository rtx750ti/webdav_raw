use webdav_raw::WebdavAuth;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 验证认证对象发出的 GET 请求携带默认 Basic Authorization 请求头。
#[tokio::test]
async fn authenticated_get_sends_basic_authorization_header() {
    let server = local_http::start_server().await;
    Mock::given(method("GET"))
        .and(path("/dav/health"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let base_url = local_http::base_url(&server, "dav");
    let auth =
        WebdavAuth::new("alice", "password", base_url.as_str()).expect("回环地址应创建认证对象");

    let response = auth
        .get()
        .relative_path("health")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("本地请求应发送成功");

    assert_eq!(response.status(), 200);
}

/// 验证 `get_client()` 交出的原始 Client 已经装好默认 Authorization。
///
/// 调用方拿这个 Client 直接发请求时，凭据同样会带上；这也是「默认请求头只装一次」
/// 的对外承诺。两次取到的是同一个实例，不是每次新建。
#[tokio::test]
async fn raw_client_carries_default_authorization() {
    let server = local_http::start_server().await;
    Mock::given(method("GET"))
        .and(path("/dav/raw"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let base_url = local_http::base_url(&server, "dav");
    let auth =
        WebdavAuth::new("alice", "password", base_url.as_str()).expect("回环地址应创建认证对象");

    assert!(
        std::ptr::eq(auth.get_client(), auth.get_client()),
        "每次取到的应是同一个 Client"
    );

    let response = auth
        .get_client()
        .get(base_url.join("raw").expect("地址应可拼接"))
        .send()
        .await
        .expect("用原始 Client 发请求应成功");

    assert_eq!(response.status(), 204);
}

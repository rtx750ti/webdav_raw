#[path = "../../../common/local_http.rs"]
mod local_http;

use webdav_core::auth::WebdavAuth;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

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
        .relative_url("health".to_owned())
        .send()
        .await
        .expect("本地请求应发送成功");

    assert_eq!(response.status(), 200);
}

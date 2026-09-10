#[path = "../../../common/local_http.rs"]
mod local_http;

use webdav_core::auth::WebdavAuth;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

/// 验证 HTTP 错误状态仍作为原始响应返回给调用方。
#[tokio::test]
async fn error_statuses_return_raw_response() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    for status in [401, 404, 500] {
        Mock::given(method("GET"))
            .and(path(format!("/dav/status-{status}")))
            .respond_with(ResponseTemplate::new(status))
            .mount(&server)
            .await;
    }
    let auth =
        WebdavAuth::new("alice", "password", base_url.as_str()).expect("回环地址应创建认证对象");

    for status in [401, 404, 500] {
        let response = auth
            .get()
            .relative_url(format!("status-{status}"))
            .send()
            .await
            .expect("错误状态应仍返回响应");
        assert_eq!(response.status().as_u16(), status);
    }
}

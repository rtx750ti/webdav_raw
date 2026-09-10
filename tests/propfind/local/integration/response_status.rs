#[path = "../../../common/local_http.rs"]
mod local_http;

use webdav_core::Client;
use webdav_core::propfind::builder::{PropFindBuilder, PropFindError};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

/// 验证非 207 响应会映射为携带原始状态码的协议错误。
#[tokio::test]
async fn non_multi_status_response_returns_unexpected_status() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    for status in [200, 401, 404, 500] {
        Mock::given(method("PROPFIND"))
            .and(path(format!("/dav/status-{status}")))
            .respond_with(ResponseTemplate::new(status))
            .mount(&server)
            .await;
    }

    for status in [200, 401, 404, 500] {
        let result = PropFindBuilder::new(Client::new(), base_url.clone())
            .path(format!("status-{status}"))
            .send_and_deserialize()
            .await;
        assert!(
            matches!(result, Err(PropFindError::UnexpectedStatus(actual)) if actual.as_u16() == status)
        );
    }
}

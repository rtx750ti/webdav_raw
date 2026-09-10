#[path = "../../../common/local_http.rs"]
mod local_http;

use webdav_core::Client;
use webdav_core::propfind::builder::{Depth, PropFindBuilder};
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, ResponseTemplate};

/// 验证 allprop 请求的路径、Depth、请求头和 XML 请求体。
#[tokio::test]
async fn allprop_request_matches_webdav_contract() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    Mock::given(method("PROPFIND"))
        .and(path("/dav/Documents"))
        .and(header("Depth", "1"))
        .and(header("Content-Type", "application/xml; charset=utf-8"))
        .and(body_string(
            r#"<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#,
        ))
        .respond_with(ResponseTemplate::new(207))
        .expect(1)
        .mount(&server)
        .await;

    let response = PropFindBuilder::new(Client::new(), base_url)
        .path("Documents")
        .depth(Depth::One)
        .allprop()
        .send()
        .await
        .expect("请求应发送成功");

    assert_eq!(response.status(), 207);
}

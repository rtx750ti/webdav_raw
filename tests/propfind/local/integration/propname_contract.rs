use webdav_raw::Client;
use webdav_raw::{Depth, PropFindBuilder};
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 验证 propname 请求携带正确的选择器 XML。
#[tokio::test]
async fn propname_request_matches_webdav_contract() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    Mock::given(method("PROPFIND"))
        .and(path("/dav/"))
        .and(header("Depth", "0"))
        .and(body_string(
            r#"<D:propfind xmlns:D="DAV:"><D:propname/></D:propfind>"#,
        ))
        .respond_with(ResponseTemplate::new(207))
        .expect(1)
        .mount(&server)
        .await;

    let response = PropFindBuilder::new(Client::new(), base_url)
        .depth(Depth::Zero)
        .prop_name()
        .send()
        .await
        .expect("请求应发送成功");

    assert_eq!(response.status(), 207);
}

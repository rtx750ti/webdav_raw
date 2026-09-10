#[path = "../../../common/local_http.rs"]
mod local_http;

use webdav_core::Client;
use webdav_core::propfind::builder::PropFindBuilder;
use webdav_core::propfind::find_props::FindProp;
use wiremock::matchers::{body_string, method, path};
use wiremock::{Mock, ResponseTemplate};

/// 验证指定属性会去重并以稳定顺序发送到服务器。
#[tokio::test]
async fn properties_request_deduplicates_and_orders_xml() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    Mock::given(method("PROPFIND"))
        .and(path("/dav/metadata"))
        .and(body_string(r#"<D:propfind xmlns:D="DAV:"><D:prop><D:resourcetype/><D:getetag/><D:getcontenttype/></D:prop></D:propfind>"#))
        .respond_with(ResponseTemplate::new(207))
        .expect(1)
        .mount(&server)
        .await;

    let response = PropFindBuilder::new(Client::new(), base_url)
        .path("metadata")
        .props([
            FindProp::Getetag,
            FindProp::Getcontenttype,
            FindProp::Getetag,
            FindProp::Resourcetype,
        ])
        .send()
        .await
        .expect("请求应发送成功");

    assert_eq!(response.status(), 207);
}

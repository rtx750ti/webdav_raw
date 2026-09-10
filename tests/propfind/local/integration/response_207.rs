#[path = "../../../common/local_http.rs"]
mod local_http;

use webdav_core::Client;
use webdav_core::propfind::builder::{Depth, PropFindBuilder};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::fixtures::RAW_RESULT_1;

/// 验证 207 响应会解析为完整且有序的 MultiStatus 结果。
#[tokio::test]
async fn multi_status_response_deserializes_complete_response_list() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    Mock::given(method("PROPFIND"))
        .and(path("/dav/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(RAW_RESULT_1))
        .expect(1)
        .mount(&server)
        .await;

    let multistatus = PropFindBuilder::new(Client::new(), base_url)
        .depth(Depth::One)
        .send_and_deserialize()
        .await
        .expect("207 响应应解析成功");

    assert_eq!(multistatus.response.len(), 5);
    assert_eq!(
        multistatus.response.front().expect("应包含根资源").href,
        "/dav/"
    );
    assert_eq!(
        multistatus.response.get(2).expect("应包含文件资源").href,
        "/dav/test.txt"
    );
}

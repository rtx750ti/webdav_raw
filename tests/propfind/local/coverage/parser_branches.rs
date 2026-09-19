//! 覆盖率补点：PROPFIND 响应体解析失败的分支。
//!
//! 对应 `cargo llvm-cov` 报告（`docs/coverage/html`）中的红点：
//! `src/propfind/builder.rs::send_and_deserialize` 的 `MultiStatus::from_str(&body)?`
//! 错误传播区域。既有本地集成测试只覆盖「207 + 合法 XML」，这条线路必须由真实回包
//! 触发，因此按测试规范放在 `local/coverage`，而不是重写已经命中的正常场景。

use webdav_raw::Client;
use webdav_raw::{PropFindBuilder, PropFindError};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 207 但响应体不是合法 XML 时返回反序列化错误。
#[tokio::test]
async fn malformed_multi_status_body_returns_deserialization_error() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    Mock::given(method("PROPFIND"))
        .and(path("/dav/broken.xml"))
        .respond_with(ResponseTemplate::new(207).set_body_string("<not-xml"))
        .expect(1)
        .mount(&server)
        .await;

    let error = PropFindBuilder::new(Client::new(), base_url)
        .path("broken.xml")
        .send_and_deserialize()
        .await
        .expect_err("非法 XML 应报反序列化错误");

    assert!(
        matches!(error, PropFindError::DeError(_)),
        "应为 DeError，实际: {error:?}"
    );
}

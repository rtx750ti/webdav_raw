//! 覆盖率补点：MOVE 读取响应体失败的分支。
//!
//! 对应 `cargo llvm-cov` 报告（`docs/coverage/html`）中的红点：
//! `src/move/builder.rs::send_and_deserialize` 的 `response.text().await?` 错误传播
//! 区域。既有集成测试覆盖了 207 正常解析、非 207 状态码和非法 XML，但「响应体本身
//! 读不出来」这条线路没走过，它必须由真实回包触发。

use webdav_raw::{MoveError, WebdavAuth};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 服务端谎报 `Content-Encoding: gzip` 时返回请求错误。
#[tokio::test]
async fn misreported_gzip_content_encoding_returns_request_error() {
    let server = local_http::start_server().await;
    Mock::given(method("MOVE"))
        .and(path("/dav/source.txt"))
        .respond_with(
            ResponseTemplate::new(207)
                .insert_header("content-encoding", "gzip")
                .set_body_string("<D:multistatus xmlns:D=\"DAV:\"></D:multistatus>"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let error = auth
        .mv()
        .source_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .send_and_deserialize()
        .await
        .expect_err("谎报 gzip 的响应体应报请求错误");

    assert!(
        matches!(error, MoveError::Request(_)),
        "应为 Request，实际: {error:?}"
    );
}

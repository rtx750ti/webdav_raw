//! 覆盖率补点：PROPFIND 请求链路上未命中的错误分支。
//!
//! 对应 `cargo llvm-cov` 报告（`docs/coverage/html`）中的红点：
//!
//! - `src/propfind/builder.rs::send` 的 `self.client.execute(request).await?`：
//!   请求发送失败的 `Err` 传播区域。
//! - `src/propfind/builder.rs::send_and_deserialize` 的
//!   `self.client.execute(request).await?` 与 `response.text().await?`：
//!   发送失败、响应体解码失败的 `Err` 传播区域。
//!
//! 既有 PROPFIND 测试只打 MockServer 的正常与状态码场景，没有「连不上」和「响应体
//! 读不出来」两条线路。这两个缺口都需要真实 socket 行为，因此放在 `local/coverage`。

use webdav_raw::{Client, PropFindBuilder, PropFindError, Url};

use crate::common::local_http;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

/// 端口 1 上没有监听者，`send()` 把底层连接错误原样交出。
#[tokio::test]
async fn send_returns_request_error_when_server_is_unreachable() {
    let target = Url::parse("http://127.0.0.1:1/dav/").expect("地址必须合法");

    let error = PropFindBuilder::new(Client::new(), target)
        .path("missing/")
        .send()
        .await
        .expect_err("不可达地址应报请求错误");

    assert!(
        matches!(error, PropFindError::Request(_)),
        "应为 Request，实际: {error:?}"
    );
}

/// 同一条不可达线路在 `send_and_deserialize()` 上同样报请求错误。
#[tokio::test]
async fn send_and_deserialize_reports_request_error_when_server_is_unreachable() {
    let target = Url::parse("http://127.0.0.1:1/dav/").expect("地址必须合法");

    let error = PropFindBuilder::new(Client::new(), target)
        .path("missing/")
        .send_and_deserialize()
        .await
        .expect_err("不可达地址应报请求错误");

    assert!(
        matches!(error, PropFindError::Request(_)),
        "应为 Request，实际: {error:?}"
    );
}

/// 根地址无法拼接相对路径时，两个发送入口都在构建阶段报 `Url` 错误。
///
/// PROPFIND 是唯一在 `build()` 里才解析请求 URL 的领域（其余领域的 setter 已经拼好
/// 地址），因此 `self.build()?` 的错误传播线路只有这里能被真实命中：`data:` URL 属于
/// 「不能作为基地址」的地址，`join` 必然失败。
#[tokio::test]
async fn send_reports_url_error_when_base_url_cannot_be_joined() {
    let base = Url::parse("data:text/plain,hello").expect("data URL 应可解析");

    let send_error = PropFindBuilder::new(Client::new(), base.clone())
        .path("folder")
        .send()
        .await
        .expect_err("不能拼接相对路径的根地址应报 URL 错误");
    let deserialize_error = PropFindBuilder::new(Client::new(), base)
        .path("folder")
        .send_and_deserialize()
        .await
        .expect_err("不能拼接相对路径的根地址应报 URL 错误");

    assert!(
        matches!(send_error, PropFindError::Url(_)),
        "send 应为 Url，实际: {send_error:?}"
    );
    assert!(
        matches!(deserialize_error, PropFindError::Url(_)),
        "send_and_deserialize 应为 Url，实际: {deserialize_error:?}"
    );
}

/// 服务端谎报 `Content-Encoding: gzip` 时，读取响应体失败并交回请求错误。
///
/// 响应头声明 gzip、字节却不是 gzip：`response.text()` 在解码阶段失败。本库把它
/// 作为 `PropFindError::Request` 原样上交，既不 panic 也不吞掉错误。
#[tokio::test]
async fn misreported_gzip_content_encoding_returns_request_error() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    Mock::given(method("PROPFIND"))
        .and(path("/dav/lying-gzip.xml"))
        .respond_with(
            ResponseTemplate::new(207)
                .insert_header("content-encoding", "gzip")
                .set_body_string("<D:multistatus xmlns:D=\"DAV:\"></D:multistatus>"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let error = PropFindBuilder::new(Client::new(), base_url)
        .path("lying-gzip.xml")
        .send_and_deserialize()
        .await
        .expect_err("谎报 gzip 的响应体应报请求错误");

    assert!(
        matches!(error, PropFindError::Request(_)),
        "应为 Request，实际: {error:?}"
    );
}

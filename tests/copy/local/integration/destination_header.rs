//! `Destination` 头在完整 HTTP 链路里的取值。
//!
//! 单元测试验证的是构建出来的请求；这里验证服务端**实际收到**的取值，
//! 包括中文与空格经过 HTTP 之后仍然是编码后的形态。

use webdav_core::{Overwrite, WebdavAuth};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 服务端收到的 `Destination` 是带 scheme 与 authority 的完整 URI。
#[tokio::test]
async fn destination_is_absolute_uri_from_server_point_of_view() {
    let server = local_http::start_server().await;
    let uri = server.uri();
    let expected = format!("{uri}/dav/release/report.pdf");

    Mock::given(method("COPY"))
        .and(path("/dav/staging/report.pdf"))
        .and(header("destination", expected.as_str()))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let response = auth
        .copy()
        .source_path("staging/report.pdf")
        .expect("合法相对路径应被接受")
        .target_path("release/report.pdf")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("COPY 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 中文与空格在服务端收到的 `Destination` 里是百分号编码的形态。
#[tokio::test]
async fn destination_is_percent_encoded_on_the_wire() {
    let server = local_http::start_server().await;
    let uri = server.uri();
    let expected = format!(
        "{uri}/dav/%E5%A4%87%E4%BB%BD/%E6%8A%A5%E5%91%8A%202026.pdf"
    );

    Mock::given(method("COPY"))
        .and(path("/dav/source.pdf"))
        .and(header("destination", expected.as_str()))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let response = auth
        .copy()
        .source_path("source.pdf")
        .expect("合法相对路径应被接受")
        .target_path("备份/报告 2026.pdf")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("COPY 应发送成功");

    assert_eq!(response.status(), 201);
}

/// `target_url` 指向另一台服务时，`Destination` 就是那个完整地址。
#[tokio::test]
async fn absolute_target_is_sent_verbatim() {
    let server = local_http::start_server().await;
    let other = local_http::start_server().await;
    let expected = format!("{}/remote/copy.txt", other.uri());

    Mock::given(method("COPY"))
        .and(path("/dav/source.txt"))
        .and(header("destination", expected.as_str()))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let response = auth
        .copy()
        .source_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_url(&expected)
        .expect("合法绝对地址应被接受")
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("COPY 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 同一份请求里 `Destination` 与 `Overwrite` 同时正确。
#[tokio::test]
async fn destination_and_overwrite_are_sent_together() {
    let server = local_http::start_server().await;
    let uri = server.uri();
    let expected = format!("{uri}/dav/target.txt");

    Mock::given(method("COPY"))
        .and(path("/dav/source.txt"))
        .and(header("destination", expected.as_str()))
        .and(header("overwrite", "F"))
        .and(header("depth", "0"))
        .respond_with(ResponseTemplate::new(412))
        .expect(1)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let response = auth
        .copy()
        .source_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .overwrite(Overwrite::False)
        .depth(webdav_core::CopyDepth::Zero)
        .send()
        .await
        .expect("COPY 应发送成功");

    assert_eq!(response.status(), 412);
}

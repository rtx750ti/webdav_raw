//! MOVE 请求契约：方法、源地址、目标地址与请求头。

use webdav_core::{MoveDepth, Overwrite, WebdavAuth};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 认证对象创建的 MOVE 请求：方法、源路径与默认 Authorization 都正确。
#[tokio::test]
async fn authenticated_move_uses_auth_root_and_method() {
    let server = local_http::start_server().await;
    Mock::given(method("MOVE"))
        .and(path("/dav/staging/report.pdf"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
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
        .mv()
        .move_from_path("staging/report.pdf")
        .expect("合法相对路径应被接受")
        .target_path("archive/report.pdf")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MOVE 应发送成功");

    assert_eq!(response.status(), 201);
}

/// `source_path` 与 `move_from_path` 都能设置源，且行为一致。
#[tokio::test]
async fn both_source_setters_reach_the_same_place() {
    let server = local_http::start_server().await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    for source in ["via-source-path.txt", "via-move-from-path.txt"] {
        Mock::given(method("MOVE"))
            .and(path(format!("/dav/{source}")))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&server)
            .await;

        let builder = auth.mv();
        let builder = if source.starts_with("via-source") {
            builder.source_path(source).expect("合法相对路径应被接受")
        } else {
            builder
                .move_from_path(source)
                .expect("合法相对路径应被接受")
        };

        let response = builder
            .target_path("target.txt")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("MOVE 应发送成功");

        assert_eq!(response.status(), 201);
    }
}

/// 默认 `Depth` 是 infinity，随请求到达服务端。
#[tokio::test]
async fn default_depth_reaches_server() {
    let server = local_http::start_server().await;
    Mock::given(method("MOVE"))
        .and(path("/dav/a.txt"))
        .and(header("depth", "infinity"))
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
        .mv()
        .move_from_path("a.txt")
        .expect("合法相对路径应被接受")
        .target_path("b.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MOVE 应发送成功");

    assert_eq!(response.status(), 201);
}

/// `MoveDepth::Zero` 与 `One` 如实到达服务端。
#[tokio::test]
async fn explicit_depths_reach_server() {
    let server = local_http::start_server().await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    for (depth, expected) in [(MoveDepth::Zero, "0"), (MoveDepth::One, "1")] {
        let source = format!("dir-{expected}/");
        Mock::given(method("MOVE"))
            .and(path(format!("/dav/{source}")))
            .and(header("depth", expected))
            .respond_with(ResponseTemplate::new(201))
            .expect(1)
            .mount(&server)
            .await;

        let response = auth
            .mv()
            .move_from_path(&source)
            .expect("合法相对路径应被接受")
            .target_path("target/")
            .expect("合法相对路径应被接受")
            .depth(depth)
            .send()
            .await
            .expect("MOVE 应发送成功");

        assert_eq!(response.status(), 201);
    }
}

/// 默认不发 `Overwrite` 头。
#[tokio::test]
async fn default_request_omits_overwrite_header() {
    let server = local_http::start_server().await;
    let expected = format!("{}/dav/b.txt", server.uri());

    Mock::given(method("MOVE"))
        .and(path("/dav/a.txt"))
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
        .mv()
        .move_from_path("a.txt")
        .expect("合法相对路径应被接受")
        .target_path("b.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MOVE 应发送成功");

    assert_eq!(response.status(), 201);
}

/// `Overwrite::False` 时 `Overwrite: F` 到达服务端。
#[tokio::test]
async fn overwrite_false_reaches_server() {
    let server = local_http::start_server().await;
    Mock::given(method("MOVE"))
        .and(path("/dav/a.txt"))
        .and(header("overwrite", "F"))
        .respond_with(ResponseTemplate::new(204))
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
        .mv()
        .move_from_path("a.txt")
        .expect("合法相对路径应被接受")
        .target_path("b.txt")
        .expect("合法相对路径应被接受")
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("MOVE 应发送成功");

    assert_eq!(response.status(), 204);
}

/// 调用方设置的自定义请求头原样到达服务端。
#[tokio::test]
async fn caller_set_header_reaches_server() {
    let server = local_http::start_server().await;
    Mock::given(method("MOVE"))
        .and(path("/dav/a.txt"))
        .and(header("if-match", "\"v1\""))
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
        .mv()
        .move_from_path("a.txt")
        .expect("合法相对路径应被接受")
        .target_path("b.txt")
        .expect("合法相对路径应被接受")
        .header("if-match", "\"v1\"")
        .expect("合法请求头应被接受")
        .send()
        .await
        .expect("MOVE 应发送成功");

    assert_eq!(response.status(), 201);
}

/// `source_url` 换的是地址，不是 Client：默认 Authorization 也跟着走。
#[tokio::test]
async fn absolute_source_sends_default_authorization_to_another_origin() {
    let auth_server = local_http::start_server().await;
    let other_server = local_http::start_server().await;

    Mock::given(method("MOVE"))
        .and(path("/source.txt"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&other_server)
        .await;

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&auth_server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let response = auth
        .mv()
        .source_url(&format!("{}/source.txt", other_server.uri()))
        .expect("合法绝对地址应被接受")
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("跨服务 MOVE 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 中文与空格在服务端收到的 `Destination` 里是百分号编码的形态。
#[tokio::test]
async fn destination_is_percent_encoded_on_the_wire() {
    let server = local_http::start_server().await;
    let expected = format!(
        "{}/dav/%E5%BD%92%E6%A1%A3/%E6%8A%A5%E5%91%8A%202026.pdf",
        server.uri()
    );

    Mock::given(method("MOVE"))
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
        .mv()
        .move_from_path("source.pdf")
        .expect("合法相对路径应被接受")
        .target_path("归档/报告 2026.pdf")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MOVE 应发送成功");

    assert_eq!(response.status(), 201);
}

/// 以斜杠开头的相对源路径覆盖认证根地址中已有的前缀。
#[tokio::test]
async fn leading_slash_source_replaces_auth_root_prefix() {
    let server = local_http::start_server().await;
    Mock::given(method("MOVE"))
        .and(path("/other/source.txt"))
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
        .mv()
        .move_from_path("/other/source.txt")
        .expect("合法相对路径应被接受")
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MOVE 应发送成功");

    assert_eq!(response.status(), 201);
}

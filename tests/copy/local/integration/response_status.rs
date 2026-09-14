//! 状态码行为：COPY 的成功与失败码原样透传，207 可另行解析。

use webdav_core::{CopyError, WebdavAuth};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 逐码验证 COPY 的状态都原样返回。
#[tokio::test]
async fn copy_status_codes_are_passed_through_unchanged() {
    let server = local_http::start_server().await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let codes = [200u16, 201, 204, 400, 401, 403, 404, 409, 412, 423, 500, 502, 507];

    for code in codes {
        let source = format!("code-{code}.txt");
        Mock::given(method("COPY"))
            .and(path(format!("/dav/{source}")))
            .respond_with(ResponseTemplate::new(code))
            .expect(1)
            .mount(&server)
            .await;

        let response = auth
            .copy()
            .source_path(&source)
            .expect("合法相对路径应被接受")
            .target_path("target.txt")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("COPY 应发送成功");

        assert_eq!(response.status().as_u16(), code, "{code} 应原样返回");
    }
}

/// 412（目标已存在且不允许覆盖）时响应头仍然可读。
#[tokio::test]
async fn precondition_failed_keeps_response_headers() {
    let server = local_http::start_server().await;
    Mock::given(method("COPY"))
        .and(path("/dav/source.txt"))
        .respond_with(ResponseTemplate::new(412).insert_header("x-reason", "target exists"))
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
        .overwrite(webdav_core::Overwrite::False)
        .send()
        .await
        .expect("COPY 应发送成功");

    assert_eq!(response.status(), 412);
    assert_eq!(response.headers().get("x-reason").unwrap(), "target exists");
}

/// 207 能被解析成 `MultiStatus`。
///
/// 固件里每个 `<propstat>` 都带 `<prop/>`（部分失败时是空元素），这是
/// `send_and_deserialize` 的解析前提，见该方法的文档。
#[tokio::test]
async fn multi_status_is_deserialized() {
    let server = local_http::start_server().await;
    Mock::given(method("COPY"))
        .and(path("/dav/dir/"))
        .respond_with(
            ResponseTemplate::new(207).set_body_string(MULTI_STATUS_BODY),
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

    let multistatus = auth
        .copy()
        .source_path("dir/")
        .expect("合法相对路径应被接受")
        .target_path("dir-copy/")
        .expect("合法相对路径应被接受")
        .send_and_deserialize()
        .await
        .expect("207 应能解析");

    assert_eq!(multistatus.response.len(), 2, "两条 response 都应保留");
    assert!(
        multistatus
            .response
            .iter()
            .any(|item| item.href.contains("locked.txt")),
        "部分失败的条目应保留在结果里"
    );
}

/// 服务端省略 `<prop>` 时，整个响应解析失败——这是已验证的已知限制。
///
/// 反序列化复用 `propfind` 领域的 [`MultiStatus`]，它的 `propstat.prop` 是必填
/// 字段。本库不为此另写一套解析器（那会形成两套 `MultiStatus` 语义），因此把
/// 真实行为固定在这里：这种响应要用 `send()` 拿原始响应自行处理。
#[tokio::test]
async fn multi_status_without_prop_element_is_reported_as_deserialization_error() {
    let server = local_http::start_server().await;
    // 同一个路径会被请求两次：一次走解析入口，一次走原始入口。
    Mock::given(method("COPY"))
        .and(path("/dav/dir/"))
        .respond_with(
            ResponseTemplate::new(207).set_body_string(MULTI_STATUS_WITHOUT_PROP_BODY),
        )
        .expect(2)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let error = auth
        .copy()
        .source_path("dir/")
        .expect("合法相对路径应被接受")
        .target_path("dir-copy/")
        .expect("合法相对路径应被接受")
        .send_and_deserialize()
        .await
        .expect_err("缺 <prop> 时应报反序列化错误");

    assert!(
        matches!(error, CopyError::De(_)),
        "应为 De 错误，实际: {error:?}"
    );

    // 原始入口仍然可用：`href` 与状态码都在响应体里，调用方能自己读。
    let raw = auth
        .copy()
        .source_path("dir/")
        .expect("合法相对路径应被接受")
        .target_path("dir-copy/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("原始入口应发送成功");
    assert_eq!(raw.status(), 207);
    let body = raw.text().await.expect("响应体应可读");
    assert!(
        body.contains("423 Locked"),
        "状态码信息应仍可从原始响应体取到"
    );
}

/// 非 207 状态下 `send_and_deserialize()` 报 `UnexpectedStatus`。
#[tokio::test]
async fn non_multi_status_returns_unexpected_status() {
    let server = local_http::start_server().await;
    Mock::given(method("COPY"))
        .and(path("/dav/source.txt"))
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

    let error = auth
        .copy()
        .source_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .send_and_deserialize()
        .await
        .expect_err("201 应被报为意外状态");

    match error {
        CopyError::UnexpectedStatus(status) => assert_eq!(status, 201),
        other => panic!("应为 UnexpectedStatus，实际: {other:?}"),
    }
}

/// 207 但响应体不是合法 XML 时报反序列化错误。
#[tokio::test]
async fn malformed_multi_status_body_returns_deserialization_error() {
    let server = local_http::start_server().await;
    Mock::given(method("COPY"))
        .and(path("/dav/source.txt"))
        .respond_with(ResponseTemplate::new(207).set_body_string("<not-xml"))
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
        .copy()
        .source_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .send_and_deserialize()
        .await
        .expect_err("非法 XML 应报反序列化错误");

    assert!(matches!(error, CopyError::De(_)));
}

/// 207 响应体固件：一条成功、一条部分失败，每个 `<propstat>` 都带 `<prop/>`。
const MULTI_STATUS_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/dav/dir/copied.txt</D:href>
    <D:propstat>
      <D:prop/>
      <D:status>HTTP/1.1 201 Created</D:status>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/dav/dir/locked.txt</D:href>
    <D:propstat>
      <D:prop/>
      <D:status>HTTP/1.1 423 Locked</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;

/// 省略 `<prop>` 的 207 固件，用于固定已知限制的真实行为。
const MULTI_STATUS_WITHOUT_PROP_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/dav/dir/locked.txt</D:href>
    <D:propstat>
      <D:status>HTTP/1.1 423 Locked</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;

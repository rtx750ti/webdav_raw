//! 状态码行为：MOVE 的成功与失败码原样透传，207 可另行解析。

use webdav_raw::{MoveError, WebdavAuth};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 逐码验证 MOVE 的状态都原样返回。
#[tokio::test]
async fn move_status_codes_are_passed_through_unchanged() {
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
        Mock::given(method("MOVE"))
            .and(path(format!("/dav/{source}")))
            .respond_with(ResponseTemplate::new(code))
            .expect(1)
            .mount(&server)
            .await;

        let response = auth
            .mv()
            .move_from_path(&source)
            .expect("合法相对路径应被接受")
            .target_path("target.txt")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("MOVE 应发送成功");

        assert_eq!(response.status().as_u16(), code, "{code} 应原样返回");
    }
}

/// 403（例如目标是源的祖先）时响应头仍然可读。
#[tokio::test]
async fn forbidden_keeps_response_headers() {
    let server = local_http::start_server().await;
    Mock::given(method("MOVE"))
        .and(path("/dav/dir/"))
        .respond_with(ResponseTemplate::new(403).insert_header("x-reason", "destination inside source"))
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
        .move_from_path("dir/")
        .expect("合法相对路径应被接受")
        .target_path("dir/inner/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("MOVE 应发送成功");

    assert_eq!(response.status(), 403);
    assert_eq!(
        response.headers().get("x-reason").unwrap(),
        "destination inside source"
    );
}

/// 207 能被解析成 `MultiStatus`。
#[tokio::test]
async fn multi_status_is_deserialized() {
    let server = local_http::start_server().await;
    Mock::given(method("MOVE"))
        .and(path("/dav/dir/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(MULTI_STATUS_BODY))
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
        .mv()
        .move_from_path("dir/")
        .expect("合法相对路径应被接受")
        .target_path("dir-moved/")
        .expect("合法相对路径应被接受")
        .send_and_deserialize()
        .await
        .expect("207 应能解析");

    assert_eq!(multistatus.response.len(), 2, "两条 response 都应保留");
}

/// 非 207 状态下 `send_and_deserialize()` 报 `UnexpectedStatus`。
#[tokio::test]
async fn non_multi_status_returns_unexpected_status() {
    let server = local_http::start_server().await;
    Mock::given(method("MOVE"))
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
        .mv()
        .move_from_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .send_and_deserialize()
        .await
        .expect_err("201 应被报为意外状态");

    match error {
        MoveError::UnexpectedStatus(status) => assert_eq!(status, 201),
        other => panic!("应为 UnexpectedStatus，实际: {other:?}"),
    }
}

/// 服务端省略 `<prop>` 时也能解析：`href` 与状态码都要能拿到。
///
/// `propstat.prop` 现在有 `#[serde(default)]`，与 COPY 侧同源修复。
/// 这条用例曾经断言「解析应失败」，补上默认值后行为反转。
#[tokio::test]
async fn multi_status_without_prop_element_is_still_parseable() {
    let server = local_http::start_server().await;
    Mock::given(method("MOVE"))
        .and(path("/dav/dir/"))
        .respond_with(
            ResponseTemplate::new(207).set_body_string(MULTI_STATUS_WITHOUT_PROP_BODY),
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
        .mv()
        .move_from_path("dir/")
        .expect("合法相对路径应被接受")
        .target_path("dir-moved/")
        .expect("合法相对路径应被接受")
        .send_and_deserialize()
        .await
        .expect("缺 <prop> 的 207 也应能解析");

    let response = multistatus.response.front().expect("应保留失败条目");
    assert_eq!(response.href, "/dav/dir/locked.txt");
    let propstat = response.propstat.first().expect("应有一条 propstat");
    assert_eq!(propstat.status, "HTTP/1.1 423 Locked");
    assert!(
        propstat.prop.etag.is_none() && propstat.prop.resource_type.is_none(),
        "省略 <prop> 时属性应全为空"
    );
}

/// 207 响应体固件：一条成功、一条部分失败，每个 `<propstat>` 都带 `<prop/>`。
const MULTI_STATUS_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/dav/dir/moved.txt</D:href>
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

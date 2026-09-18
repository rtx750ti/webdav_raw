//! 状态码透传：本库不对任何状态码做解释。

use webdav_raw::WebdavAuth;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;
use crate::support::fixtures::bytes_body;

/// 服务端错误状态原样返回，本库不做过滤。
#[tokio::test]
async fn server_error_statuses_are_returned_without_library_filtering() {
    let server = local_http::start_server().await;
    let statuses = [400, 401, 403, 404, 409, 412, 500];

    for status in statuses {
        Mock::given(method("PUT"))
            .and(path(format!("/dav/status-{status}")))
            .respond_with(ResponseTemplate::new(status).set_body_string("server-error"))
            .mount(&server)
            .await;
    }

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    for status in statuses {
        let response = auth
            .put()
            .relative_path(&format!("status-{status}"))
            .expect("相对路径必须有效")
            .body(bytes_body(b"x".to_vec()))
            .send()
            .await
            .expect("PUT 应发送成功");

        assert_eq!(response.status().as_u16(), status);
        assert_eq!(response.text().await.unwrap(), "server-error");
    }
}

/// 成功状态原样返回：本库不把任何状态码当作"成功"来吞掉。
#[tokio::test]
async fn success_statuses_are_returned_verbatim() {
    let server = local_http::start_server().await;
    let statuses = [200, 201, 204];

    for status in statuses {
        Mock::given(method("PUT"))
            .and(path(format!("/dav/ok-{status}")))
            .respond_with(ResponseTemplate::new(status))
            .mount(&server)
            .await;
    }

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    for status in statuses {
        let response = auth
            .put()
            .relative_path(&format!("ok-{status}"))
            .expect("相对路径必须有效")
            .body(bytes_body(b"x".to_vec()))
            .send()
            .await
            .expect("PUT 应发送成功");

        assert_eq!(response.status().as_u16(), status);
    }
}

/// 分片源请求得到非 206 时，本库不会自行判定为"部分上传成功"。
///
/// 标准 WebDAV 不支持部分上传，服务端忽略 `Content-Range` 时会回 200/201，
/// 与"支持并成功"无法区分。本库的策略是不发该头、也不对状态码做任何解释，
/// 因此这里收到什么就返回什么。
#[tokio::test]
async fn chunk_put_status_is_passed_through_uninterpreted() {
    let server = local_http::start_server().await;
    Mock::given(method("PUT"))
        .and(path("/dav/chunk.bin"))
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
        .put()
        .relative_path("chunk.bin")
        .expect("相对路径必须有效")
        .body(crate::support::fixtures::bytes_body(b"chunk".to_vec()))
        .send()
        .await
        .expect("PUT 应发送成功");

    assert_eq!(
        response.status(),
        201,
        "本库不对状态码做判断，原样交给调用方"
    );
}

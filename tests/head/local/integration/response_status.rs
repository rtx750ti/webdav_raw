//! 状态码行为：服务端的判定原样透传，库不改成自己的错误。

use webdav_raw::WebdavAuth;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 逐码验证 HEAD 的成功与失败状态都原样返回。
#[tokio::test]
async fn head_status_codes_are_passed_through_unchanged() {
    let server = local_http::start_server().await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let codes = [200u16, 204, 301, 304, 400, 401, 403, 404, 405, 500];

    for code in codes {
        let target = format!("code-{code}.txt");
        Mock::given(method("HEAD"))
            .and(path(format!("/dav/{target}")))
            .respond_with(ResponseTemplate::new(code))
            .expect(1)
            .mount(&server)
            .await;

        let response = auth
            .head()
            .target_path(&target)
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("HEAD 应发送成功");

        assert_eq!(response.status().as_u16(), code, "{code} 应原样返回");
    }
}

/// 404 时响应头仍然可读，调用方可以据此区分「不存在」和其他失败。
#[tokio::test]
async fn not_found_keeps_response_headers() {
    let server = local_http::start_server().await;
    Mock::given(method("HEAD"))
        .and(path("/dav/missing.txt"))
        .respond_with(ResponseTemplate::new(404).insert_header("x-reason", "no such resource"))
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
        .head()
        .target_path("missing.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");

    assert_eq!(response.status(), 404);
    assert_eq!(response.headers().get("x-reason").unwrap(), "no such resource");
}

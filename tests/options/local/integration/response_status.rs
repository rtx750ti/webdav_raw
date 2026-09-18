//! 状态码行为：服务端的判定原样透传，库不改成自己的错误。

use webdav_raw::WebdavAuth;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 逐码验证 OPTIONS 的状态都原样返回。
#[tokio::test]
async fn options_status_codes_are_passed_through_unchanged() {
    let server = local_http::start_server().await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let codes = [200u16, 204, 400, 401, 403, 404, 405, 500, 501];

    for code in codes {
        let target = format!("code-{code}/");
        Mock::given(method("OPTIONS"))
            .and(path(format!("/dav/{target}")))
            .respond_with(ResponseTemplate::new(code))
            .expect(1)
            .mount(&server)
            .await;

        let response = auth
            .options()
            .target_path(&target)
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("OPTIONS 应发送成功");

        assert_eq!(response.status().as_u16(), code, "{code} 应原样返回");
    }
}

/// 非成功状态码下响应头仍然可读，调用方可以据此判断原因。
#[tokio::test]
async fn error_status_keeps_response_headers() {
    let server = local_http::start_server().await;
    Mock::given(method("OPTIONS"))
        .and(path("/dav/forbidden/"))
        .respond_with(ResponseTemplate::new(403).insert_header("x-reason", "denied"))
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
        .options()
        .target_path("forbidden/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("OPTIONS 应发送成功");

    assert_eq!(response.status(), 403);
    assert_eq!(response.headers().get("x-reason").unwrap(), "denied");
}

/// 失败响应上解析能力也不报错：读不到就是空列表。
#[tokio::test]
async fn capabilities_on_error_response_are_empty_not_an_error() {
    let server = local_http::start_server().await;
    Mock::given(method("OPTIONS"))
        .and(path("/dav/denied/"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let (response, caps) = auth
        .options()
        .target_path("denied/")
        .expect("合法相对路径应被接受")
        .send_capabilities()
        .await
        .expect("OPTIONS 应发送成功，状态码不当作错误");

    assert_eq!(response.status(), 401);
    assert!(caps.dav_levels.is_empty());
    assert!(caps.allowed_methods.is_empty());
}

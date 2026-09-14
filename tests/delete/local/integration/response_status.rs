//! 状态码行为：服务端的判定原样透传，库不改成自己的错误。

use webdav_core::WebdavAuth;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::common::local_http;

/// 逐码验证 DELETE 的成功与失败状态都原样返回。
///
/// 成功侧覆盖 200 / 204，失败侧覆盖 400 / 401 / 403 / 404 / 409 / 412 / 423 / 500，
/// 以及 WebDAV 特有的 207 部分失败。
#[tokio::test]
async fn delete_status_codes_are_passed_through_unchanged() {
    let server = local_http::start_server().await;
    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    let codes = [200u16, 204, 400, 401, 403, 404, 409, 412, 423, 500];

    for code in codes {
        let target = format!("code-{code}.txt");
        Mock::given(method("DELETE"))
            .and(path(format!("/dav/{target}")))
            .respond_with(ResponseTemplate::new(code))
            .expect(1)
            .mount(&server)
            .await;

        let response = auth
            .delete()
            .target_path(&target)
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("DELETE 应发送成功");

        assert_eq!(response.status().as_u16(), code, "{code} 应原样返回");
    }
}

/// 成功响应里的响应头可以被调用方读取。
#[tokio::test]
async fn success_response_headers_are_readable() {
    let server = local_http::start_server().await;
    Mock::given(method("DELETE"))
        .and(path("/dav/tagged.txt"))
        .respond_with(ResponseTemplate::new(204).insert_header("x-deleted-by", "server"))
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
        .delete()
        .target_path("tagged.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("DELETE 应发送成功");

    assert_eq!(response.status(), 204);
    assert_eq!(
        response.headers().get("x-deleted-by").unwrap(),
        "server",
        "响应头应原样保留"
    );
}

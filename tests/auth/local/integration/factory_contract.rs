#[path = "../../../common/local_http.rs"]
mod local_http;

use webdav_core::auth::WebdavAuth;
use webdav_core::propfind::builder::Depth;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

/// 验证认证对象创建的 GET 和 PROPFIND Builder 复用根地址与客户端配置。
#[tokio::test]
async fn factories_use_auth_base_url_and_client() {
    let server = local_http::start_server().await;
    let base_url = local_http::base_url(&server, "dav");
    Mock::given(method("GET"))
        .and(path("/dav/"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/dav/folder"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(207))
        .expect(1)
        .mount(&server)
        .await;
    let auth =
        WebdavAuth::new("alice", "password", base_url.as_str()).expect("回环地址应创建认证对象");

    let get_response = auth.get().send().await.expect("GET 请求应发送成功");
    let propfind_response = auth
        .propfind()
        .path("folder")
        .depth(Depth::One)
        .send()
        .await
        .expect("PROPFIND 请求应发送成功");

    assert_eq!(get_response.status(), 200);
    assert_eq!(propfind_response.status(), 207);
}

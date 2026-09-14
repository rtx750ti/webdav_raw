//! 删除单个资源的完整流程：请求发出后服务端真的查不到它了。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use webdav_core::{DeleteDepth, WebdavAuth};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, Request, Respond, ResponseTemplate};

use crate::common::local_http;

/// 按调用次序应答：第一次给 `first`，之后都给 `rest`。
///
/// 用来模拟「删除前存在、删除后不存在」这一个路径上的状态变化。wiremock
/// 不保证同路径多份 mock 的匹配顺序，因此不能靠挂两份 mock 来表达先后。
struct ThenAlways {
    calls: Arc<AtomicUsize>,
    first: u16,
    rest: u16,
}

impl Respond for ThenAlways {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let nth = self.calls.fetch_add(1, Ordering::SeqCst);

        ResponseTemplate::new(if nth == 0 { self.first } else { self.rest })
    }
}

/// 删除文件前后用 GET 核对：删之前存在，删之后查不到。
///
/// 这里验证的是「删除真的生效」，而不只是「DELETE 请求发得出去」——
/// 状态码断言已经由 `response_status.rs` 覆盖，本用例补的是行为结果。
#[tokio::test]
async fn deleted_file_is_no_longer_reachable() {
    let server = local_http::start_server().await;
    let probes = Arc::new(AtomicUsize::new(0));

    Mock::given(method("GET"))
        .and(path("/dav/doomed.txt"))
        .respond_with(ThenAlways {
            calls: Arc::clone(&probes),
            first: 200,
            rest: 404,
        })
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/dav/doomed.txt"))
        .and(header("depth", "0"))
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

    let before = auth
        .get()
        .relative_url("doomed.txt".to_owned())
        .send()
        .await
        .expect("删除前的 GET 应发送成功");
    assert_eq!(before.status(), 200, "删除前资源应存在");

    let removed = auth
        .delete()
        .target_path("doomed.txt")
        .expect("合法相对路径应被接受")
        .depth(DeleteDepth::Zero)
        .send()
        .await
        .expect("DELETE 应发送成功");
    assert_eq!(removed.status(), 204);

    let after = auth
        .get()
        .relative_url("doomed.txt".to_owned())
        .send()
        .await
        .expect("删除后的 GET 应发送成功");
    assert_eq!(after.status(), 404, "删除后资源应查不到");
    assert_eq!(
        probes.load(Ordering::SeqCst),
        2,
        "删除前后各探测一次，共两次"
    );
}

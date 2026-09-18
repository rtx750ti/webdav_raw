//! 删除文件夹的边界：空文件夹与非空文件夹。
//!
//! 这是 DELETE 领域最需要钉住的边界，因为两种集合的删除在协议上走同一条路径
//! （`Depth` + 状态码），但服务端结果完全不同：
//!
//! - 空集合：合法 `Depth` 下都应删掉。
//! - 非空集合：只有递归删除才能整体删掉；用 `Depth: 0` 时服务端会拒绝。
//!
//! 删除是否真的生效，用一个带状态的 responder 验证：它模拟真实服务端——
//! 收到成功的 DELETE 后，才对同一路径返回 404。这样「删掉之后查不到」这个
//! 结论与 DELETE 的成败是因果相连的，而不是靠两份 mock 的匹配顺序碰运气。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use webdav_raw::{DeleteDepth, FindProp, WebdavAuth};
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, Request, Respond, ResponseTemplate};

use crate::common::local_http;

/// 模拟真实服务端的删除语义。
///
/// 当 `gone` 为真时对同一路径返回 404，否则返回 200。`gone` 只在收到
/// `Depth: infinity` 的 DELETE 之后才置位——这正是真实服务端的行为：
/// 没有成功删除，路径就还在。
struct ServerState {
    gone: Arc<AtomicBool>,
    hits: Arc<AtomicUsize>,
}

impl Respond for ServerState {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        self.hits.fetch_add(1, Ordering::SeqCst);

        if self.gone.load(Ordering::SeqCst) {
            ResponseTemplate::new(404)
        } else {
            ResponseTemplate::new(200).set_body_string("present")
        }
    }
}

/// 收到递归 DELETE 后把资源标记为「已删除」。
struct DeleteResponder {
    gone: Arc<AtomicBool>,
}

impl Respond for DeleteResponder {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        self.gone.store(true, Ordering::SeqCst);

        ResponseTemplate::new(204)
    }
}

/// 空文件夹：递归删除后服务端查不到它。
#[tokio::test]
async fn empty_collection_is_gone_after_recursive_delete() {
    let server = local_http::start_server().await;
    let gone = Arc::new(AtomicBool::new(false));
    let hits = Arc::new(AtomicUsize::new(0));

    Mock::given(method("GET"))
        .and(path("/dav/empty-dir/"))
        .respond_with(ServerState {
            gone: Arc::clone(&gone),
            hits: Arc::clone(&hits),
        })
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/dav/empty-dir/"))
        .and(header("depth", "infinity"))
        .respond_with(DeleteResponder {
            gone: Arc::clone(&gone),
        })
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
        .relative_path("empty-dir/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("删除前的 GET 应发送成功");
    assert_eq!(before.status(), 200, "删除前空集合应存在");

    let removed = auth
        .delete()
        .target_path("empty-dir/")
        .expect("合法相对路径应被接受")
        .depth(DeleteDepth::Infinity)
        .send()
        .await
        .expect("DELETE 应发送成功");
    assert_eq!(removed.status(), 204);

    let after = auth
        .get()
        .relative_path("empty-dir/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("删除后的 GET 应发送成功");
    assert_eq!(after.status(), 404, "删除后空集合应查不到");
    assert_eq!(hits.load(Ordering::SeqCst), 2, "删除前后各探测一次");
}

/// 非空文件夹：递归删除把集合连同成员一起删掉，成员也查不到了。
#[tokio::test]
async fn non_empty_collection_and_its_members_are_gone_after_recursive_delete() {
    let server = local_http::start_server().await;
    let collection_gone = Arc::new(AtomicBool::new(false));
    let member_gone = Arc::new(AtomicBool::new(false));
    let collection_hits = Arc::new(AtomicUsize::new(0));
    let member_hits = Arc::new(AtomicUsize::new(0));

    // 集合本身：删除前可列出，删除后查不到。
    Mock::given(method("PROPFIND"))
        .and(path("/dav/full-dir/"))
        .respond_with(
            ResponseTemplate::new(207)
                .set_body_string(collection_body())
                .insert_header("content-type", "application/xml; charset=utf-8"),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/dav/full-dir/"))
        .respond_with(ServerState {
            gone: Arc::clone(&collection_gone),
            hits: Arc::clone(&collection_hits),
        })
        .mount(&server)
        .await;

    // 集合成员：删除后随集合一起消失。
    Mock::given(method("GET"))
        .and(path("/dav/full-dir/inner.txt"))
        .respond_with(ServerState {
            gone: Arc::clone(&member_gone),
            hits: Arc::clone(&member_hits),
        })
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/dav/full-dir/"))
        .and(header("depth", "infinity"))
        .respond_with(CollectionDeleteResponder {
            collection_gone: Arc::clone(&collection_gone),
            member_gone: Arc::clone(&member_gone),
        })
        .expect(1)
        .mount(&server)
        .await;

    let auth = WebdavAuth::new(
        "alice",
        "password",
        local_http::base_url(&server, "dav").as_str(),
    )
    .expect("回环地址应创建认证对象");

    // 删除前：集合可列出（含自身与一个成员），集合与成员都查得到。
    let listing = auth
        .propfind()
        .path("full-dir/")
        .props([FindProp::Resourcetype])
        .send_and_deserialize()
        .await
        .expect("删除前的 PROPFIND 应成功");
    assert_eq!(listing.response.len(), 2, "集合应含自身与一个成员");
    assert!(
        listing
            .response
            .iter()
            .any(|item| item.href.contains("inner.txt")),
        "非空集合的列表里应能看到成员"
    );
    assert_eq!(
        auth.get()
            .relative_path("full-dir/")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("删除前的集合 GET 应发送成功")
            .status(),
        200,
        "删除前集合应存在"
    );
    assert_eq!(
        auth.get()
            .relative_path("full-dir/inner.txt")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("删除前的成员 GET 应发送成功")
            .status(),
        200,
        "删除前成员应存在"
    );

    // 递归删除。
    let removed = auth
        .delete()
        .target_path("full-dir/")
        .expect("合法相对路径应被接受")
        .depth(DeleteDepth::Infinity)
        .send()
        .await
        .expect("DELETE 应发送成功");
    assert_eq!(removed.status(), 204);

    // 删除后：集合与成员都查不到。
    assert_eq!(
        auth.get()
            .relative_path("full-dir/")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("删除后的集合 GET 应发送成功")
            .status(),
        404,
        "删除后集合应查不到"
    );
    assert_eq!(
        auth.get()
            .relative_path("full-dir/inner.txt")
            .expect("合法相对路径应被接受")
            .send()
            .await
            .expect("删除后的成员 GET 应发送成功")
            .status(),
        404,
        "删除后成员也应查不到"
    );

    assert_eq!(collection_hits.load(Ordering::SeqCst), 2);
    assert_eq!(member_hits.load(Ordering::SeqCst), 2);
}

/// 递归删除集合时，服务端把集合与其成员一并移除。
struct CollectionDeleteResponder {
    collection_gone: Arc<AtomicBool>,
    member_gone: Arc<AtomicBool>,
}

impl Respond for CollectionDeleteResponder {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        self.collection_gone.store(true, Ordering::SeqCst);
        self.member_gone.store(true, Ordering::SeqCst);

        ResponseTemplate::new(204)
    }
}

/// 非空文件夹用 `Depth: 0` 时服务端拒绝；本库不替服务端判断，原样透传。
#[tokio::test]
async fn non_empty_collection_with_zero_depth_returns_server_verdict() {
    let server = local_http::start_server().await;
    Mock::given(method("DELETE"))
        .and(path("/dav/full-dir/"))
        .and(header("depth", "0"))
        .respond_with(ResponseTemplate::new(400))
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
        .target_path("full-dir/")
        .expect("合法相对路径应被接受")
        .depth(DeleteDepth::Zero)
        .send()
        .await
        .expect("DELETE 应发送成功");

    assert_eq!(
        response.status(),
        400,
        "服务端的拒绝原样返回，库不改成自己的错误"
    );
}

/// 删除请求带空请求体。
#[tokio::test]
async fn delete_request_has_empty_body() {
    let server = local_http::start_server().await;
    Mock::given(method("DELETE"))
        .and(path("/dav/bodyless.txt"))
        .and(body_string(""))
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
        .delete()
        .target_path("bodyless.txt")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("DELETE 应发送成功");

    assert_eq!(response.status(), 204);
}

/// 207 部分失败原样透传，调用方自己决定怎么处理。
#[tokio::test]
async fn partial_failure_status_is_passed_through() {
    let server = local_http::start_server().await;
    Mock::given(method("DELETE"))
        .and(path("/dav/locked-dir/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(partial_failure_body()))
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
        .target_path("locked-dir/")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("DELETE 应发送成功");

    assert_eq!(response.status(), 207);
    let body = response.text().await.expect("响应体应可读");
    assert!(
        body.contains("423 Locked"),
        "207 响应体应原样保留，实际: {body}"
    );
}

/// 集合自身的 PROPFIND 响应固件：集合 + 一个成员文件。
fn collection_body() -> &'static str {
    r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/dav/full-dir/</D:href>
    <D:propstat>
      <D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/dav/full-dir/inner.txt</D:href>
    <D:propstat>
      <D:prop><D:resourcetype/></D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#
}

/// 207 部分失败的响应固件。
fn partial_failure_body() -> &'static str {
    r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/dav/locked-dir/</D:href>
    <D:propstat>
      <D:status>HTTP/1.1 423 Locked</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#
}

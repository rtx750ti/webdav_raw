//! 按段下载的完整调用链：经本地 MockServer 发出真实 HTTP 请求。
//!
//! 本文件固定「调用方设了 `Range` 之后，线上实际发出什么、拿回什么」：请求头的取值
//! 是否精确、206 与 200 两种答复是否都原样透传、未设区间时是否不发该头。
//!
//! 库不维护分片状态，因此这里也按「一段一次请求」的方式验证，不做多段编排。

use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

use webdav_raw::WebdavAuth;

use crate::common::local_http;

/// 分片用例请求的目标路径。
const TARGET: &str = "/dav/big.bin";

/// 启动服务器并创建使用认证根地址的对象。
async fn auth_for(server: &wiremock::MockServer) -> WebdavAuth {
    let base_url = local_http::base_url(server, "dav");

    WebdavAuth::new("alice", "password", base_url.as_str()).expect("回环地址应创建认证对象")
}

/// `range` 之后线上确实发出精确的 `Range` 头，并把 206 答复原样交回。
#[tokio::test]
async fn range_request_sends_exact_header_and_returns_206_untouched() {
    let server = local_http::start_server().await;
    Mock::given(method("GET"))
        .and(path(TARGET))
        .and(header("range", "bytes=0-1023"))
        .respond_with(
            ResponseTemplate::new(206)
                .insert_header("content-range", "bytes 0-1023/8637440")
                .insert_header("content-length", "4")
                .set_body_string("PArt"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let auth = auth_for(&server).await;

    let response = auth
        .get()
        .relative_path("big.bin")
        .expect("合法相对路径应被接受")
        .range(0, 1023)
        .expect("合法区间应被接受")
        .send()
        .await
        .expect("按段请求应发送成功");

    assert_eq!(response.status().as_u16(), 206);
    assert_eq!(
        response
            .headers()
            .get("content-range")
            .expect("206 应带 Content-Range")
            .to_str()
            .expect("应为文本"),
        "bytes 0-1023/8637440"
    );
    assert_eq!(
        response.text().await.expect("响应体应可读取"),
        "PArt"
    );
}

/// 服务端忽略 `Range` 回 200 时，整份内容照样原样交回，库不报错也不改写。
#[tokio::test]
async fn ignored_range_returns_whole_file_untouched() {
    let server = local_http::start_server().await;
    Mock::given(method("GET"))
        .and(path(TARGET))
        .and(header("range", "bytes=0-1023"))
        .respond_with(ResponseTemplate::new(200).set_body_string("whole-file-content"))
        .expect(1)
        .mount(&server)
        .await;
    let auth = auth_for(&server).await;

    let response = auth
        .get()
        .relative_path("big.bin")
        .expect("合法相对路径应被接受")
        .range(0, 1023)
        .expect("合法区间应被接受")
        .send()
        .await
        .expect("按段请求应发送成功");

    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers().get("content-range").is_none());
    assert_eq!(
        response.text().await.expect("响应体应可读取"),
        "whole-file-content"
    );
}

/// 多段各自发一次请求，每段的 `Range` 头与答复都对应各自的区间。
#[tokio::test]
async fn sequential_segments_each_carry_their_own_range() {
    let server = local_http::start_server().await;
    for (range, body) in [
        ("bytes=0-3", "aaaa"),
        ("bytes=4-7", "bbbb"),
        ("bytes=8-11", "cccc"),
    ] {
        Mock::given(method("GET"))
            .and(path(TARGET))
            .and(header("range", range))
            .respond_with(ResponseTemplate::new(206).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
    }
    let auth = auth_for(&server).await;

    for (start, end, expected) in [(0_u64, 3_u64, "aaaa"), (4, 7, "bbbb"), (8, 11, "cccc")] {
        let response = auth
            .get()
            .relative_path("big.bin")
            .expect("合法相对路径应被接受")
            .range(start, end)
            .expect("合法区间应被接受")
            .send()
            .await
            .expect("按段请求应发送成功");

        assert_eq!(response.status().as_u16(), 206);
        assert_eq!(
            response.text().await.expect("响应体应可读取"),
            expected,
            "区间 {start}-{end} 应取回对应内容"
        );
    }
}

/// `Range` 与认证头共存：默认 Authorization 不因附加头而丢失。
#[tokio::test]
async fn range_coexists_with_authorization_header() {
    let server = local_http::start_server().await;
    Mock::given(method("GET"))
        .and(path(TARGET))
        .and(header("range", "bytes=0-1023"))
        .and(header("authorization", "Basic YWxpY2U6cGFzc3dvcmQ="))
        .respond_with(ResponseTemplate::new(206).set_body_string("auth-ok"))
        .expect(1)
        .mount(&server)
        .await;
    let auth = auth_for(&server).await;

    let response = auth
        .get()
        .relative_path("big.bin")
        .expect("合法相对路径应被接受")
        .range(0, 1023)
        .expect("合法区间应被接受")
        .send()
        .await
        .expect("按段请求应发送成功");

    assert_eq!(response.status().as_u16(), 206);
}

/// 未调用 `range` 时不发 `Range` 头；服务端据此回整份。
#[tokio::test]
async fn absent_range_omits_header_on_the_wire() {
    let server = local_http::start_server().await;
    Mock::given(method("GET"))
        .and(path(TARGET))
        .respond_with(ResponseTemplate::new(200).set_body_string("whole"))
        .expect(1)
        .mount(&server)
        .await;
    let auth = auth_for(&server).await;

    let response = auth
        .get()
        .relative_path("big.bin")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("请求应发送成功");

    assert_eq!(response.status().as_u16(), 200);
    let requests = server
        .received_requests()
        .await
        .expect("MockServer 应记录请求");
    assert!(
        requests[0].headers.get("range").is_none(),
        "未设区间时不应发出 Range 头"
    );
}

/// 单字节区间在线上就是 `bytes=7-7`。
#[tokio::test]
async fn single_byte_range_reaches_the_wire() {
    let server = local_http::start_server().await;
    Mock::given(method("GET"))
        .and(path(TARGET))
        .and(header("range", "bytes=7-7"))
        .respond_with(ResponseTemplate::new(206).set_body_string("x"))
        .expect(1)
        .mount(&server)
        .await;
    let auth = auth_for(&server).await;

    let response = auth
        .get()
        .relative_path("big.bin")
        .expect("合法相对路径应被接受")
        .range(7, 7)
        .expect("合法区间应被接受")
        .send()
        .await
        .expect("按段请求应发送成功");

    assert_eq!(response.status().as_u16(), 206);
    assert_eq!(response.text().await.expect("响应体应可读取"), "x");
}

/// `header` 显式覆盖 `range` 后，线上发出的是覆盖后的取值。
#[tokio::test]
async fn explicit_header_override_reaches_the_wire() {
    let server = local_http::start_server().await;
    Mock::given(method("GET"))
        .and(path(TARGET))
        .and(header("range", "bytes=100-199"))
        .respond_with(ResponseTemplate::new(206).set_body_string("override"))
        .expect(1)
        .mount(&server)
        .await;
    let auth = auth_for(&server).await;

    let response = auth
        .get()
        .relative_path("big.bin")
        .expect("合法相对路径应被接受")
        .range(0, 1023)
        .expect("合法区间应被接受")
        .header("range", "bytes=100-199")
        .expect("合法头应被接受")
        .send()
        .await
        .expect("按段请求应发送成功");

    assert_eq!(response.text().await.expect("响应体应可读取"), "override");
}

/// 服务端对区间不满意时回 416，该状态原样交回，库不解释也不重试。
#[tokio::test]
async fn unsatisfiable_range_status_is_returned_raw() {
    let server = local_http::start_server().await;
    Mock::given(method("GET"))
        .and(path(TARGET))
        .and(header("range", "bytes=99999999-100000000"))
        .respond_with(
            ResponseTemplate::new(416)
                .insert_header("content-range", "bytes */8637440")
                .set_body_string("range not satisfiable"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let auth = auth_for(&server).await;

    let response = auth
        .get()
        .relative_path("big.bin")
        .expect("合法相对路径应被接受")
        .range(99_999_999, 100_000_000)
        .expect("合法区间应被接受")
        .send()
        .await
        .expect("请求应发送成功");

    assert_eq!(
        response.status().as_u16(),
        416,
        "服务端的区间错误状态应原样交回"
    );
    assert_eq!(
        response
            .headers()
            .get("content-range")
            .expect("416 应带 Content-Range")
            .to_str()
            .expect("应为文本"),
        "bytes */8637440"
    );
}

/// 一次 Builder 的区间设置不会渗到下一次请求，同一路径可连续取不同段。
#[tokio::test]
async fn range_does_not_leak_between_requests() {
    let server = local_http::start_server().await;
    Mock::given(method("GET"))
        .and(path(TARGET))
        .and(header("range", "bytes=0-3"))
        .respond_with(ResponseTemplate::new(206).set_body_string("first"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(TARGET))
        .and(header("range", "bytes=4-7"))
        .respond_with(ResponseTemplate::new(206).set_body_string("second"))
        .expect(1)
        .mount(&server)
        .await;
    let auth = auth_for(&server).await;

    let first = auth
        .get()
        .relative_path("big.bin")
        .expect("合法相对路径应被接受")
        .range(0, 3)
        .expect("合法区间应被接受")
        .send()
        .await
        .expect("首次请求应发送成功");
    let second = auth
        .get()
        .relative_path("big.bin")
        .expect("合法相对路径应被接受")
        .range(4, 7)
        .expect("合法区间应被接受")
        .send()
        .await
        .expect("二次请求应发送成功");

    assert_eq!(first.text().await.expect("应可读取"), "first");
    assert_eq!(second.text().await.expect("应可读取"), "second");
}

/// 区间错误在发出请求之前就被拒，服务端不应收到任何请求。
#[tokio::test]
async fn invalid_range_never_reaches_the_wire() {
    let server = local_http::start_server().await;
    Mock::given(method("GET"))
        .and(path(TARGET))
        .respond_with(ResponseTemplate::new(206))
        .expect(0)
        .mount(&server)
        .await;
    let auth = auth_for(&server).await;

    let result = auth
        .get()
        .relative_path("big.bin")
        .expect("合法相对路径应被接受")
        .range(10, 9);

    assert!(result.is_err(), "起点大于终点应返回错误");
    assert!(
        server
            .received_requests()
            .await
            .expect("MockServer 应可查询")
            .is_empty(),
        "区间校验失败时不应发出请求"
    );
}

/// 按段取到的失败状态（403/404）原样交回，库不做判断。
#[tokio::test]
async fn error_status_for_segment_is_returned_raw() {
    let server = local_http::start_server().await;
    for status in [403, 404, 416] {
        Mock::given(method("GET"))
            .and(path(format!("/dav/status-{status}.bin")))
            .and(header("range", "bytes=0-1023"))
            .respond_with(ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;
    }
    let auth = auth_for(&server).await;

    for status in [403, 404, 416] {
        let response = auth
            .get()
            .relative_path(&format!("status-{status}.bin"))
            .expect("合法相对路径应被接受")
            .range(0, 1023)
            .expect("合法区间应被接受")
            .send()
            .await
            .expect("错误状态应仍返回响应");

        assert_eq!(response.status().as_u16(), status);
    }
}

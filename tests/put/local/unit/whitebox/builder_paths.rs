//! Builder 的路径、请求头与默认值分支。

use webdav_core::{PutBuilder, PutError};
use webdav_core::{Client, HeaderMap, HeaderValue, Request, Url};

use crate::support::fixtures::{bytes_body, bytes_body_with_content_type};

/// 构造指向固定根地址的 Builder。
fn builder() -> PutBuilder {
    PutBuilder::new(
        Client::new(),
        Url::parse("https://example.com/dav/").expect("基准地址必须合法"),
    )
}

/// 取请求头文本，便于断言。
fn header_text<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
}

#[tokio::test]
async fn build_sets_put_method_path_and_body() {
    let request = builder()
        .relative_path("hello.txt")
        .expect("相对路径必须有效")
        .body(bytes_body(b"hello".to_vec()))
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(request.method(), reqwest::Method::PUT);
    assert_eq!(request.url().as_str(), "https://example.com/dav/hello.txt");
    assert_eq!(header_text(&request, "content-length"), Some("5"));
    assert_eq!(request.body().unwrap().as_bytes().unwrap(), b"hello");
}

#[tokio::test]
async fn empty_relative_path_keeps_root_url() {
    let request = builder()
        .relative_path("")
        .expect("空相对路径应被接受")
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/dav/");
}

#[tokio::test]
async fn relative_path_encodes_space_and_unicode() {
    let request = builder()
        .relative_path("目录/测试 文件.txt")
        .expect("合法相对路径应被接受")
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(
        request.url().as_str(),
        "https://example.com/dav/%E7%9B%AE%E5%BD%95/%E6%B5%8B%E8%AF%95%20%E6%96%87%E4%BB%B6.txt"
    );
}

#[tokio::test]
async fn absolute_path_replaces_auth_root() {
    let request = builder()
        .absolute_path("https://upload.example.com/file.bin")
        .expect("合法绝对地址应被接受")
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(
        request.url().as_str(),
        "https://upload.example.com/file.bin"
    );
}

#[tokio::test]
async fn invalid_relative_path_returns_url_error() {
    let result = builder().relative_path("http://");

    assert!(matches!(result, Err(PutError::Url(_))));
}

#[tokio::test]
async fn invalid_absolute_path_returns_url_error() {
    let result = builder().absolute_path("not a url");

    assert!(matches!(result, Err(PutError::Url(_))));
}

#[tokio::test]
async fn invalid_header_name_returns_header_name_error() {
    let result = builder().header("bad header", "value");

    assert!(matches!(result, Err(PutError::HeaderName(_))));
}

#[tokio::test]
async fn invalid_header_value_returns_header_value_error() {
    let result = builder().header("x-test", "bad\nvalue");

    assert!(matches!(result, Err(PutError::HeaderValue(_))));
}

#[tokio::test]
async fn custom_header_is_written_to_request() {
    let request = builder()
        .header("x-custom", "present")
        .expect("合法请求头应被接受")
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(header_text(&request, "x-custom"), Some("present"));
}

#[tokio::test]
async fn headers_batch_setter_is_applied() {
    let mut headers = HeaderMap::new();
    headers.insert("x-batch-one", HeaderValue::from_static("1"));
    headers.insert("x-batch-two", HeaderValue::from_static("2"));

    let request = builder()
        .headers(headers)
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(header_text(&request, "x-batch-one"), Some("1"));
    assert_eq!(header_text(&request, "x-batch-two"), Some("2"));
}

/// 没有数据源时补零长度与默认内容类型。
#[tokio::test]
async fn default_headers_are_applied_without_body() {
    let request = builder().build().await.expect("请求应构建成功");

    assert_eq!(header_text(&request, "content-length"), Some("0"));
    assert_eq!(
        header_text(&request, "content-type"),
        Some("application/octet-stream")
    );
}

/// 调用方设置的内容类型优先，其余自定义头一并保留。
#[tokio::test]
async fn caller_content_type_is_preserved() {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", HeaderValue::from_static("text/custom"));
    headers.insert("x-keep", HeaderValue::from_static("yes"));

    let request = builder()
        .headers(headers)
        .body(bytes_body_with_content_type(vec![1], "text/plain"))
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(header_text(&request, "content-type"), Some("text/custom"));
    assert_eq!(header_text(&request, "x-keep"), Some("yes"));
    assert_eq!(header_text(&request, "content-length"), Some("1"));
}

/// 调用方设置的 `Content-Length` 被数据源长度覆盖。
#[tokio::test]
async fn caller_content_length_is_overridden_by_data_length() {
    let request = builder()
        .header("content-length", "999")
        .expect("合法请求头名应被接受")
        .body(bytes_body(vec![1, 2]))
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(
        header_text(&request, "content-length"),
        Some("2"),
        "长度只有一个可信来源，不接受调用方覆盖"
    );
}

/// 内存源里有内容类型时写入请求头。
#[tokio::test]
async fn memory_source_content_type_reaches_header() {
    let request = builder()
        .body(bytes_body_with_content_type(vec![1], "application/zip"))
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(header_text(&request, "content-type"), Some("application/zip"));
}

#[tokio::test]
async fn send_returns_request_error_when_server_is_unreachable() {
    let result = PutBuilder::new(Client::new(), Url::parse("http://127.0.0.1:1/").unwrap())
        .send()
        .await;

    assert!(matches!(result, Err(PutError::Request(_))));
}

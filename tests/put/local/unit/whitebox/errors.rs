//! PUT 领域错误类型的渲染分支。
//!
//! `thiserror` 派生的 `Display::fmt` 是独立函数：只有真正格式化过错误对象才会
//! 执行。只用 `matches!` 判断变体的用例不会执行它，因此这一族函数需要本文件补齐。

use webdav_core::Client;
use webdav_core::Url;
use webdav_core::put::builder::{PutBuilder, PutError};
use webdav_core::put::put_body::u8_bytes_chunk::{U8BytesChunk, U8BytesChunkError};
use webdav_core::put::put_body::u8_bytes_data::U8BytesData;

/// 构造基准 Builder，供路径与请求头错误复用。
fn builder() -> PutBuilder {
    PutBuilder::new(
        Client::new(),
        Url::parse("https://example.com/dav/").expect("基准地址必须合法"),
    )
}

#[test]
fn put_error_renders_url_message() {
    let error = builder()
        .relative_path("http://")
        .err()
        .expect("非法相对路径应返回错误");

    assert!(matches!(error, PutError::Url(_)));
    assert!(error.to_string().starts_with("URL 格式错误"));
}

#[test]
fn put_error_renders_header_name_message() {
    let error = builder()
        .header("bad header", "value")
        .err()
        .expect("非法请求头名称应返回错误");

    assert!(matches!(error, PutError::HeaderName(_)));
    assert!(error.to_string().starts_with("请求头名称格式错误"));
}

#[test]
fn put_error_renders_header_value_message() {
    let error = builder()
        .header("x-test", "bad\nvalue")
        .err()
        .expect("非法请求头取值应返回错误");

    assert!(matches!(error, PutError::HeaderValue(_)));
    assert!(error.to_string().starts_with("请求头值格式错误"));
}

/// `PutError::FileMetadata` 渲染文件长度读取失败的来源信息。
#[test]
fn put_error_renders_file_metadata_message() {
    let error = PutError::from(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "missing.bin",
    ));

    assert!(matches!(error, PutError::FileMetadata(_)));
    assert!(error.to_string().starts_with("读取文件长度失败"));
    assert!(error.to_string().contains("missing.bin"));
}

#[tokio::test]
async fn put_error_renders_request_message() {
    let error = PutBuilder::new(Client::new(), Url::parse("http://127.0.0.1:1/").unwrap())
        .send()
        .await
        .err()
        .expect("不可达地址应返回请求错误");

    assert!(matches!(error, PutError::Request(_)));
    assert!(error.to_string().starts_with("构建 HTTP 请求失败"));
}

#[test]
fn put_error_exposes_source_chain() {
    use std::error::Error;

    let url_error = builder()
        .absolute_path("not a url")
        .err()
        .expect("非法绝对地址应返回错误");
    assert!(url_error.source().is_some());

    let io_error = PutError::from(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "missing.bin",
    ));
    assert!(io_error.source().is_some());
}

#[test]
fn chunk_error_renders_every_variant() {
    let cases: [(U8BytesChunkError, &str); 5] = [
        (U8BytesChunkError::IncompleteRange, "分片起止范围必须同时提供"),
        (U8BytesChunkError::StartAfterEnd, "分片起点不能大于终点"),
        (U8BytesChunkError::RangeOverflow, "分片范围长度溢出"),
        (U8BytesChunkError::EndExceedsTotal, "分片终点超出总长度"),
        (
            U8BytesChunkError::LengthMismatch {
                range_length: 1,
                data_length: 2,
            },
            "分片范围长度为 1，实际数据长度为 2",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
}

/// 每个分片错误变体都能由公开 API 真实触发。
#[test]
fn chunk_errors_are_reachable_through_public_api() {
    let cases: [U8BytesChunkError; 5] = [
        U8BytesChunk::new(
            U8BytesData::new(vec![1], None).unwrap(),
            Some(0),
            None,
            None,
            None,
        )
        .unwrap_err(),
        U8BytesChunk::new(
            U8BytesData::new(vec![1, 2], None).unwrap(),
            Some(3),
            Some(2),
            None,
            None,
        )
        .unwrap_err(),
        U8BytesChunk::new(
            U8BytesData::new(vec![1], None).unwrap(),
            Some(0),
            Some(u64::MAX),
            None,
            None,
        )
        .unwrap_err(),
        U8BytesChunk::new(
            U8BytesData::new(vec![1], None).unwrap(),
            Some(0),
            Some(0),
            Some(0),
            None,
        )
        .unwrap_err(),
        U8BytesChunk::new(
            U8BytesData::new(vec![1, 2], None).unwrap(),
            Some(0),
            Some(0),
            None,
            None,
        )
        .unwrap_err(),
    ];

    for error in cases {
        assert!(!error.to_string().is_empty(), "{error:?} 必须能渲染");
    }
}

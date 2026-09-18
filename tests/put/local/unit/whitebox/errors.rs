//! PUT 领域错误类型的渲染分支。
//!
//! `thiserror` 派生的 `Display::fmt` 是独立函数：只有真正格式化过错误对象才会
//! 执行。只用 `matches!` 判断变体的用例不会执行它，因此这一族函数需要本文件补齐。

use webdav_raw::Client;
use webdav_raw::Url;
use webdav_raw::{PutBuilder, PutError};
use webdav_raw::{U8BytesChunk, U8BytesChunkError};
use webdav_raw::{U8BytesData, U8BytesDataError, U8BytesError, U8Metadata};

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
        .expect_err("非法相对路径应返回错误");

    assert!(matches!(error, PutError::Url(_)));
    assert!(error.to_string().starts_with("URL 格式错误"));
}

#[test]
fn put_error_renders_header_name_message() {
    let error = builder()
        .header("bad header", "value")
        .expect_err("非法请求头名称应返回错误");

    assert!(matches!(error, PutError::HeaderName(_)));
    assert!(error.to_string().starts_with("请求头名称格式错误"));
}

#[test]
fn put_error_renders_header_value_message() {
    let error = builder()
        .header("x-test", "bad\nvalue")
        .expect_err("非法请求头取值应返回错误");

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
        .expect_err("不可达地址应返回请求错误");

    assert!(matches!(error, PutError::Request(_)));
    assert!(error.to_string().starts_with("构建 HTTP 请求失败"));
}

#[test]
fn put_error_exposes_source_chain() {
    use std::error::Error;

    let url_error = builder()
        .absolute_path("not a url")
        .expect_err("非法绝对地址应返回错误");
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
        (
            U8BytesChunkError::IncompleteRange,
            "分片起止范围必须同时提供",
        ),
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

/// 内存二进制元数据的两个错误变体都能由公开 API 触发并渲染。
#[test]
fn u8_bytes_error_renders_every_variant() {
    let empty_name = U8Metadata::from_name("   ".to_owned()).expect_err("纯空白文件名必须被拒绝");
    assert_eq!(empty_name, U8BytesError::EmptyName);
    assert_eq!(empty_name.to_string(), "文件名不能为空");

    let mut metadata = U8Metadata::from_name("a.txt".to_owned()).expect("推断必须成功");
    metadata.content_type = "no-slash".to_owned();
    let invalid = metadata.validate().expect_err("非法内容类型必须被校验发现");

    assert_eq!(
        invalid,
        U8BytesError::InvalidContentType("no-slash".to_owned())
    );
    assert_eq!(invalid.to_string(), "Content-Type 格式错误: no-slash");
}

/// 长度溢出只在 `usize` 比 `u64` 宽的平台上可能出现，因此这里只验证变体
/// 本身可构造、可渲染，不伪造一个在该平台上不存在的输入。
#[test]
fn u8_bytes_data_error_renders_length_overflow() {
    use std::error::Error;

    let error = U8BytesDataError::LengthOverflow;

    assert_eq!(error.to_string(), "二进制数据长度超出 u64 范围");
    assert!(error.source().is_none());
}

//! PUT 数据来源到请求语义的转换。
//!
//! 覆盖"来源到请求体字节、元数据到请求头"的一致性，以及三种来源经 Builder 后的
//! 语义是否一致。

use webdav_core::PutBuilder;
use webdav_core::U8Bytes;
use webdav_core::U8BytesChunk;
use webdav_core::U8BytesData;
use webdav_core::{PutBody, FileHandle};
use webdav_core::{Client, Request, Url};

use crate::support::fixtures::{
    bytes_body, bytes_body_with_content_type, metadata, remove_temp_file, write_temp_file,
};

/// 构造统一的基准 Builder。
fn builder() -> PutBuilder {
    PutBuilder::new(
        Client::new(),
        Url::parse("https://example.com/dav/").expect("基准地址必须合法"),
    )
}

/// 读取请求的 `Content-Length` 文本。
fn content_length(request: &Request) -> String {
    request
        .headers()
        .get("content-length")
        .expect("请求必须带 Content-Length")
        .to_str()
        .expect("Content-Length 必须是可见文本")
        .to_owned()
}

/// 内存源的字节与长度原样进入请求。
#[tokio::test]
async fn bytes_source_converts_to_request_body_with_matching_length() {
    let request = builder()
        .body(bytes_body(vec![1, 2, 3]))
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(content_length(&request), "3");
    assert_eq!(request.body().unwrap().as_bytes().unwrap(), &[1, 2, 3]);
}

/// 元数据的内容类型映射为请求的 `Content-Type`。
#[tokio::test]
async fn metadata_content_type_maps_to_request_header() {
    let request = builder()
        .body(bytes_body_with_content_type(vec![7], "text/custom"))
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(
        request.headers().get("content-type").unwrap(),
        "text/custom"
    );
    assert_eq!(content_length(&request), "1");
}

/// 元数据里除 `content_type` 之外的字段不会自动变成请求头。
#[tokio::test]
async fn other_metadata_fields_never_become_request_headers() {
    let mut metadata = metadata("application/zip");
    metadata.etag = Some("\"abc\"".to_owned());
    metadata.last_modified = Some("Wed, 21 Oct 2015 07:28:00 GMT".to_owned());

    let data = U8BytesData::new(vec![1], None).expect("应构造成功");
    let request = builder()
        .body(PutBody::from_bytes(U8Bytes::new(data, metadata)))
        .build()
        .await
        .expect("请求应构建成功");

    for header in ["if-match", "if-unmodified-since", "x-id", "x-name"] {
        assert!(
            request.headers().get(header).is_none(),
            "{header} 不应由本库自动写入"
        );
    }
    assert_eq!(
        request.headers().get("content-type").unwrap(),
        "application/zip"
    );
}

/// 分片源默认**不**把范围写成 `Content-Range`。
#[tokio::test]
async fn chunk_source_does_not_write_content_range_by_default() {
    let data = U8BytesData::new(vec![9, 8], None).expect("应构造成功");
    let chunk = U8BytesChunk::new(data, Some(0), Some(1), Some(2), None).expect("应构造成功");

    let request = builder()
        .body(PutBody::from_chunk(chunk))
        .build()
        .await
        .expect("请求应构建成功");

    assert!(
        request.headers().get("content-range").is_none(),
        "本库不默认宣称服务端支持部分上传"
    );
    assert_eq!(content_length(&request), "2");
    assert_eq!(request.body().unwrap().as_bytes().unwrap(), &[9, 8]);
}

/// 分片范围可以由调用方显式写成请求头。
#[tokio::test]
async fn chunk_range_can_be_sent_when_caller_opts_in() {
    let data = U8BytesData::new(vec![9, 8], None).expect("应构造成功");
    let chunk = U8BytesChunk::new(data, Some(0), Some(1), Some(2), None).expect("应构造成功");
    let range = chunk.to_content_range().expect("分片带范围");

    let request = builder()
        .body(PutBody::from_chunk(chunk))
        .header("content-range", &range)
        .expect("合法请求头应被接受")
        .build()
        .await
        .expect("请求应构建成功");

    assert_eq!(
        request.headers().get("content-range").unwrap(),
        "bytes 0-1/2"
    );
}

/// 内存源与文件源经 Builder 后长度一致；分片源的字节与长度也一致。
#[tokio::test]
async fn all_three_sources_produce_equivalent_request_semantics() {
    let payload = b"semantic".to_vec();
    let expected_length = payload.len().to_string();
    let total = payload.len() as u64;

    let bytes_request = builder()
        .body(bytes_body(payload.clone()))
        .build()
        .await
        .expect("内存源请求应构建成功");

    let chunk = U8BytesChunk::new(
        U8BytesData::new(payload.clone(), None).expect("应构造成功"),
        Some(0),
        Some(total - 1),
        Some(total),
        None,
    )
    .expect("分片应构造成功");
    let chunk_request = builder()
        .body(PutBody::from_chunk(chunk))
        .build()
        .await
        .expect("分片源请求应构建成功");

    let path = write_temp_file("equivalent", &payload).await;
    let file = tokio::fs::File::open(&path)
        .await
        .expect("临时文件必须可打开");
    let file_request = builder()
        .body(PutBody::from_file(FileHandle::new(file, None)))
        .build()
        .await
        .expect("文件源请求应构建成功");
    remove_temp_file(&path).await;

    assert_eq!(content_length(&bytes_request), expected_length);
    assert_eq!(content_length(&chunk_request), expected_length);
    assert_eq!(content_length(&file_request), expected_length);

    assert_eq!(
        bytes_request.body().unwrap().as_bytes().unwrap(),
        payload.as_slice()
    );
    assert_eq!(
        chunk_request.body().unwrap().as_bytes().unwrap(),
        payload.as_slice()
    );
    assert!(
        file_request.body().unwrap().as_bytes().is_none(),
        "文件源必须是流式 body，不能整体读入内存"
    );
}

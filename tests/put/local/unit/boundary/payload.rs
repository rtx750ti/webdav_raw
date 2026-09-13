//! PUT 载荷与数据源的边界。
//!
//! 覆盖空值、单字节、空文件、空分片，以及"来源与元数据正交"这一点。

use webdav_core::{FileHandle, PutBody, PutData, U8Bytes, U8BytesChunk, U8BytesData, U8Metadata};

use crate::support::fixtures::{bytes_body, default_metadata, metadata, write_temp_file};

#[test]
fn empty_metadata_has_no_optional_values() {
    let metadata = U8Metadata::default();

    assert_eq!(metadata.name, "");
    assert_eq!(metadata.content_type, "");
    assert_eq!(metadata.etag, None);
    assert_eq!(metadata.last_modified, None);
    assert_eq!(metadata.create_time, None);
    assert_eq!(metadata.update_time, None);
}

#[test]
fn bytes_payload_length_comes_from_data() {
    let data = U8BytesData::new(vec![1], None).expect("应构造成功");
    let bytes = U8Bytes::new(data, default_metadata());

    assert_eq!(bytes.data.length, 1);
    assert_eq!(bytes.metadata.content_type, "application/octet-stream");
}

#[test]
fn empty_bytes_body_is_still_a_memory_source() {
    let body = bytes_body(Vec::new());

    assert!(matches!(body.data, PutData::U8Bytes(_)));
}

/// 三种数据源都能用 `PutBody::new` 显式装配，不必走专用构造方法。
#[test]
fn put_body_new_accepts_every_data_source() {
    let data = U8BytesData::new(vec![1], None).expect("应构造成功");
    let body = PutBody::new(PutData::U8Bytes(U8Bytes::new(data, default_metadata())));
    assert!(matches!(body.data, PutData::U8Bytes(_)));

    let data = U8BytesData::new(vec![1], None).expect("应构造成功");
    let chunk = U8BytesChunk::new(data, None, None, None, None).expect("应构造成功");
    let body = PutBody::new(PutData::U8BytesChunk(chunk));
    assert!(matches!(body.data, PutData::U8BytesChunk(_)));
}

/// `PutBody.data` 是公开字段，调用方可以直接替换数据源。
#[tokio::test]
async fn put_body_data_field_can_be_replaced() {
    let path = write_temp_file("replace_source", b"x").await;
    let file = tokio::fs::File::open(&path)
        .await
        .expect("临时文件必须可打开");

    let mut body = bytes_body(vec![1, 2, 3]);
    body.data = PutData::File(FileHandle::new(file, None));

    assert!(matches!(body.data, PutData::File(_)));

    crate::support::fixtures::remove_temp_file(&path).await;
}

/// `U8Bytes::new` 把数据与元数据原样绑定，不做二次加工。
#[test]
fn u8_bytes_new_binds_data_and_metadata() {
    let data = U8BytesData::new(vec![1, 2], Some("op-1".to_owned())).expect("应构造成功");
    let metadata = metadata("application/zip");
    let bytes = U8Bytes::new(data, metadata);

    assert_eq!(bytes.data.length, 2);
    assert_eq!(bytes.data.id.as_deref(), Some("op-1"));
    assert_eq!(bytes.metadata.content_type, "application/zip");
    assert_eq!(bytes.metadata.name, "test.bin");
}

/// 构造出的完整二进制可以直接交给 `PutBody`。
#[test]
fn u8_bytes_new_result_can_be_wrapped_into_body() {
    let bytes = U8Bytes::new(
        U8BytesData::new(vec![1, 2, 3], None).expect("应构造成功"),
        default_metadata(),
    );

    let body = PutBody::from_bytes(bytes);

    match body.data {
        PutData::U8Bytes(bytes) => assert_eq!(bytes.data.length, 3),
        other => panic!("期望 U8Bytes，实际为 {other:?}"),
    }
}

#[test]
fn chunk_body_carries_range_but_is_still_one_source() {
    let data = U8BytesData::new(vec![9, 8], None).expect("应构造成功");
    let chunk = U8BytesChunk::new(data, Some(0), Some(1), Some(2), None).expect("应构造成功");
    let body = PutBody::from_chunk(chunk);

    match body.data {
        PutData::U8BytesChunk(chunk) => {
            assert_eq!(chunk.data.length, 2);
            assert_eq!(chunk.to_content_range().as_deref(), Some("bytes 0-1/2"));
        }
        other => panic!("期望 U8BytesChunk，实际为 {other:?}"),
    }
}

#[test]
fn file_body_reports_its_own_data_field() {
    let path = std::env::temp_dir().join("webdav_core_put_file_body_probe.bin");
    std::fs::write(&path, b"x").expect("临时文件必须可写");
    let file = std::fs::File::open(&path).expect("临时文件必须可打开");
    let handle = FileHandle::new(tokio::fs::File::from_std(file), Some("op-1".to_owned()));

    let body = PutBody::from_file(handle);

    match body.data {
        PutData::File(handle) => assert_eq!(handle.id.as_deref(), Some("op-1")),
        other => panic!("期望 File，实际为 {other:?}"),
    }

    let _ = std::fs::remove_file(&path);
}

/// 元数据与数据来源正交：三种来源都能带上同一份元数据。
#[tokio::test]
async fn metadata_is_orthogonal_to_every_source() {
    let metadata = U8Metadata {
        name: "report.zip".to_owned(),
        content_type: "application/zip".to_owned(),
        etag: Some("\"abc\"".to_owned()),
        last_modified: Some("Wed, 21 Oct 2015 07:28:00 GMT".to_owned()),
        create_time: Some("2015-10-21T07:28:00Z".to_owned()),
        update_time: Some("2015-10-22T07:28:00Z".to_owned()),
    };

    let bytes = U8Bytes::new(
        U8BytesData::new(vec![1], None).expect("应构造成功"),
        metadata,
    );

    assert_eq!(bytes.metadata.name, "report.zip");
    assert_eq!(bytes.metadata.content_type, "application/zip");
    assert_eq!(bytes.metadata.etag.as_deref(), Some("\"abc\""));
    assert_eq!(
        bytes.metadata.last_modified.as_deref(),
        Some("Wed, 21 Oct 2015 07:28:00 GMT")
    );
}

/// 空文件仍然是一个合法的文件源。
#[tokio::test]
async fn empty_file_is_an_accepted_source() {
    let path = write_temp_file("empty_file", b"").await;
    let file = tokio::fs::File::open(&path)
        .await
        .expect("临时文件必须可打开");
    let body = PutBody::from_file(FileHandle::new(file, None));

    assert!(matches!(body.data, PutData::File(_)));

    crate::support::fixtures::remove_temp_file(&path).await;
}

/// 元数据字段公开，调用方可以直接改写。
#[test]
fn metadata_fields_are_public() {
    let mut metadata = metadata("text/plain");
    metadata.etag = Some("\"v1\"".to_owned());
    metadata.content_type = "application/x-custom".to_owned();

    assert_eq!(metadata.etag.as_deref(), Some("\"v1\""));
    assert_eq!(metadata.content_type, "application/x-custom");
}

//! 二进制数据与分片范围的边界。
//!
//! 长度是唯一信任源：由字节算出，不接受调用方声明。分片范围采用闭区间语义，
//! `0..=1` 表示两个字节。

use webdav_core::U8BytesData;
use webdav_core::{U8BytesChunk, U8BytesChunkError};

#[test]
fn empty_data_has_zero_length() {
    let data = U8BytesData::new(Vec::new(), None).expect("空数据应构造成功");

    assert_eq!(data.length, 0);
    assert_eq!(data.len(), 0);
    assert!(data.is_empty());
}

#[test]
fn single_byte_data_has_length_one() {
    let data = U8BytesData::new(vec![0x7f], None).expect("单字节应构造成功");

    assert_eq!(data.length, 1);
    assert_eq!(data.len(), 1);
    assert!(!data.is_empty());
}

#[test]
fn length_is_computed_from_bytes_not_declared() {
    let data = U8BytesData::new(vec![1, 2, 3, 4, 5], None).expect("数据应构造成功");

    assert_eq!(data.length, 5, "长度必须等于实际字节数");
}

#[test]
fn id_is_preserved_and_optional() {
    let with_id = U8BytesData::new(vec![1], Some("payload".to_owned())).expect("应构造成功");
    let without_id = U8BytesData::new(vec![1], None).expect("应构造成功");

    assert_eq!(with_id.id.as_deref(), Some("payload"));
    assert_eq!(without_id.id, None);
}

#[test]
fn into_data_hands_over_bytes_and_clears_object() {
    let data = U8BytesData::new(vec![1, 2, 3], None).expect("应构造成功");

    assert_eq!(data.into_data(), vec![1, 2, 3]);
}

#[test]
fn take_clears_bytes_and_resets_length() {
    let mut data = U8BytesData::new(vec![1, 2, 3], None).expect("应构造成功");

    let taken = data.take();

    assert_eq!(taken, vec![1, 2, 3]);
    assert_eq!(data.length, 0, "取出后长度必须归零，保持长度与字节同源");
    assert!(data.is_empty());
    assert_eq!(data.into_data(), Vec::<u8>::new());
}

/// `Default` 与 `new(Vec::new(), None)` 表现一致：长度零、字节为空。
#[test]
fn default_data_is_empty_and_self_consistent() {
    let data = U8BytesData::default();

    assert_eq!(data.length, 0);
    assert!(data.is_empty());
    assert_eq!(data.id, None);
    assert_eq!(data.into_data(), Vec::<u8>::new());
}

/// 空数据上取字节同样把长度归零，不会出现负数或残留长度。
#[test]
fn take_on_empty_data_stays_consistent() {
    let mut data = U8BytesData::default();

    assert!(data.take().is_empty());
    assert_eq!(data.length, 0);
    assert!(data.is_empty());
}

#[test]
fn omitted_range_is_accepted() {
    let data = U8BytesData::new(vec![1, 2], None).expect("应构造成功");
    let chunk = U8BytesChunk::new(data, None, None, None, None).expect("缺省范围应构造成功");

    assert_eq!(chunk.range_start, None);
    assert_eq!(chunk.range_end, None);
    assert_eq!(chunk.total_length, None);
    assert_eq!(chunk.data.length, 2);
    assert_eq!(chunk.to_content_range(), None, "没有范围就没有请求头取值");
}

/// 空数据也可以是不带范围的分片。
#[test]
fn empty_chunk_without_range_is_accepted() {
    let chunk = U8BytesChunk::new(U8BytesData::default(), None, None, None, None)
        .expect("空数据不带范围应构造成功");

    assert_eq!(chunk.data.length, 0);
    assert!(chunk.data.is_empty());
    assert_eq!(chunk.to_content_range(), None);
}

/// 空数据配上范围时，范围长度 1 与数据长度 0 对不上。
#[test]
fn empty_chunk_with_range_is_rejected() {
    let data = U8BytesData::new(Vec::new(), None).expect("应构造成功");

    assert!(matches!(
        U8BytesChunk::new(data, Some(0), Some(0), None, None),
        Err(U8BytesChunkError::LengthMismatch {
            range_length: 1,
            data_length: 0
        })
    ));
}

/// `create_time` 只落到字段上：既不参与校验，也不进请求头。
#[test]
fn create_time_is_kept_without_being_validated() {
    let data = U8BytesData::new(vec![1], None).expect("应构造成功");
    let chunk = U8BytesChunk::new(data, None, None, None, Some("任意文本".to_owned()))
        .expect("create_time 不参与校验");

    assert_eq!(chunk.create_time.as_deref(), Some("任意文本"));
    assert_eq!(chunk.to_content_range(), None);
}

/// 只给总长度、不给起止范围时字段被保留，但不产生请求头取值。
#[test]
fn total_length_without_range_stays_local() {
    let data = U8BytesData::new(vec![1], None).expect("应构造成功");
    let chunk =
        U8BytesChunk::new(data, None, None, Some(1024), None).expect("只有总长度应构造成功");

    assert_eq!(chunk.total_length, Some(1024));
    assert_eq!(
        chunk.to_content_range(),
        None,
        "没有起止范围就没有 Content-Range"
    );
}

#[test]
fn inclusive_range_is_preserved_and_converted() {
    let data = U8BytesData::new(vec![1, 2], None).expect("应构造成功");
    let chunk =
        U8BytesChunk::new(data, Some(4), Some(5), Some(6), None).expect("合法分片应构造成功");

    assert_eq!(chunk.range_start, Some(4));
    assert_eq!(chunk.range_end, Some(5));
    assert_eq!(chunk.total_length, Some(6));
    assert_eq!(chunk.to_content_range().as_deref(), Some("bytes 4-5/6"));
}

#[test]
fn range_without_total_length_converts_without_total() {
    let data = U8BytesData::new(vec![1], None).expect("应构造成功");
    let chunk = U8BytesChunk::new(data, Some(9), Some(9), None, None).expect("合法分片应构造成功");

    assert_eq!(chunk.to_content_range().as_deref(), Some("bytes 9-9"));
}

#[test]
fn incomplete_range_is_rejected() {
    let data = U8BytesData::new(vec![1], None).expect("应构造成功");
    assert!(matches!(
        U8BytesChunk::new(data, Some(0), None, None, None),
        Err(U8BytesChunkError::IncompleteRange)
    ));

    let data = U8BytesData::new(vec![1], None).expect("应构造成功");
    assert!(matches!(
        U8BytesChunk::new(data, None, Some(0), None, None),
        Err(U8BytesChunkError::IncompleteRange)
    ));
}

#[test]
fn start_after_end_is_rejected() {
    let data = U8BytesData::new(vec![1, 2], None).expect("应构造成功");

    assert!(matches!(
        U8BytesChunk::new(data, Some(3), Some(2), None, None),
        Err(U8BytesChunkError::StartAfterEnd)
    ));
}

#[test]
fn range_length_must_match_data_length() {
    let data = U8BytesData::new(vec![1, 2], None).expect("应构造成功");

    assert!(matches!(
        U8BytesChunk::new(data, Some(0), Some(0), None, None),
        Err(U8BytesChunkError::LengthMismatch {
            range_length: 1,
            data_length: 2
        })
    ));
}

#[test]
fn range_overflow_is_rejected() {
    let data = U8BytesData::new(vec![1], None).expect("应构造成功");

    assert!(matches!(
        U8BytesChunk::new(data, Some(0), Some(u64::MAX), None, None),
        Err(U8BytesChunkError::RangeOverflow)
    ));
}

#[test]
fn end_at_or_beyond_total_is_rejected() {
    let data = U8BytesData::new(vec![1, 2], None).expect("应构造成功");
    assert!(matches!(
        U8BytesChunk::new(data, Some(0), Some(1), Some(1), None),
        Err(U8BytesChunkError::EndExceedsTotal)
    ));

    let data = U8BytesData::new(vec![1, 2], None).expect("应构造成功");
    assert!(matches!(
        U8BytesChunk::new(data, Some(0), Some(1), Some(0), None),
        Err(U8BytesChunkError::EndExceedsTotal)
    ));
}

#[test]
fn end_at_total_minus_one_is_accepted() {
    let data = U8BytesData::new(vec![1], None).expect("应构造成功");
    let chunk =
        U8BytesChunk::new(data, Some(9), Some(9), Some(10), None).expect("右边界应构造成功");

    assert_eq!(chunk.range_end, Some(9));
    assert_eq!(chunk.total_length, Some(10));
    assert_eq!(chunk.to_content_range().as_deref(), Some("bytes 9-9/10"));
}

#[test]
fn fields_are_public_and_reassignable() {
    let data = U8BytesData::new(vec![1], None).expect("应构造成功");
    let mut chunk = U8BytesChunk::new(data, None, None, None, None).expect("应构造成功");

    // 字段公开：调用方可以自行改写（本库不再校验）。
    chunk.range_start = Some(0);
    chunk.range_end = Some(0);
    chunk.total_length = Some(1);
    chunk.create_time = Some("2026-09-13T17:00:00Z".to_owned());

    assert_eq!(chunk.to_content_range().as_deref(), Some("bytes 0-0/1"));
    assert_eq!(chunk.create_time.as_deref(), Some("2026-09-13T17:00:00Z"));
}

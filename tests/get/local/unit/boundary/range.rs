//! 按段区间的边界取值。
//!
//! 这组用例固定 `range` 的输入边界：闭区间的两个端点、单字节区间、零长度区间、
//! `u64` 最大值，以及起点大于终点时的错误返回。区间是否被服务端接受不在本文件
//! 的范围内，那属于集成测试。

use webdav_raw::{GetBuilder, GetError};

use crate::support::fixtures::{base_url, client};

/// 用给定区间构建请求；失败时让用例失败。
fn request_with_range(start: u64, end: u64) -> webdav_raw::Request {
    GetBuilder::new(client(), base_url())
        .relative_path("big.bin")
        .expect("合法相对路径应被接受")
        .range(start, end)
        .expect("合法区间应被接受")
        .build()
        .expect("请求应构建成功")
}

/// 用给定区间调用 `range` 并取出错误。
fn range_error(start: u64, end: u64) -> GetError {
    match GetBuilder::new(client(), base_url()).range(start, end) {
        Ok(_) => panic!("区间 {start}-{end} 应返回错误"),
        Err(error) => error,
    }
}

/// 闭区间起点为 0 时，终点比段长小 1。
#[test]
fn range_from_zero_maps_end_as_inclusive() {
    let request = request_with_range(0, 1023);

    assert_eq!(
        request
            .headers()
            .get("range")
            .expect("Range 头应存在")
            .to_str()
            .expect("Range 头应为文本"),
        "bytes=0-1023"
    );
}

/// 起点等于终点表示只取一个字节，不是空区间。
#[test]
fn equal_bounds_request_single_byte() {
    let request = request_with_range(7, 7);

    assert_eq!(
        request
            .headers()
            .get("range")
            .expect("Range 头应存在")
            .to_str()
            .expect("Range 头应为文本"),
        "bytes=7-7"
    );
}

/// 起点非零时，区间写法不补前导零也不变形。
#[test]
fn non_zero_start_keeps_exact_bounds() {
    let request = request_with_range(1024, 2047);

    assert_eq!(
        request
            .headers()
            .get("range")
            .expect("Range 头应存在")
            .to_str()
            .expect("Range 头应为文本"),
        "bytes=1024-2047"
    );
}

/// `u64` 最大值可作端点，同时验证端点加法不会溢出。
#[test]
fn maximum_u64_bounds_do_not_overflow() {
    let request = request_with_range(u64::MAX, u64::MAX);

    assert_eq!(
        request
            .headers()
            .get("range")
            .expect("Range 头应存在")
            .to_str()
            .expect("Range 头应为文本"),
        format!("bytes={}-{}", u64::MAX, u64::MAX)
    );
}

/// 起点大于终点时返回 `Range` 错误，并带上原始两个端点。
#[test]
fn start_after_end_returns_range_error() {
    let error = range_error(10, 9);

    assert!(
        matches!(error, GetError::Range { start: 10, end: 9 }),
        "应为携带原始端点的 Range 错误，实际: {error:?}"
    );
}

/// 端点相差 1 的最小非法区间同样被拒。
#[test]
fn minimal_invalid_range_is_rejected() {
    let error = range_error(1, 0);

    assert!(
        matches!(error, GetError::Range { start: 1, end: 0 }),
        "应为 Range 错误，实际: {error:?}"
    );
}

/// `u64` 端点上起点大于终点也被拒，不因接近上限而放行。
#[test]
fn start_after_end_near_maximum_is_rejected() {
    let error = range_error(u64::MAX, u64::MAX - 1);

    assert!(
        matches!(error, GetError::Range { .. }),
        "应为 Range 错误，实际: {error:?}"
    );
}

/// 未调用 `range` 时不写 `Range` 头，请求保持无附加头。
#[test]
fn absent_range_leaves_no_range_header() {
    let request = GetBuilder::new(client(), base_url())
        .relative_path("file.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert!(request.headers().get("range").is_none());
}

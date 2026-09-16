//! Builder 请求头的内部路径与替换语义。
//!
//! `GetBuilder` 的请求头只有一个 `HeaderMap`，因此 `header`、`headers`、`range`
//! 三者写的是同一份存储。本文件按控制流逐条验证：先设后覆盖、批量合并、默认空表，
//! 以及构建时把表挂到请求上。

use webdav_core::{Client, GetBuilder, HeaderMap, HeaderValue, Url};

use crate::support::fixtures::{base_url, client};

/// 直接用值类型构造一个头表，供批量设置使用。
fn header_map_of(entries: &[(&'static str, &'static str)]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in entries {
        headers.insert(
            webdav_core::HeaderName::try_from(*name).expect("测试头名应合法"),
            HeaderValue::try_from(*value).expect("测试取值应合法"),
        );
    }
    headers
}

/// 未设置任何头时，构建出的请求不带附加头。
#[test]
fn default_builder_has_no_headers() {
    let request = GetBuilder::new(client(), base_url())
        .build()
        .expect("请求应构建成功");

    assert!(
        request.headers().is_empty(),
        "默认请求不应带任何头，实际: {:?}",
        request.headers()
    );
}

/// 单个头被完整挂到构建结果上，方法与地址不受影响。
#[test]
fn single_header_is_attached_to_request() {
    let request = GetBuilder::new(client(), base_url())
        .relative_path("file.bin")
        .expect("合法相对路径应被接受")
        .header("if-range", "\"v1\"")
        .expect("合法头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.method().as_str(), "GET");
    assert_eq!(request.url().as_str(), "https://example.com/dav/file.bin");
    assert_eq!(
        request
            .headers()
            .get("if-range")
            .expect("if-range 应存在")
            .to_str()
            .expect("应为文本"),
        "\"v1\""
    );
}

/// 同名头是替换语义：后设的覆盖先设的，不出现两个取值。
#[test]
fn later_header_replaces_earlier_same_name() {
    let request = GetBuilder::new(client(), base_url())
        .header("x-note", "first")
        .expect("合法头应被接受")
        .header("x-note", "second")
        .expect("合法头应被接受")
        .build()
        .expect("请求应构建成功");

    let values: Vec<&str> = request
        .headers()
        .get_all("x-note")
        .iter()
        .map(|value| value.to_str().expect("应为文本"))
        .collect();

    assert_eq!(values, vec!["second"]);
}

/// `header` 后设能覆盖 `range` 写的 `Range` 头。
#[test]
fn header_overrides_range_written_by_range_method() {
    let request = GetBuilder::new(client(), base_url())
        .range(0, 1023)
        .expect("合法区间应被接受")
        .header("range", "bytes=100-199")
        .expect("合法头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request
            .headers()
            .get("range")
            .expect("Range 应存在")
            .to_str()
            .expect("应为文本"),
        "bytes=100-199"
    );
}

/// `range` 后设能覆盖 `header` 写的 `Range` 头。
#[test]
fn range_method_overrides_header_written_by_header() {
    let request = GetBuilder::new(client(), base_url())
        .header("range", "bytes=100-199")
        .expect("合法头应被接受")
        .range(0, 1023)
        .expect("合法区间应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request
            .headers()
            .get("range")
            .expect("Range 应存在")
            .to_str()
            .expect("应为文本"),
        "bytes=0-1023"
    );
}

/// 空头表是合法输入，不改变已有头，也不清空已有头。
#[test]
fn empty_header_map_is_accepted_and_preserves_existing() {
    let request = GetBuilder::new(client(), base_url())
        .header("x-note", "kept")
        .expect("合法头应被接受")
        .headers(HeaderMap::new())
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request
            .headers()
            .get("x-note")
            .expect("原有头应保留")
            .to_str()
            .expect("应为文本"),
        "kept"
    );
}

/// 批量设置合并进同一份存储，且同名项被替换。
#[test]
fn headers_map_merges_and_replaces_by_name() {
    let request = GetBuilder::new(client(), base_url())
        .header("x-note", "first")
        .expect("合法头应被接受")
        .headers(header_map_of(&[("x-note", "second"), ("if-range", "\"v2\"")]))
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request
            .headers()
            .get("x-note")
            .expect("x-note 应存在")
            .to_str()
            .expect("应为文本"),
        "second"
    );
    assert_eq!(
        request
            .headers()
            .get("if-range")
            .expect("if-range 应存在")
            .to_str()
            .expect("应为文本"),
        "\"v2\""
    );
}

/// 批量设置同时写入 `Range` 与自定义头，两者互不覆盖。
#[test]
fn headers_map_can_carry_range_and_extra_header() {
    let request = GetBuilder::new(client(), base_url())
        .range(0, 1023)
        .expect("合法区间应被接受")
        .headers(header_map_of(&[("if-range", "\"v3\"")]))
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request
            .headers()
            .get("range")
            .expect("Range 应存在")
            .to_str()
            .expect("应为文本"),
        "bytes=0-1023"
    );
    assert_eq!(
        request
            .headers()
            .get("if-range")
            .expect("if-range 应存在")
            .to_str()
            .expect("应为文本"),
        "\"v3\""
    );
}

/// 同一个 Builder 连续 `build()` 两次，请求各自独立且头一致。
#[test]
fn repeated_build_keeps_headers_identical() {
    let builder = GetBuilder::new(client(), base_url())
        .range(0, 1023)
        .expect("合法区间应被接受");

    let first = builder.build().expect("首次构建应成功");
    let second = builder.build().expect("二次构建应成功");

    assert_eq!(
        first.headers().get("range"),
        second.headers().get("range")
    );
    assert_eq!(first.url(), second.url());
}

/// 用 `absolute_path` 设置地址时，请求头照样挂上。
#[test]
fn absolute_path_builder_still_carries_headers() {
    let request = GetBuilder::new(Client::new(), Url::parse("https://example.com/dav/").expect("合法地址"))
        .absolute_path("https://cdn.example.com/file.bin")
        .expect("合法地址应被接受")
        .range(1024, 2047)
        .expect("合法区间应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://cdn.example.com/file.bin");
    assert_eq!(
        request
            .headers()
            .get("range")
            .expect("Range 应存在")
            .to_str()
            .expect("应为文本"),
        "bytes=1024-2047"
    );
}

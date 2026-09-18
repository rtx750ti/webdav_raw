//! 响应头到能力模型的转换关系。
//!
//! 这里验证的是「协议文本 → 模型」这一方向的语义保持：拆分结果与原始取值一一对应，
//! 顺序不变；以及「模型 → 判断」这一层的语义——本库只给数据，不做判定。

use webdav_raw::{HeaderMap, HeaderValue, OptionsCapabilities};

/// 真实服务端常见的 `DAV` 取值能被完整还原，顺序保持不变。
#[test]
fn typical_dav_header_keeps_order() {
    let mut headers = HeaderMap::new();
    headers.insert("dav", HeaderValue::from_static("1, 2, 3"));

    let caps = OptionsCapabilities::from_headers(&headers);

    assert_eq!(caps.dav_levels.len(), 3);
    assert_eq!(caps.dav_levels[0], "1");
    assert_eq!(caps.dav_levels[1], "2");
    assert_eq!(caps.dav_levels[2], "3");
}

/// `Allow` 里的方法名按原样保留（服务端通常给大写）。
#[test]
fn allow_methods_are_kept_as_sent() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "allow",
        HeaderValue::from_static("OPTIONS, GET, HEAD, POST, DELETE, PROPFIND, PROPPATCH, MKCOL, COPY, MOVE"),
    );

    let caps = OptionsCapabilities::from_headers(&headers);

    assert_eq!(
        caps.allowed_methods,
        [
            "OPTIONS",
            "GET",
            "HEAD",
            "POST",
            "DELETE",
            "PROPFIND",
            "PROPPATCH",
            "MKCOL",
            "COPY",
            "MOVE"
        ]
    );
}

/// 拆分结果不引入额外的空白或空项，可以直接用于等值比较。
#[test]
fn split_result_has_no_padding() {
    let mut headers = HeaderMap::new();
    headers.insert("dav", HeaderValue::from_static(" 1 , 2 "));

    let caps = OptionsCapabilities::from_headers(&headers);

    for level in &caps.dav_levels {
        assert_eq!(level, level.trim(), "拆分结果不应带空白: {level:?}");
        assert!(!level.is_empty(), "拆分结果不应含空项");
    }
}

/// 库不替调用方判定能力：返回的就是原始声明，不追加也不删减。
#[test]
fn capabilities_are_not_normalized_or_filtered() {
    let mut headers = HeaderMap::new();
    // 故意混入未知级别与非标准方法，库应原样保留。
    headers.insert("dav", HeaderValue::from_static("1, 2, 3, 4"));
    headers.insert("allow", HeaderValue::from_static("GET, X-CUSTOM"));

    let caps = OptionsCapabilities::from_headers(&headers);

    assert_eq!(caps.dav_levels, ["1", "2", "3", "4"]);
    assert_eq!(caps.allowed_methods, ["GET", "X-CUSTOM"]);
}

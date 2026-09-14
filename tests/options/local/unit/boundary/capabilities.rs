//! 能力头解析的输入边界。
//!
//! `DAV` 与 `Allow` 的取值形态在实际服务端上差异很大：有的带空格、有的不带，
//! 有的只声明一个级别，有的重复发送同一个头。这里把能遇到的形态逐个钉住。

use webdav_core::{HeaderMap, HeaderValue, OptionsCapabilities};

/// 带空格的逗号分隔列表。
#[test]
fn comma_separated_with_spaces_is_split() {
    let mut headers = HeaderMap::new();
    headers.insert("dav", HeaderValue::from_static("1, 2, 3"));

    let caps = OptionsCapabilities::from_headers(&headers);

    assert_eq!(caps.dav_levels, ["1", "2", "3"]);
}

/// 不带空格的逗号分隔列表。
#[test]
fn comma_separated_without_spaces_is_split() {
    let mut headers = HeaderMap::new();
    headers.insert("dav", HeaderValue::from_static("1,2,3"));

    let caps = OptionsCapabilities::from_headers(&headers);

    assert_eq!(caps.dav_levels, ["1", "2", "3"]);
}

/// 单值头。
#[test]
fn single_value_is_kept() {
    let mut headers = HeaderMap::new();
    headers.insert("dav", HeaderValue::from_static("1"));

    let caps = OptionsCapabilities::from_headers(&headers);

    assert_eq!(caps.dav_levels, ["1"]);
}

/// 多余空格与空项被丢弃。
#[test]
fn extra_spaces_and_empty_items_are_dropped() {
    let mut headers = HeaderMap::new();
    headers.insert("dav", HeaderValue::from_static("  1 ,, 2 ,  ,3  "));

    let caps = OptionsCapabilities::from_headers(&headers);

    assert_eq!(caps.dav_levels, ["1", "2", "3"]);
}

/// 空取值得到空列表，而不是含空字符串的列表。
#[test]
fn empty_header_value_yields_empty_list() {
    let mut headers = HeaderMap::new();
    headers.insert("dav", HeaderValue::from_static(""));

    let caps = OptionsCapabilities::from_headers(&headers);

    assert!(caps.dav_levels.is_empty());
}

/// 只有逗号的取值同样得到空列表。
#[test]
fn only_commas_yields_empty_list() {
    let mut headers = HeaderMap::new();
    headers.insert("dav", HeaderValue::from_static(", ,,"));

    let caps = OptionsCapabilities::from_headers(&headers);

    assert!(caps.dav_levels.is_empty());
}

/// 响应头缺失时得到空列表，不报错。
#[test]
fn missing_headers_yield_empty_lists() {
    let headers = HeaderMap::new();

    let caps = OptionsCapabilities::from_headers(&headers);

    assert!(caps.dav_levels.is_empty());
    assert!(caps.allowed_methods.is_empty());
}

/// 同一个头重复发送时，所有取值都会被解析。
#[test]
fn repeated_header_values_are_all_parsed() {
    let mut headers = HeaderMap::new();
    headers.append("dav", HeaderValue::from_static("1, 2"));
    headers.append("dav", HeaderValue::from_static("3"));

    let caps = OptionsCapabilities::from_headers(&headers);

    assert_eq!(caps.dav_levels, ["1", "2", "3"]);
}

/// `DAV` 与 `Allow` 互相独立：只有一个存在时，另一个为空。
#[test]
fn headers_are_independent() {
    let mut only_dav = HeaderMap::new();
    only_dav.insert("dav", HeaderValue::from_static("1, 2"));

    let caps = OptionsCapabilities::from_headers(&only_dav);
    assert_eq!(caps.dav_levels, ["1", "2"]);
    assert!(caps.allowed_methods.is_empty());

    let mut only_allow = HeaderMap::new();
    only_allow.insert("allow", HeaderValue::from_static("GET, PUT"));

    let caps = OptionsCapabilities::from_headers(&only_allow);
    assert!(caps.dav_levels.is_empty());
    assert_eq!(caps.allowed_methods, ["GET", "PUT"]);
}

/// 响应头的名字大小写不敏感。
#[test]
fn header_names_are_case_insensitive() {
    let mut headers = HeaderMap::new();
    headers.insert("DAV", HeaderValue::from_static("1, 2"));
    headers.insert("Allow", HeaderValue::from_static("GET"));

    let caps = OptionsCapabilities::from_headers(&headers);

    assert_eq!(caps.dav_levels, ["1", "2"]);
    assert_eq!(caps.allowed_methods, ["GET"]);
}

/// 取值里的非 ASCII 内容按原样保留，不做大小写或内容加工。
#[test]
fn values_are_kept_verbatim() {
    let mut headers = HeaderMap::new();
    headers.insert("dav", HeaderValue::from_static("1, 2, extended-marker"));

    let caps = OptionsCapabilities::from_headers(&headers);

    assert_eq!(caps.dav_levels, ["1", "2", "extended-marker"]);
}

/// 默认构造就是两个空列表。
#[test]
fn default_is_empty() {
    let caps = OptionsCapabilities::default();

    assert_eq!(caps, OptionsCapabilities::from_headers(&HeaderMap::new()));
}

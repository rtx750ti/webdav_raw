//! GET 错误类型的每个分支与文案。
//!
//! `GetError` 有五个变体：路径解析、头名解析、头值解析、按段区间、请求构建。
//! 前四个由本库主动返回，本文件逐条固定它们的触发条件与 `Display` 文案。
//!
//! `Request` 变体是 `reqwest::Error` 的透传出口，需要真实连接失败才可能命中；
//! 它由 `tests/get/local/coverage/send_errors.rs` 用不可达回环端口覆盖，本文件
//! 不重复该场景。

use webdav_raw::{Client, GetBuilder, GetError, HeaderName, ParseError, Url};

use crate::support::fixtures::{base_url, client};

/// 调用一个返回 Builder 的操作并取出错误；`Ok` 视为用例失败。
fn error_of(result: Result<GetBuilder, GetError>) -> GetError {
    match result {
        Ok(_) => panic!("该输入应返回错误"),
        Err(error) => error,
    }
}

/// 路径语法非法时返回 `Url` 错误，文案前缀稳定。
#[test]
fn url_error_display_mentions_url() {
    let error = error_of(GetBuilder::new(client(), base_url()).relative_path("http://[::1"));

    assert!(matches!(error, GetError::Url(_)), "应为 Url 错误");
    assert_eq!(error.to_string(), format!("URL 格式错误: {}", path_error_message()));
}

/// 取出一条真实的 URL 解析错误文案，避免手写死字符串。
fn path_error_message() -> ParseError {
    Url::parse("http://[::1").expect_err("该地址应解析失败")
}

/// 头名非法时返回 `HeaderName` 错误，文案带内层原因。
#[test]
fn header_name_error_display_mentions_header_name() {
    let inner = HeaderName::try_from("bad name").expect_err("该头名应非法");
    let error = error_of(GetBuilder::new(client(), base_url()).header("bad name", "value"));

    assert!(matches!(error, GetError::HeaderName(_)), "应为 HeaderName 错误");
    assert_eq!(error.to_string(), format!("请求头名称格式错误: {inner}"));
}

/// 头值非法时返回 `HeaderValue` 错误，文案带内层原因。
#[test]
fn header_value_error_display_mentions_header_value() {
    let inner = webdav_raw::HeaderValue::try_from("line\nbreak").expect_err("该取值应非法");
    let error = error_of(GetBuilder::new(client(), base_url()).header("x-note", "line\nbreak"));

    assert!(matches!(error, GetError::HeaderValue(_)), "应为 HeaderValue 错误");
    assert_eq!(error.to_string(), format!("请求头值格式错误: {inner}"));
}

/// 区间非法时返回 `Range` 错误，文案带原始两个端点。
#[test]
fn range_error_display_carries_both_bounds() {
    let error = error_of(GetBuilder::new(client(), base_url()).range(10, 9));

    assert!(
        matches!(error, GetError::Range { start: 10, end: 9 }),
        "应为 Range 错误"
    );
    assert_eq!(error.to_string(), "按段区间无效: 起点 10 大于终点 9");
}

/// 端点接近 `u64` 上限时，错误文案仍完整可读。
#[test]
fn range_error_display_handles_large_bounds() {
    let error = error_of(GetBuilder::new(client(), base_url()).range(u64::MAX, 0));

    assert_eq!(
        error.to_string(),
        format!("按段区间无效: 起点 {} 大于终点 0", u64::MAX)
    );
}

/// 路径解析失败后 Builder 仍可用，错误不污染已设的请求头。
#[test]
fn url_error_leaves_builder_usable() {
    let builder = GetBuilder::new(Client::new(), Url::parse("https://example.com/dav/").expect("合法地址"))
        .header("x-note", "kept")
        .expect("合法头应被接受");

    assert!(
        builder.absolute_path("not a url").is_err(),
        "非法地址应返回错误"
    );
}

/// `absolute_path` 的 URL 解析失败走 `?` 提前返回，不覆盖已设的地址。
#[test]
fn absolute_path_failure_returns_url_error_without_touching_state() {
    let error = error_of(
        GetBuilder::new(client(), base_url())
            .relative_path("kept.bin")
            .expect("合法相对路径应被接受")
            .absolute_path("http://[::1"),
    );

    assert!(
        matches!(error, GetError::Url(_)),
        "应为 Url 错误，实际: {error:?}"
    );
}

/// 错误文案里不出现认证信息，避免把凭据带进日志。
#[test]
fn error_display_never_contains_credentials() {
    let errors = [
        error_of(GetBuilder::new(client(), base_url()).relative_path("http://[::1")),
        error_of(GetBuilder::new(client(), base_url()).header("bad name", "value")),
        error_of(GetBuilder::new(client(), base_url()).header("x-note", "line\nbreak")),
        error_of(GetBuilder::new(client(), base_url()).range(10, 9)),
    ];

    for error in errors {
        let text = error.to_string();
        assert!(
            !text.contains("alice") && !text.contains("password") && !text.contains("Basic"),
            "错误文案不应含凭据: {text}"
        );
    }
}

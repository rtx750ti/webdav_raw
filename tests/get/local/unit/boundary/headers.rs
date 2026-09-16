//! 附加请求头的边界输入。
//!
//! `header` 接收字符串，因此名称和取值都需要现场解析，非法输入必须返回错误而不是
//! panic。本文件固定「哪些名称和取值合法、哪些被拒」，并确认被拒的输入没有产生
//! 任何副作用。

use webdav_core::{GetBuilder, GetError};

use crate::support::fixtures::{base_url, client};

/// 调用 `header` 并返回结果；发生 panic 时让用例失败。
fn header(name: &str, value: &str) -> Result<GetBuilder, GetError> {
    match std::panic::catch_unwind(|| {
        GetBuilder::new(client(), base_url()).header(name, value)
    }) {
        Ok(result) => result,
        Err(_) => panic!("header({name:?}, {value:?}) 不应 panic，应返回 Err"),
    }
}

/// 取出错误，失败信息里不打印 `GetBuilder`（它没有实现 `Debug`）。
fn error_of(result: Result<GetBuilder, GetError>, input: &str) -> GetError {
    match result {
        Ok(_) => panic!("{input:?} 应返回错误"),
        Err(error) => error,
    }
}

/// 日常请求头名称和取值都被接受。
#[test]
fn ordinary_headers_are_accepted() {
    let cases = [
        ("if-range", "\"83cc00-63d3371160718\""),
        ("if-none-match", "W/\"weak\""),
        ("x-note", "第一次下载"),
        ("accept-encoding", "identity"),
        ("Range", "bytes=0-1023"),
    ];

    for (name, value) in cases {
        assert!(
            header(name, value).is_ok(),
            "应被接受: {name}: {value}"
        );
    }
}

/// 取值里的非 ASCII 字节在多字节 UTF-8 下按原样写入，不会被拒绝。
///
/// 这类取值不是合法 HTTP 头文本（`to_str()` 会失败），但 `HeaderValue::from_str`
/// 接受任意字节串，因此本库也照收不误——是否可用由服务端决定。
#[test]
fn multi_byte_header_value_is_stored_as_bytes() {
    let request = header("x-note", "中文取值")
        .expect("多字节取值应被接受")
        .build()
        .expect("请求应构建成功");

    let stored = request
        .headers()
        .get("x-note")
        .expect("自定义头应存在");

    assert_eq!(stored.as_bytes(), "中文取值".as_bytes());
    assert!(stored.to_str().is_err(), "该取值不是合法头文本");
}

/// 含空格或控制字符的名称返回 `HeaderName` 错误。
#[test]
fn invalid_header_name_returns_header_name_error() {
    for name in ["bad name", "bad\tname", "bad\nname", ""] {
        let error = error_of(header(name, "value"), name);
        assert!(
            matches!(error, GetError::HeaderName(_)),
            "{name:?} 应为 HeaderName 错误，实际: {error:?}"
        );
    }
}

/// 含换行或回车等控制字符的取值返回 `HeaderValue` 错误。
#[test]
fn invalid_header_value_returns_header_value_error() {
    for value in ["line\nbreak", "carriage\rreturn", "null\0byte"] {
        let error = error_of(header("x-note", value), value);
        assert!(
            matches!(error, GetError::HeaderValue(_)),
            "{value:?} 应为 HeaderValue 错误，实际: {error:?}"
        );
    }
}

/// 非法头名被拒后不影响后续合法设置。
#[test]
fn rejected_header_leaves_builder_usable() {
    let builder = match header("bad name", "value") {
        Ok(_) => panic!("非法头名应返回错误"),
        Err(_) => GetBuilder::new(client(), base_url()),
    };
    let request = builder
        .relative_path("file.txt")
        .expect("合法相对路径应被接受")
        .header("x-note", "ok")
        .expect("合法头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request
            .headers()
            .get("x-note")
            .expect("自定义头应存在")
            .to_str()
            .expect("自定义头应为文本"),
        "ok"
    );
    assert!(request.headers().get("bad name").is_none());
}

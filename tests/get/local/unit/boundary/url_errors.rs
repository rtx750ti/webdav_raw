//! 非法路径返回错误而不是 panic。
//!
//! 这组用例来自一次真实修正：`relative_url` / `absolute_url` 原先用 `expect` 处理
//! URL 解析失败，调用方传入语法非法的路径会直接 panic。现在两个方法都返回
//! `Result`，本文件把「哪些输入合法、哪些返回错误」固定下来。
//!
//! 用 `std::panic::catch_unwind` 兜底：只要还有输入能触发 panic，用例就会失败。

use webdav_raw::GetBuilder;
use webdav_raw::{Client, GetError, Url};

use crate::support::fixtures::base_url;

/// 调用 `relative_url` 并返回结果；发生 panic 时让用例失败。
fn relative_url(path: &str) -> Result<GetBuilder, GetError> {
    match std::panic::catch_unwind(|| {
        GetBuilder::new(Client::new(), base_url()).relative_path(path)
    }) {
        Ok(result) => result,
        Err(_) => panic!("relative_url({path:?}) 不应 panic，应返回 Err"),
    }
}

/// 调用 `absolute_url` 并返回结果；发生 panic 时让用例失败。
fn absolute_url(url: &str) -> Result<GetBuilder, GetError> {
    match std::panic::catch_unwind(|| {
        GetBuilder::new(Client::new(), base_url()).absolute_path(url)
    }) {
        Ok(result) => result,
        Err(_) => panic!("absolute_url({url:?}) 不应 panic，应返回 Err"),
    }
}

/// 取出错误，失败信息里不打印 `GetBuilder`（它没有实现 `Debug`）。
fn url_error_of(result: Result<GetBuilder, GetError>, input: &str) -> GetError {
    match result {
        Ok(_) => panic!("{input:?} 应返回错误"),
        Err(error) => error,
    }
}

/// 日常路径都不会返回错误，也不会 panic。
#[test]
fn ordinary_paths_are_accepted() {
    let paths = [
        "",
        "file.txt",
        "目录/报告 2026.pdf",
        "Documents/report.pdf",
        "/absolute/from-host-root.txt",
        "a//b///c",
        "a/./b/../c",
        "../sibling.txt",
        "已编码/%E6%8A%A5%E5%91%8A.pdf",
    ];

    for path in paths {
        assert!(
            relative_url(path).is_ok(),
            "日常路径应被接受: {path:?}"
        );
    }
}

/// 语法非法的输入返回 `Url` 错误，而不是 panic。
#[test]
fn malformed_input_returns_url_error() {
    for value in ["http://[::1", "http://["] {
        let error = url_error_of(relative_url(value), value);
        assert!(
            matches!(error, GetError::Url(_)),
            "应为 Url 错误，实际: {error:?}"
        );
    }
}

/// `absolute_url` 对合法地址成功、对非法地址返回错误。
#[test]
fn absolute_url_reports_errors_instead_of_panicking() {
    assert!(absolute_url("https://example.com/file.txt").is_ok());

    for value in ["not a url", "http://[::1"] {
        let error = url_error_of(absolute_url(value), value);
        assert!(
            matches!(error, GetError::Url(_)),
            "应为 Url 错误，实际: {error:?}"
        );
    }
}

/// 空字符串是合法输入，落回认证根地址。
#[test]
fn empty_path_keeps_base_url() {
    let request = relative_url("")
        .expect("空路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), base_url().as_str());
    assert_eq!(
        Url::parse("https://example.com/dav/").expect("应有效").as_str(),
        "https://example.com/dav/"
    );
}

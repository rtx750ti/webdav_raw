//! 目标地址设置的边界：编码、根前缀覆盖与非法输入。

use webdav_raw::{DeleteBuilder, DeleteError};

use crate::support::fixtures::{base_url, client};

/// 空相对路径退回认证根地址本身。
#[test]
fn empty_target_path_keeps_auth_root() {
    let request = DeleteBuilder::new(client(), base_url())
        .target_path("")
        .expect("空相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/dav/");
}

/// 相对路径中的空格与中文被百分号编码。
#[test]
fn target_path_encodes_spaces_and_unicode() {
    let request = DeleteBuilder::new(client(), base_url())
        .target_path("我的 目录/报告.pdf")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.url().as_str(),
        "https://example.com/dav/%E6%88%91%E7%9A%84%20%E7%9B%AE%E5%BD%95/%E6%8A%A5%E5%91%8A.pdf"
    );
}

/// 以斜杠开头的相对路径从主机根开始，覆盖认证根里已有的前缀。
#[test]
fn leading_slash_target_path_replaces_auth_root_prefix() {
    let request = DeleteBuilder::new(client(), base_url())
        .target_path("/other/file.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/other/file.txt");
}

/// 点路径与重复斜杠遵循 `Url` 的既有拼接语义。
#[test]
fn target_path_normalizes_dot_segments_and_keeps_repeated_slashes() {
    let normalized = DeleteBuilder::new(client(), base_url())
        .target_path("a/./b/../c")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");
    let repeated = DeleteBuilder::new(client(), base_url())
        .target_path("a//b///c")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(normalized.url().as_str(), "https://example.com/dav/a/c");
    assert_eq!(repeated.url().as_str(), "https://example.com/dav/a//b///c");
}

/// 完整 URL 也能通过 `target_path` 设置，等价于 `target_url`。
#[test]
fn target_path_accepts_absolute_url() {
    let request = DeleteBuilder::new(client(), base_url())
        .target_path("https://example.com/other.txt")
        .expect("完整 URL 应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/other.txt");
}

/// 相对路径参数不接受语法非法的输入。
#[test]
fn target_path_rejects_malformed_input() {
    let error = DeleteBuilder::new(client(), base_url())
        .target_path("http://[::1")
        .expect_err("非法 URL 应被拒绝");

    assert!(matches!(error, DeleteError::Url(_)));
}

/// `target_url` 接受完整 URL，不使用认证根地址。
#[test]
fn target_url_replaces_auth_root() {
    let request = DeleteBuilder::new(client(), base_url())
        .target_url("https://other.example.com/file.txt")
        .expect("合法绝对地址应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.url().as_str(),
        "https://other.example.com/file.txt"
    );
}

/// `target_url` 同样拒绝语法非法的输入。
#[test]
fn target_url_rejects_malformed_input() {
    let error = DeleteBuilder::new(client(), base_url())
        .target_url("http://[::1")
        .expect_err("非法 URL 应被拒绝");

    assert!(matches!(error, DeleteError::Url(_)));
}

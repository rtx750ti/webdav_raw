//! 目标地址设置的边界：编码、根前缀覆盖与非法输入。

use webdav_core::{MkcolBuilder, MkcolError};

use crate::support::fixtures::{base_url, client};

/// 空相对路径退回认证根地址本身。
#[test]
fn empty_target_path_keeps_auth_root() {
    let request = MkcolBuilder::new(client(), base_url())
        .target_path("")
        .expect("空相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/dav/");
}

/// 相对路径中的空格与中文被百分号编码。
#[test]
fn target_path_encodes_spaces_and_unicode() {
    let request = MkcolBuilder::new(client(), base_url())
        .target_path("新建 目录/")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.url().as_str(),
        "https://example.com/dav/%E6%96%B0%E5%BB%BA%20%E7%9B%AE%E5%BD%95/"
    );
}

/// 以斜杠开头的相对路径从主机根开始，覆盖认证根里已有的前缀。
#[test]
fn leading_slash_target_path_replaces_auth_root_prefix() {
    let request = MkcolBuilder::new(client(), base_url())
        .target_path("/other/dir/")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/other/dir/");
}

/// 点路径与重复斜杠遵循 `Url` 的既有拼接语义。
#[test]
fn target_path_normalizes_dot_segments_and_keeps_repeated_slashes() {
    let normalized = MkcolBuilder::new(client(), base_url())
        .target_path("a/./b/../c/")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");
    let repeated = MkcolBuilder::new(client(), base_url())
        .target_path("a//b///c/")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(normalized.url().as_str(), "https://example.com/dav/a/c/");
    assert_eq!(repeated.url().as_str(), "https://example.com/dav/a//b///c/");
}

/// 完整 URL 也能通过 `target_path` 设置。
#[test]
fn target_path_accepts_absolute_url() {
    let request = MkcolBuilder::new(client(), base_url())
        .target_path("https://example.com/other/")
        .expect("完整 URL 应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/other/");
}

/// 语法非法的输入返回 `Url` 错误。
#[test]
fn target_path_rejects_malformed_input() {
    let error = MkcolBuilder::new(client(), base_url())
        .target_path("http://[::1")
        .expect_err("非法 URL 应被拒绝");

    assert!(matches!(error, MkcolError::Url(_)));
}

/// `target_url` 接受完整 URL，不使用认证根地址。
#[test]
fn target_url_replaces_auth_root() {
    let request = MkcolBuilder::new(client(), base_url())
        .target_url("https://other.example.com/dir/")
        .expect("合法绝对地址应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://other.example.com/dir/");
}

/// `target_url` 同样拒绝语法非法的输入。
#[test]
fn target_url_rejects_malformed_input() {
    let error = MkcolBuilder::new(client(), base_url())
        .target_url("not a url")
        .expect_err("非法 URL 应被拒绝");

    assert!(matches!(error, MkcolError::Url(_)));
}

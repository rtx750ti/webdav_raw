use webdav_core::GetBuilder;

use crate::support::{base_url, client};

/// 验证相对路径保留根路径并对空格和中文进行 URL 编码。
#[test]
fn relative_url_encodes_spaces_and_unicode() {
    let request = GetBuilder::new(client(), base_url())
        .relative_url("我的 文档/报告.pdf".to_owned())
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.url().as_str(),
        "https://example.com/dav/%E6%88%91%E7%9A%84%20%E6%96%87%E6%A1%A3/%E6%8A%A5%E5%91%8A.pdf"
    );
}

/// 验证点路径和连续斜杠遵循 Url 拼接语义。
#[test]
fn relative_url_normalizes_dot_segments_and_preserves_repeated_slashes() {
    let normalized = GetBuilder::new(client(), base_url())
        .relative_url("a/./b/../c".to_owned())
        .build()
        .expect("请求应构建成功");
    let repeated = GetBuilder::new(client(), base_url())
        .relative_url("a//b///c".to_owned())
        .build()
        .expect("请求应构建成功");

    assert_eq!(normalized.url().as_str(), "https://example.com/dav/a/c");
    assert_eq!(repeated.url().as_str(), "https://example.com/dav/a//b///c");
}

/// 验证以斜杠开头的路径从主机根路径开始。
#[test]
fn leading_slash_relative_url_replaces_base_path() {
    let request = GetBuilder::new(client(), base_url())
        .relative_url("/other/file.txt".to_owned())
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/other/file.txt");
}

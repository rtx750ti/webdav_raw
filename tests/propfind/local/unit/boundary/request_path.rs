use webdav_raw::PropFindBuilder;
use webdav_raw::{Client, Url};

/// 验证 PROPFIND 路径保留根路径并处理编码和点路径。
#[test]
fn request_path_encodes_unicode_and_normalizes_dot_segments() {
    let base_url = Url::parse("https://example.com/webdav/").expect("固定地址必须有效");
    let unicode_request = PropFindBuilder::new(Client::new(), base_url.clone())
        .path("我的 文档/报告.pdf")
        .build()
        .expect("请求应构建成功");
    let dot_request = PropFindBuilder::new(Client::new(), base_url)
        .path("a/./b/../c")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        unicode_request.url().as_str(),
        "https://example.com/webdav/%E6%88%91%E7%9A%84%20%E6%96%87%E6%A1%A3/%E6%8A%A5%E5%91%8A.pdf"
    );
    assert_eq!(dot_request.url().as_str(), "https://example.com/webdav/a/c");
}

/// 验证空路径、前导斜杠和连续斜杠的 URL 拼接结果。
#[test]
fn request_path_handles_empty_leading_and_repeated_slashes() {
    let base_url = Url::parse("https://example.com/webdav/").expect("固定地址必须有效");
    let empty = PropFindBuilder::new(Client::new(), base_url.clone())
        .build()
        .expect("请求应构建成功");
    let leading = PropFindBuilder::new(Client::new(), base_url.clone())
        .path("/other")
        .build()
        .expect("请求应构建成功");
    let repeated = PropFindBuilder::new(Client::new(), base_url)
        .path("a//b///c")
        .build()
        .expect("请求应构建成功");

    assert_eq!(empty.url().as_str(), "https://example.com/webdav/");
    assert_eq!(leading.url().as_str(), "https://example.com/other");
    assert_eq!(
        repeated.url().as_str(),
        "https://example.com/webdav/a//b///c"
    );
}

use webdav_core::GetBuilder;

use crate::support::fixtures::{base_url, client};

/// 验证绝对地址会覆盖认证根地址。
#[test]
fn absolute_url_replaces_base_url() {
    let request = GetBuilder::new(client(), base_url())
        .absolute_url("https://download.example.com/file.txt".to_owned())
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.url().as_str(),
        "https://download.example.com/file.txt"
    );
}

/// 验证绝对地址保留非标准端口。
#[test]
fn absolute_url_preserves_non_standard_port() {
    let request = GetBuilder::new(client(), base_url())
        .absolute_url("https://download.example.com:8443/file.txt".to_owned())
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.url().as_str(),
        "https://download.example.com:8443/file.txt"
    );
}

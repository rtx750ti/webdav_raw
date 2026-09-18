use webdav_raw::GetBuilder;

use crate::support::fixtures::{base_url, client};

/// 验证未设置路径时，GET Builder 使用传入的认证根地址。
#[test]
fn default_target_uses_base_url() {
    let base_url = base_url();
    let request = GetBuilder::new(client(), base_url.clone())
        .build()
        .expect("默认请求应构建成功");

    assert_eq!(request.url(), &base_url);
}

/// 验证后一次路径设置会覆盖前一次设置。
#[test]
fn later_url_setting_overrides_previous_setting() {
    let request = GetBuilder::new(client(), base_url())
        .relative_path("first.txt")
        .expect("合法路径应被接受")
        .absolute_path("https://example.com/second.txt")
        .expect("合法路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/second.txt");
}

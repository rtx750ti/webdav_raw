//! 真实服务的 HEAD 验收（默认不运行）。
//!
//! HEAD 是只读方法，不修改远端任何内容。本用例只对**已存在的只读测试资源**和
//! **确定不存在的路径**发请求，并核对 HEAD 与 GET 对同一资源的元数据一致。
//!
//! 运行方式：
//!
//! ```text
//! cargo test --features network-test --test head -- --ignored --nocapture
//! ```
//!
//! 需要 `WEBDAV_URL`、`WEBDAV_ACCOUNT`、`WEBDAV_PASSWORD`。输出里不会出现账号、
//! 密码或 Authorization。

use webdav_core::WebdavAuth;

use crate::common::network_config;

/// 受控环境中已存在且只读的测试文件路径。
const EXISTING_FILE: &str = "测试文件夹/fast-sync.exe";

/// 验证真实服务的 HEAD 能读到元数据，且与 GET 报告的长度一致。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn head_reports_metadata_consistent_with_get() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let head_response = auth
        .head()
        .target_path(EXISTING_FILE)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");
    assert_eq!(head_response.status(), 200);

    // 先把要用的头部读出来，再消费响应体——`text()` 会拿走整个响应。
    let head_length = head_response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let head_body = head_response.text().await.expect("响应体应可读");
    assert!(head_body.is_empty(), "HEAD 响应体必须为空");
    assert!(head_length.is_some(), "HEAD 应返回 Content-Length");

    let get_response = auth
        .get()
        .relative_url(EXISTING_FILE.to_owned())
        .send()
        .await
        .expect("GET 应发送成功");
    assert_eq!(get_response.status(), 200);

    let get_length = get_response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    assert_eq!(
        head_length, get_length,
        "HEAD 与 GET 对同一资源应报告相同的 Content-Length"
    );
    assert!(
        get_length
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|length| length > 0),
        "已存在的测试文件应有正的长度"
    );
}

/// 验证真实服务对不存在路径返回非成功状态。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn head_missing_path_returns_non_success_status() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let response = auth
        .head()
        .target_path("__webdav_core_missing_file__")
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("请求应返回原始响应");

    assert!(!response.status().is_success());
}

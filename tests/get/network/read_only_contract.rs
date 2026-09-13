use webdav_core::WebdavAuth;

use crate::common::network_config;

/// 受控环境中已存在且只读的测试文件路径。
const EXISTING_FILE: &str = "测试文件夹/fast-sync.exe";

/// 验证真实服务可以读取已存在文件的状态和 Content-Length。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn download_existing_returns_status_and_content_length() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let response = auth
        .get()
        .relative_url(EXISTING_FILE.to_owned())
        .send()
        .await
        .expect("下载请求应发送成功");
    let content_length = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    assert_eq!(response.status(), 200);
    assert!(content_length.is_some_and(|length| length > 0));
}

/// 验证真实服务对不存在路径返回非成功状态。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn download_missing_returns_non_success_status() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let response = auth
        .get()
        .relative_url("__webdav_core_missing_file__".to_owned())
        .send()
        .await
        .expect("请求应返回原始响应");

    assert!(!response.status().is_success());
}

/// 验证错误凭据返回认证失败响应且不输出凭据。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn download_wrong_auth_returns_auth_failure_status() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, "invalid-password", config.url.as_str())
        .expect("网络测试地址应有效");

    let response = auth
        .get()
        .relative_url(EXISTING_FILE.to_owned())
        .send()
        .await
        .expect("请求应返回原始响应");

    assert!(response.status().as_u16() == 401 || response.status().as_u16() == 403);
}

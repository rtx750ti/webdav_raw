//! 真实服务的 OPTIONS 验收（默认不运行）。
//!
//! OPTIONS 是只读方法，不改动远端任何内容。本用例只读服务端的能力声明。
//!
//! 运行方式：
//!
//! ```text
//! cargo test --features network-test --test options -- --ignored --nocapture
//! ```
//!
//! 需要 `WEBDAV_URL`、`WEBDAV_ACCOUNT`、`WEBDAV_PASSWORD`。输出里不会出现账号、
//! 密码或 Authorization。

use webdav_raw::WebdavAuth;

use crate::common::network_config;

/// 验证真实服务的 OPTIONS 声明了非空的 DAV 级别。
///
/// 按 RFC 4918 §10.1，兼容 WebDAV 的服务端至少应声明 `1`。这里只断言「非空」，
/// 不逐项断言具体级别——那取决于服务端实现。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn options_reports_non_empty_dav_levels() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let (response, caps) = auth
        .options()
        .send_capabilities()
        .await
        .expect("OPTIONS 应发送成功");

    assert!(
        response.status().is_success(),
        "OPTIONS 应成功，实际: {}",
        response.status()
    );
    assert!(
        !caps.dav_levels.is_empty(),
        "WebDAV 服务端应声明至少一个 DAV 级别"
    );
    assert!(
        caps.dav_levels.iter().any(|level| level == "1"),
        "RFC 4918 要求兼容服务端声明 DAV: 1，实际声明: {:?}",
        caps.dav_levels
    );
}

/// 验证原始入口能看到未加工的能力头。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn raw_options_response_exposes_capability_headers() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let response = auth.options().send().await.expect("OPTIONS 应发送成功");

    assert!(response.status().is_success());
    assert!(
        response.headers().get("dav").is_some(),
        "原始响应里应能看到 DAV 头"
    );
    assert!(
        response.headers().get("allow").is_some(),
        "原始响应里应能看到 Allow 头"
    );
}

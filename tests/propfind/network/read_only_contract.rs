use webdav_raw::WebdavAuth;
use webdav_raw::{Depth, PropFindError};

use crate::common::network_config;

/// 验证真实服务的目录 PROPFIND；仅在受控环境手动执行。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn list_directory_returns_multi_status() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let multistatus = auth
        .propfind()
        .depth(Depth::One)
        .send_and_deserialize()
        .await
        .expect("目录 PROPFIND 应成功");

    assert!(!multistatus.response.is_empty());
}

/// 验证真实服务接受 Depth=0 的只读 PROPFIND。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn list_depth_zero_returns_multi_status() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let multistatus = auth
        .propfind()
        .depth(Depth::Zero)
        .send_and_deserialize()
        .await
        .expect("Depth=0 PROPFIND 应成功");

    assert!(!multistatus.response.is_empty());
}

/// 验证真实服务接受 Depth=1 的只读 PROPFIND。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn list_depth_one_returns_multi_status() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let multistatus = auth
        .propfind()
        .depth(Depth::One)
        .send_and_deserialize()
        .await
        .expect("Depth=1 PROPFIND 应成功");

    assert!(!multistatus.response.is_empty());
}

/// 验证不存在路径返回协议状态错误。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn missing_path_returns_unexpected_status() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let result = auth
        .propfind()
        .path("__webdav_raw_missing_path__")
        .depth(Depth::Zero)
        .send_and_deserialize()
        .await;

    assert!(matches!(result, Err(PropFindError::UnexpectedStatus(_))));
}

/// 验证错误凭据返回协议状态错误且不输出凭据。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn wrong_auth_returns_unexpected_status() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, "invalid-password", config.url.as_str())
        .expect("网络测试地址应有效");

    let result = auth
        .propfind()
        .depth(Depth::Zero)
        .send_and_deserialize()
        .await;

    assert!(matches!(result, Err(PropFindError::UnexpectedStatus(_))));
}

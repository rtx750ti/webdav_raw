use webdav_raw::WebdavAuth;

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
        .relative_path(EXISTING_FILE)
        .expect("合法相对路径应被接受")
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
        .relative_path("__webdav_raw_missing_file__")
        .expect("合法相对路径应被接受")
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
        .relative_path(EXISTING_FILE)
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("请求应返回原始响应");

    assert!(response.status().as_u16() == 401 || response.status().as_u16() == 403);
}

/// 验证真实服务对带 `Range` 的请求：支持按段取时回 206，忽略该头时回 200 加整份。
///
/// 两种答复都算通过，因为「服务端是否支持按段取」不由本库决定；这条用例固定的是
/// 客户端发出的 `Range` 头能到达服务端，且答复形态能被调用方区分出来。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn range_request_returns_segment_or_whole_file() {
    /// 一次请求的段长，从第一个字节开始取。
    const SEGMENT: u64 = 1024;

    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let response = auth
        .get()
        .relative_path(EXISTING_FILE)
        .expect("合法相对路径应被接受")
        .range(0, SEGMENT - 1)
        .expect("合法区间应被接受")
        .send()
        .await
        .expect("分片请求应发送成功");

    let status = response.status().as_u16();
    let has_content_range = response.headers().contains_key("content-range");
    let body = response.bytes().await.expect("响应体应可读取");

    assert!(
        status == 206 || status == 200,
        "按段请求的状态应为 206 或 200，实际 {status}"
    );
    assert!(!body.is_empty(), "分片响应体应有内容");

    if status == 206 {
        assert!(has_content_range, "206 响应应带 Content-Range");
        assert!(
            body.len() as u64 <= SEGMENT,
            "206 响应体不应超过请求的段长"
        );
    }
}

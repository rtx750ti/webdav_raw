//! 真实 WebDAV 服务的 PUT 验收（默认不运行）。
//!
//! 按 `docs/测试/测试规范.md`，真实网络测试同时使用 `network-test` 功能开关与
//! `#[ignore]`，只在受控环境人工验收时执行。
//!
//! PUT 会修改远端资源，因此本测试默认只做"能否建立并发出请求"的最小验收，
//! 不做写入后核对；真正的写读校验留给人工在受控环境确认。
//!
//! 运行方式：
//!
//! ```text
//! cargo test --features network-test --test put -- --ignored
//! ```

#[path = "../../common/network_config.rs"]
mod network_config;

use webdav_core::auth::WebdavAuth;
use webdav_core::put::put_body::u8_bytes::U8Metadata;
use webdav_core::put::put_body::u8_bytes_data::U8BytesData;
use webdav_core::put::put_body::{PutBody, u8_bytes::U8Bytes};

/// 用真实账号向受控服务发送一次 PUT，确认认证与请求构建可用。
#[tokio::test]
#[ignore = "会向受控 WebDAV 服务写入一个临时资源"]
async fn real_put_sends_request_with_configured_credentials() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址必须有效");

    let data = U8BytesData::new(b"webdav-core put acceptance".to_vec(), None)
        .expect("验收数据必须能构造成功");
    let metadata = U8Metadata::from_name("__webdav_core_put_acceptance.txt".to_owned())
        .expect("内容类型推断必须成功");

    let response = auth
        .put()
        .relative_path("__webdav_core_put_acceptance.txt")
        .expect("相对路径必须有效")
        .body(PutBody::from_bytes(U8Bytes::new(data, metadata)))
        .send()
        .await
        .expect("真实 PUT 必须发送成功");

    assert!(
        response.status().is_success(),
        "真实服务应接受这次 PUT，实际状态 {}",
        response.status()
    );
}

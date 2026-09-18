//! COPY 领域专用夹具。

use webdav_raw::{Client, Url};

/// 固定的认证根地址，用于不发网络请求的 Builder 测试。
pub fn base_url() -> Url {
    Url::parse("https://example.com/dav/").expect("固定根地址必须合法")
}

/// 未配置任何默认头的 Client。
pub fn client() -> Client {
    Client::new()
}

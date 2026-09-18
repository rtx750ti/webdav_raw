use webdav_raw::{Client, Url};

/// 创建不带外部状态的 GET 测试客户端。
pub fn client() -> Client {
    Client::new()
}

/// 创建固定的 WebDAV 根地址。
pub fn base_url() -> Url {
    Url::parse("https://example.com/dav/").expect("固定测试地址必须有效")
}

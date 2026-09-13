use webdav_core::Url;
use wiremock::MockServer;

/// 启动只监听本机回环地址的 HTTP MockServer。
pub async fn start_server() -> MockServer {
    MockServer::start().await
}

/// 为 MockServer 生成带 WebDAV 根路径且以斜杠结尾的基础地址。
pub fn base_url(server: &MockServer, root_path: &str) -> Url {
    let root_path = root_path.trim_matches('/');
    Url::parse(&format!("{}/{root_path}/", server.uri())).expect("MockServer 地址必须是合法 URL")
}

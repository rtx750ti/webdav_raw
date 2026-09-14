use std::env;

use webdav_core::Url;

/// 真实网络验收所需的 WebDAV 配置。
///
/// DELETE 会改动远端，因此额外要求一个**专用于写测试的临时目录**：
/// 所有删除都只在这个目录之内发生，服务端既有内容不参与。
pub struct NetworkConfig {
    pub url: Url,
    pub account: String,
    pub password: String,
}

/// 仅在已启用且手动执行的网络测试中读取环境变量。
pub fn read_network_config() -> Result<NetworkConfig, env::VarError> {
    let raw_url = env::var("WEBDAV_URL")?;
    let account = env::var("WEBDAV_ACCOUNT")?;
    let password = env::var("WEBDAV_PASSWORD")?;
    let normalized_url = if raw_url.ends_with('/') {
        raw_url
    } else {
        format!("{raw_url}/")
    };

    Ok(NetworkConfig {
        url: Url::parse(&normalized_url).expect("WEBDAV_URL 必须是合法 URL"),
        account,
        password,
    })
}

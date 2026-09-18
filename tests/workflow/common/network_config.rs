use std::env;

use webdav_raw::Url;

/// 真实网络验收所需的 WebDAV 配置。
pub struct NetworkConfig {
    pub url: Url,
    pub account: String,
    pub password: String,
}

/// 仅在已启用且手动执行的网络测试中读取环境变量。
///
/// 注意：`WEBDAV_PASSWORD` 可能只存在于 Windows 的 User 作用域而没有进入父进程
/// 环境，此时子进程读不到。运行前需要显式注入，例如：
///
/// ```text
/// $env:WEBDAV_PASSWORD = [Environment]::GetEnvironmentVariable('WEBDAV_PASSWORD','User')
/// ```
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

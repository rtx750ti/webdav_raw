use std::{fmt, sync::Arc};

use base64::Engine;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client,
};
use sha2::{Digest, Sha256};
use url::Url;

/// WebDAV 认证相关错误
#[derive(Debug, thiserror::Error)]
pub enum WebdavAuthError {
    #[error("base url 不能为空")]
    EmptyBaseUrl,

    #[error("base url 格式错误: {0}")]
    InvalidBaseUrl(#[source] url::ParseError),

    #[error("base url 必须包含主机地址")]
    MissingHost,

    #[error("base url 只支持 http 或 https 协议，当前协议为 `{0}`")]
    UnsupportedScheme(String),

    #[error("base url 不能包含 query 参数")]
    BaseUrlContainsQuery,

    #[error("base url 不能包含 fragment")]
    BaseUrlContainsFragment,

    #[error("Authorization 请求头生成失败: {0}")]
    InvalidAuthorizationHeader(#[source] reqwest::header::InvalidHeaderValue),

    #[error("HTTP Client 创建失败: {0}")]
    ClientBuild(#[source] reqwest::Error),
}

/// WebDAV 基础认证信息
///
/// 该类型不会保存明文密码，只会保存：
///
/// - 已配置 Authorization 的 reqwest Client
/// - 规范化后的 base_url
/// - Basic token 的 SHA-256 指纹
///
/// 注意：`token_fingerprint` 只是摘要，不是加密后的 token。
#[derive(Clone)]
pub struct WebdavAuth {
    client: Client,
    base_url: Arc<Url>,

    /// Basic token 的摘要，用于比较认证信息是否发生变化。
    ///
    /// 这里不保存原始 token，避免在结构体中长期持有可直接使用的认证凭据。
    token_fingerprint: Arc<String>,
}

impl WebdavAuth {
    /// 创建新的 WebDAV 认证对象
    ///
    /// `username` 和 `password` 只在构造阶段使用，构造完成后不会被保存。
    pub fn new(
        username: &str,
        password: &str,
        base_url: &str,
    ) -> Result<Self, WebdavAuthError> {
        let base_url = BaseUrl::parse(base_url)?;
        let basic_auth = BasicAuth::new(username, password)?;

        let client = HttpClientBuilder::new(&basic_auth)?;

        Ok(Self {
            client,
            base_url: Arc::new(base_url.into_url()),
            token_fingerprint: Arc::new(basic_auth.fingerprint().to_owned()),
        })
    }

    /// 获取 HTTP Client
    ///
    /// 该 Client 已经配置了默认 Authorization 请求头。
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// 获取 WebDAV 根地址
    pub fn base_url(&self) -> &Url {
        self.base_url.as_ref()
    }

    /// 仅比较认证 token 是否相同
    ///
    /// 不比较 base_url。
    pub fn eq_only_token(&self, other: &Self) -> bool {
        self.token_fingerprint == other.token_fingerprint
    }

    /// 获取 token 指纹。
    ///
    /// 一般业务代码不需要调用该方法，主要用于缓存键、调试或测试。
    pub(crate) fn token_fingerprint(&self) -> &str {
        self.token_fingerprint.as_str()
    }
}

/// 比较完整的 WebDAV 认证信息
///
/// Client 本身不参与比较，因为两个 Client 可能是不同实例，
/// 但实际的认证信息和访问地址相同。
impl PartialEq for WebdavAuth {
    fn eq(&self, other: &Self) -> bool {
        self.base_url == other.base_url
            && self.token_fingerprint == other.token_fingerprint
    }
}

impl Eq for WebdavAuth {}

/// 防止 Debug 时泄漏账号、密码和 Authorization
impl fmt::Debug for WebdavAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebdavAuth")
            .field("client", &"<Client with hidden authorization>")
            .field("base_url", &self.base_url)
            .field("token_fingerprint", &"<hidden>")
            .finish()
    }
}

/// 规范化后的 WebDAV Base URL
#[derive(Debug, Clone)]
struct BaseUrl(Url);

impl BaseUrl {
    fn parse(raw_url: &str) -> Result<Self, WebdavAuthError> {
        if raw_url.trim().is_empty() {
            return Err(WebdavAuthError::EmptyBaseUrl);
        }

        let mut url =
            Url::parse(raw_url).map_err(WebdavAuthError::InvalidBaseUrl)?;

        match url.scheme() {
            "http" | "https" => {}
            scheme => {
                return Err(WebdavAuthError::UnsupportedScheme(
                    scheme.to_owned(),
                ));
            }
        }

        if url.host_str().is_none() {
            return Err(WebdavAuthError::MissingHost);
        }

        // 如果把 query 或 fragment 放在 base_url 中，
        // 后续拼接 WebDAV 路径时容易出现不可预期行为。
        if url.query().is_some() {
            return Err(WebdavAuthError::BaseUrlContainsQuery);
        }

        if url.fragment().is_some() {
            return Err(WebdavAuthError::BaseUrlContainsFragment);
        }

        // 保证 base_url 的 path 以 `/` 结尾。
        //
        // 例如：
        //   https://example.com/webdav
        // 会被规范化为：
        //   https://example.com/webdav/
        if !url.path().ends_with('/') {
            let new_path = format!("{}/", url.path());
            url.set_path(&new_path);
        }

        Ok(Self(url))
    }

    fn into_url(self) -> Url {
        self.0
    }
}

/// Basic Authentication 数据
///
/// 该类型只在构造阶段短暂存在。
/// 不实现 Debug，避免误打印 Authorization。
struct BasicAuth {
    authorization: HeaderValue,
    fingerprint: String,
}

impl BasicAuth {
    fn new(username: &str, password: &str) -> Result<Self, WebdavAuthError> {
        let raw_token = format!("{username}:{password}");

        let encoded_token = base64::engine::general_purpose::STANDARD
            .encode(raw_token.as_bytes());

        let authorization =
            HeaderValue::from_str(&format!("Basic {encoded_token}"))
                .map_err(WebdavAuthError::InvalidAuthorizationHeader)?;

        let fingerprint = sha256_hex(&encoded_token);

        Ok(Self {
            authorization,
            fingerprint,
        })
    }

    fn authorization(&self) -> &HeaderValue {
        &self.authorization
    }

    fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

/// HTTP Client 构造器
struct HttpClientBuilder;

impl HttpClientBuilder {
    fn new(auth: &BasicAuth) -> Result<Client, WebdavAuthError> {
        let mut headers = HeaderMap::new();

        headers.insert(
            AUTHORIZATION,
            auth.authorization().clone(),
        );

        Client::builder()
            // 保留原有行为，某些 WebDAV 服务端对 HTTP/2 支持较差。
            .http1_only()
            .default_headers(headers)
            .build()
            .map_err(WebdavAuthError::ClientBuild)
    }
}

/// 计算字符串的 SHA-256 十六进制摘要
///
/// 这里是 fingerprint，不是加密。
fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());

    format!("{:x}", hasher.finalize())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_base_url_and_append_trailing_slash() {
        let base_url =
            BaseUrl::parse("https://example.com/webdav").unwrap();

        assert_eq!(
            base_url.0.as_str(),
            "https://example.com/webdav/"
        );
    }

    #[test]
    fn test_base_url_root_path() {
        let base_url =
            BaseUrl::parse("https://example.com").unwrap();

        assert_eq!(
            base_url.0.as_str(),
            "https://example.com/"
        );
    }

    #[test]
    fn test_base_url_keeps_existing_trailing_slash() {
        let base_url =
            BaseUrl::parse("https://example.com/webdav/").unwrap();

        assert_eq!(
            base_url.0.as_str(),
            "https://example.com/webdav/"
        );
    }

    #[test]
    fn test_reject_empty_base_url() {
        let result = BaseUrl::parse("");

        assert!(matches!(
            result,
            Err(WebdavAuthError::EmptyBaseUrl)
        ));
    }

    #[test]
    fn test_reject_invalid_base_url() {
        let result = BaseUrl::parse("not a url");

        assert!(matches!(
            result,
            Err(WebdavAuthError::InvalidBaseUrl(_))
        ));
    }

    #[test]
    fn test_reject_unsupported_scheme() {
        let result = BaseUrl::parse("ftp://example.com/webdav");

        assert!(matches!(
            result,
            Err(WebdavAuthError::UnsupportedScheme(_))
        ));
    }

    #[test]
    fn test_reject_base_url_with_query() {
        let result = BaseUrl::parse(
            "https://example.com/webdav?token=secret",
        );

        assert!(matches!(
            result,
            Err(WebdavAuthError::BaseUrlContainsQuery)
        ));
    }

    #[test]
    fn test_reject_base_url_with_fragment() {
        let result = BaseUrl::parse(
            "https://example.com/webdav#fragment",
        );

        assert!(matches!(
            result,
            Err(WebdavAuthError::BaseUrlContainsFragment)
        ));
    }

    #[test]
    fn test_basic_auth_header() {
        let auth = BasicAuth::new("alice", "password").unwrap();

        assert_eq!(
            auth.authorization().to_str().unwrap(),
            "Basic YWxpY2U6cGFzc3dvcmQ="
        );
    }

    #[test]
    fn test_same_credentials_have_same_fingerprint() {
        let auth1 = BasicAuth::new("alice", "password").unwrap();
        let auth2 = BasicAuth::new("alice", "password").unwrap();

        assert_eq!(
            auth1.fingerprint(),
            auth2.fingerprint()
        );
    }

    #[test]
    fn test_different_credentials_have_different_fingerprint() {
        let auth1 = BasicAuth::new("alice", "password1").unwrap();
        let auth2 = BasicAuth::new("alice", "password2").unwrap();

        assert_ne!(
            auth1.fingerprint(),
            auth2.fingerprint()
        );
    }

    #[test]
    fn test_webdav_auth_eq() {
        let auth1 = WebdavAuth::new(
            "alice",
            "password",
            "https://example.com/webdav",
        )
        .unwrap();

        let auth2 = WebdavAuth::new(
            "alice",
            "password",
            "https://example.com/webdav/",
        )
        .unwrap();

        assert_eq!(auth1, auth2);
    }

    #[test]
    fn test_webdav_auth_eq_includes_base_url() {
        let auth1 = WebdavAuth::new(
            "alice",
            "password",
            "https://example.com/webdav",
        )
        .unwrap();

        let auth2 = WebdavAuth::new(
            "alice",
            "password",
            "https://example.com/other",
        )
        .unwrap();

        assert_ne!(auth1, auth2);
        assert!(auth1.eq_only_token(&auth2));
    }

    #[test]
    fn test_debug_does_not_contain_password_or_authorization() {
        let auth = WebdavAuth::new(
            "alice",
            "super-secret-password",
            "https://example.com/webdav",
        )
        .unwrap();

        let debug_text = format!("{auth:?}");

        assert!(!debug_text.contains("alice"));
        assert!(!debug_text.contains("super-secret-password"));
        assert!(!debug_text.contains("Basic"));
        assert!(!debug_text.contains("YWxpY2U"));
    }
}

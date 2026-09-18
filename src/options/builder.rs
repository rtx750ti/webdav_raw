//! HTTP OPTIONS 请求构建、发送与能力头解析。
//!
//! OPTIONS 不返回资源内容，只回答「这台服务器允许做什么」。WebDAV 服务端的
//! 答案主要写在两个响应头里：
//!
//! - `DAV`：支持的 WebDAV 合规级别，通常是 `1, 2, 3` 这样的列表。
//!   按 RFC 4918 §10.1，`1` 与 `2` 是必给的，`3` 表示支持客户端自定义属性。
//! - `Allow`：允许的 HTTP 方法列表。
//!
//! 本库只**解析**这两个头，不替调用方判断「能不能做某件事」：要不要因为缺少
//! `PROPPATCH` 而跳过某一步，是调用方的决策。

use reqwest::{
    Client, Method, Request, Response,
    header::{ALLOW, HeaderMap, HeaderName, HeaderValue},
};
use thiserror::Error;
use url::Url;

/// `DAV` 响应头的名字。
const DAV: &str = "dav";

/// OPTIONS 请求构建和发送错误。
#[derive(Debug, Error)]
pub enum OptionsError {
    #[error("URL 格式错误: {0}")]
    Url(#[from] url::ParseError),

    #[error("请求头名称格式错误: {0}")]
    HeaderName(#[from] reqwest::header::InvalidHeaderName),

    #[error("请求头值格式错误: {0}")]
    HeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error("构建 HTTP 请求失败: {0}")]
    Request(#[from] reqwest::Error),
}

/// 从 OPTIONS 响应头解析出来的服务端能力。
///
/// 两个字段互相独立：任一响应头缺失时，对应的 `Vec` 为空，不影响另一个。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionsCapabilities {
    /// `DAV` 响应头的合规级别，例如 `["1", "2", "3"]`。
    pub dav_levels: Vec<String>,

    /// `Allow` 响应头允许的 HTTP 方法，例如 `["OPTIONS", "GET", "PUT"]`。
    pub allowed_methods: Vec<String>,
}

impl OptionsCapabilities {
    /// 从响应头解析能力信息。
    ///
    /// 解析规则：
    ///
    /// - 逗号分隔，每项 `trim` 后丢弃空项，保留原始大小写
    /// - 响应头缺失、取值非 UTF-8、取值为空 —— 都返回空 `Vec`，不报错
    /// - `DAV` 头可能被多次发送（`HeaderMap` 会保留多个同名头），所有取值都会被解析
    ///
    /// ```
    /// use webdav_raw::{HeaderMap, HeaderValue, OptionsCapabilities};
    ///
    /// let mut headers = HeaderMap::new();
    /// headers.insert("dav", HeaderValue::from_static("1, 2, 3"));
    /// headers.insert("allow", HeaderValue::from_static("OPTIONS, GET, PUT"));
    ///
    /// let caps = OptionsCapabilities::from_headers(&headers);
    /// assert_eq!(caps.dav_levels, ["1", "2", "3"]);
    /// assert_eq!(caps.allowed_methods, ["OPTIONS", "GET", "PUT"]);
    /// ```
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            dav_levels: parse_comma_separated(headers, DAV),
            allowed_methods: parse_comma_separated(headers, ALLOW.as_str()),
        }
    }
}

/// 把一个可能重复出现的响应头按逗号拆成列表。
///
/// 非 UTF-8 的取值会被跳过而不是报错：能力信息读不出来，等价于服务端没有声明
/// 该能力，调用方按「不支持」处理即可。
fn parse_comma_separated(headers: &HeaderMap, name: &str) -> Vec<String> {
    headers
        .get_all(name)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

/// 一次 OPTIONS 请求构建器。
///
/// 通常由 [`WebdavAuth::options`](crate::auth::WebdavAuth::options) 创建，复用认证
/// Client 与根地址；不设置目标地址时指向根地址本身。
///
/// 两个发送入口：
///
/// - [`send`](Self::send)：返回原始 [`Response`]，状态码与响应头都由调用方自己看。
/// - [`send_capabilities`](Self::send_capabilities)：额外把 `DAV` 与 `Allow`
///   解析成 [`OptionsCapabilities`]，同时仍然把原始响应交回。
///
/// ```
/// use webdav_raw::{Client, OptionsBuilder, Url};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let request = OptionsBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?)
///     .build()?;
///
/// assert_eq!(request.method().as_str(), "OPTIONS");
/// assert_eq!(request.url().as_str(), "https://example.com/dav/");
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct OptionsBuilder {
    client: Client,
    base_url: Url,
    target_path: Url,
    headers: HeaderMap,
}

impl OptionsBuilder {
    /// 使用认证根地址创建 OPTIONS Builder。
    ///
    /// 通常不需要直接调用，
    /// [`WebdavAuth::options`](crate::auth::WebdavAuth::options)
    /// 会用认证 Client 与根地址创建它。
    pub fn new(client: Client, base_url: Url) -> Self {
        Self {
            target_path: base_url.clone(),
            base_url,
            client,
            headers: HeaderMap::new(),
        }
    }

    /// 用相对路径设置目标地址，相对认证根地址解析。
    ///
    /// 路径里的空格和中文会被百分号编码。以 `/` 开头会从域名根开始解析，
    /// 覆盖掉根地址中已有的路径前缀；传入完整 URL 时直接使用该地址。
    pub fn target_path(mut self, path: &str) -> Result<Self, OptionsError> {
        self.target_path = self.base_url.join(path)?;

        Ok(self)
    }

    /// 用完整 URL 设置目标地址，不使用认证根地址。
    ///
    /// # 凭据只会跟着认证 Client 走
    ///
    /// 这里换掉的是地址，换不掉 Client：认证对象默认注入的 `Authorization`
    /// 请求头对本 Client 发出的**每一个**请求都生效。因此指向另一个主机时，
    /// 凭据会一并发过去。只有确认目标主机可信时才这样用；不确定就退回
    /// [`target_path`](Self::target_path)，它是拼接在认证根地址下的。
    pub fn target_url(mut self, url: &str) -> Result<Self, OptionsError> {
        self.target_path = Url::parse(url)?;

        Ok(self)
    }

    /// 追加一个请求头。
    ///
    /// 同名请求头是**替换**语义：后设的覆盖先设的，不追加第二个同名头。
    pub fn header(mut self, name: &str, value: &str) -> Result<Self, OptionsError> {
        self.headers
            .insert(HeaderName::try_from(name)?, HeaderValue::try_from(value)?);

        Ok(self)
    }

    /// 批量追加请求头，优先级同 [`header`](Self::header)。
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);

        self
    }

    /// 构建请求但不发送。
    ///
    /// 请求体为空，本库只负责写调用方设置的请求头。
    pub fn build(self) -> Result<Request, OptionsError> {
        let Self {
            client,
            base_url: _,
            target_path,
            headers,
        } = self;

        Ok(client
            .request(Method::OPTIONS, target_path)
            .headers(headers)
            .build()?)
    }

    /// 构建并发送，返回未经处理的原始响应。
    ///
    /// 状态码、响应头、响应体都原样交给调用方。想看 `DAV` 原始取值就用这个入口。
    pub async fn send(self) -> Result<Response, OptionsError> {
        let client = self.client.clone();
        let request = self.build()?;

        Ok(client.execute(request).await?)
    }

    /// 构建并发送，同时把 `DAV` 与 `Allow` 解析成结构化能力。
    ///
    /// 返回 `(响应, 能力)` 两项：原始响应仍然交回，调用方不必为了拿结构化结果而
    /// 放弃状态码和其余响应头。
    ///
    /// 本库只解析、不判定：要不要因为缺少某个方法而跳过后续步骤，由调用方
    /// 读 [`OptionsCapabilities::allowed_methods`] 自己决定。
    pub async fn send_capabilities(
        self,
    ) -> Result<(Response, OptionsCapabilities), OptionsError> {
        let response = self.send().await?;
        let capabilities = OptionsCapabilities::from_headers(response.headers());

        Ok((response, capabilities))
    }
}

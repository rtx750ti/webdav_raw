//! HTTP HEAD 请求构建与发送。
//!
//! HEAD 与 GET 的路径语义完全一致，区别只有一个：**响应没有响应体**。
//! `Content-Length`、`Content-Type`、`Last-Modified`、`ETag` 这些元数据仍然
//! 在响应头里，因此 HEAD 是「先问元数据、不下载内容」的手段。
//!
//! 一个 Builder 构建一次 HEAD 请求，不保存任何请求之间的状态。

use reqwest::{
    Client, Method, Request, Response,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use thiserror::Error;
use url::Url;

/// HEAD 请求构建和发送错误。
#[derive(Debug, Error)]
pub enum HeadError {
    #[error("URL 格式错误: {0}")]
    Url(#[from] url::ParseError),

    #[error("请求头名称格式错误: {0}")]
    HeaderName(#[from] reqwest::header::InvalidHeaderName),

    #[error("请求头值格式错误: {0}")]
    HeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error("构建 HTTP 请求失败: {0}")]
    Request(#[from] reqwest::Error),
}

/// 一次 HEAD 请求构建器。
///
/// 通常由 [`WebdavAuth::head`](crate::auth::WebdavAuth::head) 创建，复用认证
/// Client 与根地址；不设置目标地址时指向根地址本身。
///
/// ```
/// use webdav_raw::{Client, HeadBuilder, Url};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let request = HeadBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?)
///     .target_path("Documents/report.pdf")?
///     .build()?;
///
/// assert_eq!(request.method().as_str(), "HEAD");
/// assert_eq!(
///     request.url().as_str(),
///     "https://example.com/dav/Documents/report.pdf"
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct HeadBuilder {
    client: Client,
    base_url: Url,
    target_path: Url,
    headers: HeaderMap,
}

impl HeadBuilder {
    /// 使用认证根地址创建 HEAD Builder。
    ///
    /// 通常不需要直接调用，[`WebdavAuth::head`](crate::auth::WebdavAuth::head)
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
    ///
    /// ```
    /// use webdav_raw::{Client, HeadBuilder, Url};
    ///
    /// let builder = HeadBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?);
    /// let builder = builder.target_path("目录/报告 1.txt")?;
    /// # let _ = builder;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn target_path(mut self, path: &str) -> Result<Self, HeadError> {
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
    ///
    /// # 只检查语法，不检查协议
    ///
    /// `ftp://` 这类地址语法合法，能通过 [`build`](Self::build)，但发不出去：
    /// 协议检查发生在发送阶段，[`send`](Self::send) 会以 [`HeadError::Request`]
    /// 报错。本库不额外拦一道，避免和 reqwest 出现两套规则。
    pub fn target_url(mut self, url: &str) -> Result<Self, HeadError> {
        self.target_path = Url::parse(url)?;

        Ok(self)
    }

    /// 追加一个请求头。
    ///
    /// 同名请求头是**替换**语义：后设的覆盖先设的，不追加第二个同名头。
    ///
    /// ```
    /// use webdav_raw::{Client, HeadBuilder, Url};
    ///
    /// let builder = HeadBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?);
    /// let builder = builder.header("if-none-match", "\"v1\"")?;
    /// # let _ = builder;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn header(mut self, name: &str, value: &str) -> Result<Self, HeadError> {
        self.headers
            .insert(HeaderName::try_from(name)?, HeaderValue::try_from(value)?);

        Ok(self)
    }

    /// 批量追加请求头，优先级同 [`header`](Self::header)。
    ///
    /// 这个方法不返回 `Result`，是因为 `HeaderMap` 里的名称和取值在构造时就已经
    /// 校验过，装进来的不可能非法。同名请求头同样是替换语义。
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);

        self
    }

    /// 构建请求但不发送。
    ///
    /// 请求体为空，本库只负责写调用方设置的请求头。
    pub fn build(self) -> Result<Request, HeadError> {
        let Self {
            client,
            base_url: _,
            target_path,
            headers,
        } = self;

        Ok(client
            .request(Method::HEAD, target_path)
            .headers(headers)
            .build()?)
    }

    /// 构建并发送，返回未经处理的原始响应。
    ///
    /// 响应没有响应体，但响应头全部保留：`Content-Length`、`Content-Type`、
    /// `Last-Modified`、`ETag` 都能照常读取。状态码也不做任何判断——
    /// 200 与 404 都原样交给调用方。
    pub async fn send(self) -> Result<Response, HeadError> {
        let client = self.client.clone();
        let request = self.build()?;

        Ok(client.execute(request).await?)
    }
}

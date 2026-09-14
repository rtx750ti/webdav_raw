//! WebDAV DELETE 请求构建与发送。
//!
//! 一个 Builder 构建一次 DELETE 请求，不保存任何请求之间的状态。

use reqwest::{
    Client, Method, Request, Response,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use thiserror::Error;
use url::Url;

/// DELETE 请求构建和发送错误。
///
/// 本地错误各有自己的变体，网络相关的失败集中在 [`DeleteError::Request`] 里：
///
/// - **地址写错**：[`DeleteError::Url`]，不发出任何网络请求。
/// - **请求头写错**：[`DeleteError::HeaderName`] / [`DeleteError::HeaderValue`]。
/// - **请求构建失败**：[`DeleteError::Request`]，`reqwest::Error::is_builder`。
/// - **请求发送失败**：[`DeleteError::Request`]，可用 `is_connect`、`is_timeout` 等细分。
///
/// 本库不把服务端状态码当成错误：204、404、423、207 都原样留在
/// [`Response`](reqwest::Response) 里。
#[derive(Debug, Error)]
pub enum DeleteError {
    #[error("URL 格式错误: {0}")]
    Url(#[from] url::ParseError),

    #[error("请求头名称格式错误: {0}")]
    HeaderName(#[from] reqwest::header::InvalidHeaderName),

    #[error("请求头值格式错误: {0}")]
    HeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error("构建 HTTP 请求失败: {0}")]
    Request(#[from] reqwest::Error),
}

/// DELETE 的 `Depth` 请求头取值。
///
/// RFC 4918 §9.6.1 只允许 `0` 与 `infinity` 两个值，因此这里不提供
/// `Depth::One`——协议非法的组合在编译期就不存在。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeleteDepth {
    /// 只删除当前资源；目标是集合且非空时服务端返回 207 或错误。
    Zero,

    /// 递归删除集合及其全部成员。
    #[default]
    Infinity,
}

impl DeleteDepth {
    /// 转换成 HTTP 请求头中的字符串。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::Infinity => "infinity",
        }
    }
}

/// 一次 DELETE 请求构建器。
///
/// 通常由 [`WebdavAuth::delete`](crate::auth::WebdavAuth::delete) 创建，复用认证
/// Client 与根地址；不设置目标地址时指向根地址本身。
///
/// ```
/// use webdav_core::{Client, DeleteBuilder, DeleteDepth, Url};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let builder = DeleteBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?)
///     .target_path("staging/")?
///     .depth(DeleteDepth::Infinity);
/// # let _ = builder;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct DeleteBuilder {
    client: Client,
    base_url: Url,
    target_path: Url,
    depth: DeleteDepth,
    headers: HeaderMap,
}

impl DeleteBuilder {
    /// 使用认证根地址创建 DELETE Builder。
    ///
    /// 通常不需要直接调用，[`WebdavAuth::delete`](crate::auth::WebdavAuth::delete)
    /// 会用认证 Client 与根地址创建它。
    pub fn new(client: Client, base_url: Url) -> Self {
        Self {
            target_path: base_url.clone(),
            base_url,
            client,
            depth: DeleteDepth::default(),
            headers: HeaderMap::new(),
        }
    }

    /// 用相对路径设置目标地址，相对认证根地址解析。
    ///
    /// 路径里的空格和中文会被百分号编码。以 `/` 开头会从域名根开始解析，
    /// 覆盖掉根地址中已有的路径前缀；传入完整 URL 时直接使用该地址。
    ///
    /// ```
    /// use webdav_core::{Client, DeleteBuilder, Url};
    ///
    /// let builder = DeleteBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?);
    /// let builder = builder.target_path("目录/报告 1.txt")?;
    /// # let _ = builder;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn target_path(mut self, path: &str) -> Result<Self, DeleteError> {
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
    /// 协议检查发生在发送阶段，[`send`](Self::send) 会以 [`DeleteError::Request`]
    /// 报错。本库不额外拦一道，避免和 reqwest 出现两套规则。
    pub fn target_url(mut self, url: &str) -> Result<Self, DeleteError> {
        self.target_path = Url::parse(url)?;

        Ok(self)
    }

    /// 设置 `Depth` 请求头，默认 [`DeleteDepth::Infinity`]。
    pub fn depth(mut self, depth: DeleteDepth) -> Self {
        self.depth = depth;

        self
    }

    /// 追加一个请求头。
    ///
    /// 同名请求头是**替换**语义：后设的覆盖先设的，不追加第二个同名头。
    ///
    /// ```
    /// use webdav_core::{Client, DeleteBuilder, Url};
    ///
    /// let builder = DeleteBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?);
    /// let builder = builder.header("if-match", "\"v1\"")?;
    /// # let _ = builder;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn header(mut self, name: &str, value: &str) -> Result<Self, DeleteError> {
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
    /// 请求体为空，本库只负责写 `Depth` 与调用方设置的请求头。
    ///
    /// ```
    /// use webdav_core::{Client, DeleteBuilder, DeleteDepth, Url};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let request = DeleteBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?)
    ///     .target_path("staging/old.txt")?
    ///     .depth(DeleteDepth::Zero)
    ///     .build()?;
    ///
    /// assert_eq!(request.method().as_str(), "DELETE");
    /// assert_eq!(request.url().as_str(), "https://example.com/dav/staging/old.txt");
    /// assert_eq!(request.headers().get("depth").unwrap(), "0");
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<Request, DeleteError> {
        let Self {
            client,
            base_url: _,
            target_path,
            depth,
            mut headers,
        } = self;

        headers.insert("depth", HeaderValue::from_static(depth.as_str()));

        Ok(client
            .request(Method::DELETE, target_path)
            .headers(headers)
            .build()?)
    }

    /// 构建并发送，返回未经处理的原始响应。
    ///
    /// 状态码、响应头和响应体都原样交给调用方，本库不做任何判断——与 GET 的
    /// 行为保持一致。204、404、423、207 都不会被当成错误。
    pub async fn send(self) -> Result<Response, DeleteError> {
        let client = self.client.clone();
        let request = self.build()?;

        Ok(client.execute(request).await?)
    }
}

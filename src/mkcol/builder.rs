//! WebDAV MKCOL 请求构建与发送。
//!
//! MKCOL 创建集合（目录），**不递归**：父集合不存在时服务端返回 409 Conflict，
//! 需要多层就由调用方按层级依次调用。
//!
//! 按 RFC 4918 §9.3.1，MKCOL 可以带请求体，且请求体必须是 XML。标准没有定义
//! 任何有用的 body 内容，因此本库默认发零长度请求体。调用方显式设置 body 时，
//! 本库按 XML 写 `Content-Type`，但**不校验** body 是不是合法 XML——理由与 PUT
//! 不拦 `ftp://` 相同：不和服务端造两套规则，服务端自己会回 415。

use reqwest::{
    Body, Client, Method, Request, Response,
    header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use thiserror::Error;
use url::Url;

/// 调用方提供请求体时使用的内容类型。
const XML_CONTENT_TYPE: &str = "application/xml; charset=utf-8";

/// MKCOL 请求构建和发送错误。
#[derive(Debug, Error)]
pub enum MkcolError {
    #[error("URL 格式错误: {0}")]
    Url(#[from] url::ParseError),

    #[error("请求头名称格式错误: {0}")]
    HeaderName(#[from] reqwest::header::InvalidHeaderName),

    #[error("请求头值格式错误: {0}")]
    HeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error("构建 HTTP 请求失败: {0}")]
    Request(#[from] reqwest::Error),
}

/// 一次 MKCOL 请求构建器。
///
/// 通常由 [`WebdavAuth::mkcol`](crate::auth::WebdavAuth::mkcol) 创建，复用认证
/// Client 与根地址；不设置目标地址时指向根地址本身。
///
/// 常见状态码（本库只透传，不判定）：
///
/// - `201 Created`：创建成功。
/// - `405 Method Not Allowed`：资源已经存在。
/// - `409 Conflict`：父集合不存在。
/// - `415 Unsupported Media Type`：服务端不接受请求体。
///
/// ```
/// use webdav_core::{Client, MkcolBuilder, Url};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let request = MkcolBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?)
///     .target_path("release/")?
///     .build()?;
///
/// assert_eq!(request.method().as_str(), "MKCOL");
/// assert_eq!(request.url().as_str(), "https://example.com/dav/release/");
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct MkcolBuilder {
    client: Client,
    base_url: Url,
    target_path: Url,
    body: Option<String>,
    headers: HeaderMap,
}

impl MkcolBuilder {
    /// 使用认证根地址创建 MKCOL Builder。
    ///
    /// 通常不需要直接调用，[`WebdavAuth::mkcol`](crate::auth::WebdavAuth::mkcol)
    /// 会用认证 Client 与根地址创建它。
    pub fn new(client: Client, base_url: Url) -> Self {
        Self {
            target_path: base_url.clone(),
            base_url,
            client,
            body: None,
            headers: HeaderMap::new(),
        }
    }

    /// 用相对路径设置目标地址，相对认证根地址解析。
    ///
    /// 路径里的空格和中文会被百分号编码。以 `/` 开头会从域名根开始解析，
    /// 覆盖掉根地址中已有的路径前缀；传入完整 URL 时直接使用该地址。
    ///
    /// ```
    /// use webdav_core::{Client, MkcolBuilder, Url};
    ///
    /// let builder = MkcolBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?);
    /// let builder = builder.target_path("新建 目录/")?;
    /// # let _ = builder;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn target_path(mut self, path: &str) -> Result<Self, MkcolError> {
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
    /// 协议检查发生在发送阶段，[`send`](Self::send) 会以 [`MkcolError::Request`]
    /// 报错。本库不额外拦一道，避免和 reqwest 出现两套规则。
    pub fn target_url(mut self, url: &str) -> Result<Self, MkcolError> {
        self.target_path = Url::parse(url)?;

        Ok(self)
    }

    /// 设置 MKCOL 请求体。
    ///
    /// 不设置时发送零长度请求体，且**不写** `Content-Type`；设置后本库会补上
    /// `application/xml; charset=utf-8`。空字符串等价于不设置。
    ///
    /// 本库不校验 body 是否为合法 XML：服务端不接受时会回 415，调用方按状态码
    /// 处理即可。
    pub fn body(mut self, body: impl Into<String>) -> Self {
        let body = body.into();
        self.body = if body.is_empty() { None } else { Some(body) };

        self
    }

    /// 追加一个请求头。
    ///
    /// 同名请求头是**替换**语义：后设的覆盖先设的，不追加第二个同名头。
    /// 调用方设置的 `Content-Type` 会覆盖本库为请求体补的默认值。
    ///
    /// ```
    /// use webdav_core::{Client, MkcolBuilder, Url};
    ///
    /// let builder = MkcolBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?);
    /// let builder = builder.header("x-note", "建目录")?;
    /// # let _ = builder;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn header(mut self, name: &str, value: &str) -> Result<Self, MkcolError> {
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
    /// 只有设置了请求体时才补 `Content-Type`，且调用方显式设置的值优先。
    pub fn build(self) -> Result<Request, MkcolError> {
        let Self {
            client,
            base_url: _,
            target_path,
            body,
            mut headers,
        } = self;

        let body = body.unwrap_or_default();

        if !body.is_empty() && !headers.contains_key(CONTENT_TYPE) {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static(XML_CONTENT_TYPE));
        }

        // MKCOL 不是标准 HTTP 方法，由 reqwest 透传。
        let method = Method::from_bytes(b"MKCOL").expect("MKCOL is a valid HTTP method");

        Ok(client
            .request(method, target_path)
            .headers(headers)
            .body(Body::from(body))
            .build()?)
    }

    /// 构建并发送，返回未经处理的原始响应。
    ///
    /// 状态码、响应头和响应体都原样交给调用方，本库不做任何判断：
    /// 201/405/409/415 都不会被当成错误。
    pub async fn send(self) -> Result<Response, MkcolError> {
        let client = self.client.clone();
        let request = self.build()?;

        Ok(client.execute(request).await?)
    }
}

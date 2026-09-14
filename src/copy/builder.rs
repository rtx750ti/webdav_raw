//! WebDAV COPY 请求构建与发送。
//!
//! COPY 有两个地址：**源**（请求行里的 URI，由 `source_path` / `source_url` 设置）
//! 和**目标**（`Destination` 请求头，由 `target_path` / `target_url` 设置）。
//!
//! # `Destination` 头由本库从 URL 生成
//!
//! 调用方给的是字符串，但写进请求头的是**已解析 URL 的重新序列化结果**：
//!
//! ```text
//! 调用方写：  .target_path("备份/报告 2026.pdf")
//! 库里出去：  Destination: https://dav.example.com/dav/%E5%A4%87%E4%BB%BD/%E6%8A%A5%E5%91%8A%202026.pdf
//! ```
//!
//! 这样做有两个好处：
//!
//! 1. 空格和中文由 `Url` 负责百分号编码，调用方不必自己编码。
//! 2. 取值永远带 scheme 与 authority，满足 RFC 4918 §9.9.3 对 `Destination`
//!    必须是完整 URI 的要求。
//!
//! # 缺目标时不发请求
//!
//! 忘记设置目标是调用方错误。`build()` 会返回 [`CopyError::MissingTarget`]，
//! 而不是发出一个没有 `Destination` 的请求让服务端困惑。

use reqwest::{
    Client, Method, Request, Response, StatusCode,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use thiserror::Error;
use url::Url;

use crate::copy::overwrite::Overwrite;
use crate::propfind::raw_xml::multistatus::MultiStatus;

/// COPY / MOVE 的 `Depth` 请求头取值。
///
/// 这两种方法只允许 `0`、`1` 与 `infinity`（RFC 4918 §9.8.3、§9.9.3），
/// 因此枚举里没有其他取值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CopyDepth {
    /// 只复制集合本身，不含成员。
    Zero,

    /// 复制集合本身与直接成员。
    One,

    /// 递归复制全部成员。
    #[default]
    Infinity,
}

impl CopyDepth {
    /// 转换成 HTTP 请求头中的字符串。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::One => "1",
            Self::Infinity => "infinity",
        }
    }
}

/// COPY 请求构建和发送错误。
#[derive(Debug, Error)]
pub enum CopyError {
    #[error("URL 格式错误: {0}")]
    Url(#[from] url::ParseError),

    #[error("请求头名称格式错误: {0}")]
    HeaderName(#[from] reqwest::header::InvalidHeaderName),

    #[error("请求头值格式错误: {0}")]
    HeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error("构建 HTTP 请求失败: {0}")]
    Request(#[from] reqwest::Error),

    #[error("未设置目标地址")]
    MissingTarget,

    #[error("期望 207 Multi-Status，实际收到 {0}")]
    UnexpectedStatus(StatusCode),

    #[error("反序列化失败: {0}")]
    De(#[from] quick_xml::DeError),
}

/// 一次 COPY 请求构建器。
///
/// 通常由 [`WebdavAuth::copy`](crate::auth::WebdavAuth::copy) 创建，复用认证
/// Client 与根地址；不设置源地址时源是根地址本身，但**目标必须显式设置**。
///
/// ```
/// use webdav_core::{Client, CopyDepth, CopyBuilder, Overwrite, Url};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let request = CopyBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?)
///     .source_path("staging/report.pdf")?
///     .target_path("release/report.pdf")?
///     .overwrite(Overwrite::False)
///     .depth(CopyDepth::Infinity)
///     .build()?;
///
/// assert_eq!(request.method().as_str(), "COPY");
/// assert_eq!(request.url().as_str(), "https://example.com/dav/staging/report.pdf");
/// assert_eq!(
///     request.headers().get("destination").unwrap(),
///     "https://example.com/dav/release/report.pdf"
/// );
/// assert_eq!(request.headers().get("overwrite").unwrap(), "F");
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct CopyBuilder {
    client: Client,
    base_url: Url,
    source_path: Url,
    target: Option<Url>,
    depth: CopyDepth,
    overwrite: Overwrite,
    headers: HeaderMap,
}

impl CopyBuilder {
    /// 使用认证根地址创建 COPY Builder。
    ///
    /// 通常不需要直接调用，[`WebdavAuth::copy`](crate::auth::WebdavAuth::copy)
    /// 会用认证 Client 与根地址创建它。
    pub fn new(client: Client, base_url: Url) -> Self {
        Self {
            source_path: base_url.clone(),
            base_url,
            client,
            target: None,
            depth: CopyDepth::default(),
            overwrite: Overwrite::default(),
            headers: HeaderMap::new(),
        }
    }

    /// 用相对路径设置**源**地址，相对认证根地址解析。
    ///
    /// 路径里的空格和中文会被百分号编码。以 `/` 开头会从域名根开始解析，
    /// 覆盖掉根地址中已有的路径前缀；传入完整 URL 时直接使用该地址。
    pub fn source_path(mut self, path: &str) -> Result<Self, CopyError> {
        self.source_path = self.base_url.join(path)?;

        Ok(self)
    }

    /// 用完整 URL 设置**源**地址，不使用认证根地址。
    pub fn source_url(mut self, url: &str) -> Result<Self, CopyError> {
        self.source_path = Url::parse(url)?;

        Ok(self)
    }

    /// 用相对路径设置**目标**地址，相对认证根地址解析。
    ///
    /// 这个值会经过 URL 编码后写进 `Destination` 请求头，见本模块文档。
    ///
    /// ```
    /// use webdav_core::{Client, CopyBuilder, Url};
    ///
    /// let builder = CopyBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?);
    /// let builder = builder.target_path("备份/报告 2026.pdf")?;
    /// # let _ = builder;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn target_path(mut self, path: &str) -> Result<Self, CopyError> {
        self.target = Some(self.base_url.join(path)?);

        Ok(self)
    }

    /// 用完整 URL 设置**目标**地址，不使用认证根地址。
    ///
    /// 目标不要求与源同主机：本库只负责把完整 URL 写进 `Destination`，
    /// 服务端是否接受跨主机目标由服务端决定（多数服务端会拒绝）。
    pub fn target_url(mut self, url: &str) -> Result<Self, CopyError> {
        self.target = Some(Url::parse(url)?);

        Ok(self)
    }

    /// 设置 `Depth` 请求头，默认 [`CopyDepth::Infinity`]。
    pub fn depth(mut self, depth: CopyDepth) -> Self {
        self.depth = depth;

        self
    }

    /// 设置覆盖行为，默认 [`Overwrite::True`]（即不发送 `Overwrite` 头）。
    pub fn overwrite(mut self, overwrite: Overwrite) -> Self {
        self.overwrite = overwrite;

        self
    }

    /// 追加一个请求头。
    ///
    /// 同名请求头是**替换**语义：后设的覆盖先设的，不追加第二个同名头。
    /// 调用方设置的 `Destination` 会被本库从目标 URL 生成的值覆盖——
    /// 目标地址只有一个可信来源，不提供绕过入口。
    pub fn header(mut self, name: &str, value: &str) -> Result<Self, CopyError> {
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
    /// # Errors
    ///
    /// 未设置目标地址时返回 [`CopyError::MissingTarget`]，不发出任何请求。
    pub fn build(self) -> Result<Request, CopyError> {
        let Self {
            client,
            base_url: _,
            source_path,
            target,
            depth,
            overwrite,
            mut headers,
        } = self;

        let target = target.ok_or(CopyError::MissingTarget)?;

        // `Destination` 永远取已解析 URL 的重新序列化结果，不回填调用方原字符串。
        headers.insert(
            "destination",
            HeaderValue::from_str(target.as_str()).map_err(CopyError::HeaderValue)?,
        );
        headers.insert("depth", HeaderValue::from_static(depth.as_str()));

        if overwrite == Overwrite::False {
            headers.insert("overwrite", HeaderValue::from_static(overwrite.as_str()));
        }

        // COPY 不是标准 HTTP 方法，由 reqwest 透传。
        let method = Method::from_bytes(b"COPY").expect("COPY is a valid HTTP method");

        Ok(client
            .request(method, source_path)
            .headers(headers)
            .build()?)
    }

    /// 构建并发送，返回未经处理的原始响应。
    ///
    /// 状态码、响应头和响应体都原样交给调用方：201、204、207、409、412、423
    /// 都不会被当成错误。
    pub async fn send(self) -> Result<Response, CopyError> {
        let client = self.client.clone();
        let request = self.build()?;

        Ok(client.execute(request).await?)
    }

    /// 构建并发送，要求响应必须是 207 Multi-Status，并解析成 [`MultiStatus`]。
    ///
    /// 只有服务端对集合做了部分复制时才会回 207；其他状态码返回
    /// [`CopyError::UnexpectedStatus`]。想自己处理任意状态码就用
    /// [`send`](Self::send)。
    ///
    /// # 解析前提
    ///
    /// 反序列化复用 `propfind` 领域已公开的 [`MultiStatus`]，因此继承它的一条
    /// 输入约束：**每个 `<propstat>` 都必须含有 `<prop>` 子元素**。多数服务端在
    /// 部分失败时会写成 `<propstat><prop/><status>…</status></propstat>`，这种
    /// 形态可以正常解析；若服务端连 `<prop>` 都省略，整个响应会解析失败并返回
    /// [`CopyError::De`]。
    ///
    /// 本库不为此在本领域另写一套解析器——那会和 `propfind` 形成两套 `MultiStatus`
    /// 语义。需要在这种响应上取到 `href` 与状态码时，用 [`send`](Self::send) 拿原始
    /// 响应自行处理。
    pub async fn send_and_deserialize(self) -> Result<MultiStatus, CopyError> {
        let response = self.send().await?;
        let status = response.status();

        if status != StatusCode::MULTI_STATUS {
            return Err(CopyError::UnexpectedStatus(status));
        }

        let body = response.text().await?;

        Ok(MultiStatus::from_str(&body)?)
    }
}

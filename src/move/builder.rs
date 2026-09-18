//! WebDAV MOVE 请求构建与发送。
//!
//! MOVE 的语义是「COPY + 删源」：请求成功后**源资源不再存在**，目标位置出现
//! 同一份资源。重命名（同一集合内改名字）也是 MOVE 的一种用法。
//!
//! # 入口名为什么是 `mv`
//!
//! [`WebdavAuth::mv`](crate::auth::WebdavAuth::mv) 用的是 `mv` 而不是 `r#move`：
//! `move` 是 Rust 关键字，作为方法名只能写成 `r#move()`，调用点很难看。模块名
//! `r#move` 只是实现细节，不出现在调用方代码里。
//!
//! # `Destination` 头由本库从 URL 生成
//!
//! 与 COPY 完全一致：调用方给字符串，写进请求头的是**已解析 URL 的重新序列化
//! 结果**，因此空格和中文自动百分号编码，取值永远带 scheme 与 authority。
//!
//! # 怎么确认移动真的成功
//!
//! 本库只透传状态码，不替调用方判定。要确认结果，用 [`WebdavAuth::head`] 或
//! [`WebdavAuth::propfind`] 回查：源应该查不到，目标应该查得到。
//!
//! [`WebdavAuth::head`]: crate::auth::WebdavAuth::head
//! [`WebdavAuth::propfind`]: crate::auth::WebdavAuth::propfind

use reqwest::{
    Client, Method, Request, Response, StatusCode,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use thiserror::Error;
use url::Url;

use crate::copy::overwrite::Overwrite;
use crate::propfind::raw_xml::multistatus::MultiStatus;

/// MOVE 的 `Depth` 请求头取值。
///
/// 与 COPY 一样允许 `0`、`1`、`infinity`（RFC 4918 §9.9.3）。单独定义而不是
/// 复用 COPY 的类型：两种方法将来若在取值上分道扬镳，改这里不影响 COPY。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MoveDepth {
    /// 只移动集合本身，不含成员。
    Zero,

    /// 移动集合本身与直接成员。
    One,

    /// 递归移动全部成员。
    #[default]
    Infinity,
}

impl MoveDepth {
    /// 转换成 HTTP 请求头中的字符串。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::One => "1",
            Self::Infinity => "infinity",
        }
    }
}

/// MOVE 请求构建和发送错误。
#[derive(Debug, Error)]
pub enum MoveError {
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

/// 一次 MOVE 请求构建器。
///
/// 通常由 [`WebdavAuth::mv`](crate::auth::WebdavAuth::mv) 创建，复用认证
/// Client 与根地址；不设置源地址时源是根地址本身，但**目标必须显式设置**。
///
/// ```
/// use webdav_raw::{Client, MoveBuilder, MoveDepth, Overwrite, Url};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let request = MoveBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?)
///     .move_from_path("staging/report.pdf")?
///     .target_path("archive/report.pdf")?
///     .overwrite(Overwrite::False)
///     .depth(MoveDepth::Infinity)
///     .build()?;
///
/// assert_eq!(request.method().as_str(), "MOVE");
/// assert_eq!(request.url().as_str(), "https://example.com/dav/staging/report.pdf");
/// assert_eq!(
///     request.headers().get("destination").unwrap(),
///     "https://example.com/dav/archive/report.pdf"
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct MoveBuilder {
    client: Client,
    base_url: Url,
    source_path: Url,
    target: Option<Url>,
    depth: MoveDepth,
    overwrite: Overwrite,
    headers: HeaderMap,
}

impl MoveBuilder {
    /// 使用认证根地址创建 MOVE Builder。
    ///
    /// 通常不需要直接调用，[`WebdavAuth::mv`](crate::auth::WebdavAuth::mv)
    /// 会用认证 Client 与根地址创建它。
    pub fn new(client: Client, base_url: Url) -> Self {
        Self {
            source_path: base_url.clone(),
            base_url,
            client,
            target: None,
            depth: MoveDepth::default(),
            overwrite: Overwrite::default(),
            headers: HeaderMap::new(),
        }
    }

    /// 用相对路径设置**源**地址，相对认证根地址解析。
    ///
    /// 路径里的空格和中文会被百分号编码。以 `/` 开头会从域名根开始解析，
    /// 覆盖掉根地址中已有的路径前缀；传入完整 URL 时直接使用该地址。
    ///
    /// 与 [`move_from_path`](Self::move_from_path) 完全等价，两个名字都能用：
    /// 前者与 COPY 的 `source_path` 逐字一致，后者读起来更像「移动」这件事。
    pub fn source_path(mut self, path: &str) -> Result<Self, MoveError> {
        self.source_path = self.base_url.join(path)?;

        Ok(self)
    }

    /// 用相对路径设置**源**地址，语义等同 [`source_path`](Self::source_path)。
    ///
    /// 名字里带 `move_` 是为了在调用点提醒读者：**这个资源会在请求成功后消失**。
    ///
    /// ```
    /// use webdav_raw::{Client, MoveBuilder, Url};
    ///
    /// let builder = MoveBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?);
    /// let builder = builder.move_from_path("old/name.txt")?;
    /// # let _ = builder;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn move_from_path(self, path: &str) -> Result<Self, MoveError> {
        self.source_path(path)
    }

    /// 用完整 URL 设置**源**地址，不使用认证根地址。
    pub fn source_url(mut self, url: &str) -> Result<Self, MoveError> {
        self.source_path = Url::parse(url)?;

        Ok(self)
    }

    /// 用相对路径设置**目标**地址，相对认证根地址解析。
    ///
    /// 这个值会经过 URL 编码后写进 `Destination` 请求头。
    pub fn target_path(mut self, path: &str) -> Result<Self, MoveError> {
        self.target = Some(self.base_url.join(path)?);

        Ok(self)
    }

    /// 用完整 URL 设置**目标**地址，不使用认证根地址。
    ///
    /// 目标不要求与源同主机：本库只负责把完整 URL 写进 `Destination`，
    /// 服务端是否接受跨主机目标由服务端决定（多数服务端会拒绝）。
    pub fn target_url(mut self, url: &str) -> Result<Self, MoveError> {
        self.target = Some(Url::parse(url)?);

        Ok(self)
    }

    /// 设置 `Depth` 请求头，默认 [`MoveDepth::Infinity`]。
    pub fn depth(mut self, depth: MoveDepth) -> Self {
        self.depth = depth;

        self
    }

    /// 设置覆盖行为，默认 [`Overwrite::True`]（即不发送 `Overwrite` 头）。
    ///
    /// 这里复用 COPY 领域的 [`Overwrite`]：MOVE 是「COPY + 删源」，两者对
    /// 「目标已存在时怎么办」的回答完全一致，没有必要定义两个同形类型。
    pub fn overwrite(mut self, overwrite: Overwrite) -> Self {
        self.overwrite = overwrite;

        self
    }

    /// 追加一个请求头。
    ///
    /// 同名请求头是**替换**语义：后设的覆盖先设的，不追加第二个同名头。
    /// 调用方设置的 `Destination` 会被本库从目标 URL 生成的值覆盖。
    pub fn header(mut self, name: &str, value: &str) -> Result<Self, MoveError> {
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
    /// 未设置目标地址时返回 [`MoveError::MissingTarget`]，不发出任何请求。
    pub fn build(self) -> Result<Request, MoveError> {
        let Self {
            client,
            base_url: _,
            source_path,
            target,
            depth,
            overwrite,
            mut headers,
        } = self;

        let target = target.ok_or(MoveError::MissingTarget)?;

        // `Destination` 永远取已解析 URL 的重新序列化结果，不回填调用方原字符串。
        headers.insert(
            "destination",
            HeaderValue::from_str(target.as_str()).map_err(MoveError::HeaderValue)?,
        );
        headers.insert("depth", HeaderValue::from_static(depth.as_str()));

        if overwrite == Overwrite::False {
            headers.insert("overwrite", HeaderValue::from_static(overwrite.as_str()));
        }

        // MOVE 不是标准 HTTP 方法，由 reqwest 透传。
        let method = Method::from_bytes(b"MOVE").expect("MOVE is a valid HTTP method");

        Ok(client
            .request(method, source_path)
            .headers(headers)
            .build()?)
    }

    /// 构建并发送，返回未经处理的原始响应。
    ///
    /// 状态码、响应头和响应体都原样交给调用方：201、204、207、403、409、412、
    /// 423 都不会被当成错误。
    pub async fn send(self) -> Result<Response, MoveError> {
        let client = self.client.clone();
        let request = self.build()?;

        Ok(client.execute(request).await?)
    }

    /// 构建并发送，要求响应必须是 207 Multi-Status，并解析成 [`MultiStatus`]。
    ///
    /// 只有服务端对集合做了部分移动时才会回 207；其他状态码返回
    /// [`MoveError::UnexpectedStatus`]。想自己处理任意状态码就用
    /// [`send`](Self::send)。
    ///
    /// # 解析前提
    ///
    /// 反序列化复用 `propfind` 领域已公开的 [`MultiStatus`]，因此继承它的一条
    /// 输入约束：**每个 `<propstat>` 都必须含有 `<prop>` 子元素**。多数服务端在
    /// 部分失败时会写成 `<propstat><prop/><status>…</status></propstat>`，这种
    /// 形态可以正常解析；若服务端连 `<prop>` 都省略，整个响应会解析失败并返回
    /// [`MoveError::De`]。
    ///
    /// 本库不为此在本领域另写一套解析器——那会和 `propfind` 形成两套 `MultiStatus`
    /// 语义。需要在这种响应上取到 `href` 与状态码时，用 [`send`](Self::send) 拿原始
    /// 响应自行处理。
    pub async fn send_and_deserialize(self) -> Result<MultiStatus, MoveError> {
        let response = self.send().await?;
        let status = response.status();

        if status != StatusCode::MULTI_STATUS {
            return Err(MoveError::UnexpectedStatus(status));
        }

        let body = response.text().await?;

        Ok(MultiStatus::from_str(&body)?)
    }
}

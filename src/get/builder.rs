use reqwest::{
    Client, Request, Response,
    header::{HeaderMap, HeaderName, HeaderValue, RANGE},
};
use thiserror::Error;
use url::Url;

/// GET Builder 错误。
#[derive(Debug, Error)]
pub enum GetError {
    #[error("URL 格式错误: {0}")]
    Url(#[from] url::ParseError),

    #[error("请求头名称格式错误: {0}")]
    HeaderName(#[from] reqwest::header::InvalidHeaderName),

    #[error("请求头值格式错误: {0}")]
    HeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    /// 按段区间不合法：起点必须不大于终点。
    #[error("按段区间无效: 起点 {start} 大于终点 {end}")]
    Range { start: u64, end: u64 },

    #[error("构建 HTTP 请求失败: {0}")]
    Request(#[from] reqwest::Error),
}

/// GET 请求构建器：构建并发送一次 GET，返回未经处理的原始响应。
///
/// 路径由 [`relative_path`](Self::relative_path) 或
/// [`absolute_path`](Self::absolute_path) 设置；按段取用
/// [`range`](Self::range)；附加请求头用 [`header`](Self::header) 与
/// [`headers`](Self::headers)。库不维护任何分片状态。
pub struct GetBuilder {
    client: Client,
    base_url: Url,
    absolute_url: Url,
    headers: HeaderMap,
}

impl GetBuilder {
    /// 使用认证根地址创建默认 GET 请求。
    pub fn new(client: Client, base_url: Url) -> Self {
        Self {
            client,
            absolute_url: base_url.clone(),
            base_url,
            headers: HeaderMap::new(),
        }
    }

    /// 用相对路径来设置当前请求地址
    ///
    /// 路径里的空格和中文会被百分号编码。以 `/` 开头会从域名根开始解析，
    /// 覆盖掉根地址中已有的路径前缀；传入完整 URL 时直接使用该地址。
    ///
    /// 命名与 [`PutBuilder::relative_path`](crate::put::builder::PutBuilder::relative_path)
    /// 一致：两者都是「相对认证根的地址」。
    ///
    /// # Errors
    ///
    /// 路径语法非法（例如 `http://[::1`）时返回 [`GetError::Url`]，**不会 panic**。
    /// 空字符串与普通路径都是合法输入。
    ///
    /// ```
    /// use webdav_raw::{Client, GetBuilder, Url};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = GetBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?);
    /// let builder = builder.relative_path("目录/报告 1.pdf")?;
    /// # let _ = builder;
    /// # Ok(())
    /// # }
    /// ```
    pub fn relative_path(mut self, relative_path: &str) -> Result<Self, GetError> {
        self.absolute_url = self.base_url.join(relative_path)?;

        Ok(self)
    }

    /// 用完整 URL 来设置当前请求地址
    ///
    /// 命名与 [`PutBuilder::absolute_path`](crate::put::builder::PutBuilder::absolute_path)
    /// 一致。
    ///
    /// # Errors
    ///
    /// URL 语法非法时返回 [`GetError::Url`]，**不会 panic**。
    ///
    /// ```
    /// use webdav_raw::{Client, GetBuilder, Url};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = GetBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?);
    /// let builder = builder.absolute_path("https://cdn.example.com/file.bin")?;
    /// # let _ = builder;
    /// # Ok(())
    /// # }
    /// ```
    pub fn absolute_path(mut self, absolute_url: &str) -> Result<Self, GetError> {
        self.absolute_url = Url::parse(absolute_url)?;

        Ok(self)
    }

    /// 追加一个请求头。
    ///
    /// 同名请求头是**替换**语义：后设的覆盖先设的，不追加第二个同名头。
    /// 它与 [`range`](Self::range) 写的是同一个 `Range` 头，谁后设谁生效。
    ///
    /// # Errors
    ///
    /// 请求头名称非法时返回 [`GetError::HeaderName`]，取值非法时返回
    /// [`GetError::HeaderValue`]。
    ///
    /// ```
    /// use webdav_raw::{Client, GetBuilder, Url};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let request = GetBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?)
    ///     .relative_path("big.bin")?
    ///     .header("if-range", "\"83cc00-63d3371160718\"")?
    ///     .build()?;
    ///
    /// assert_eq!(
    ///     request.headers().get("if-range").unwrap(),
    ///     "\"83cc00-63d3371160718\""
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn header(mut self, name: &str, value: &str) -> Result<Self, GetError> {
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

    /// 设置 `Range` 请求头，只取这一段字节。
    ///
    /// 起止都是**闭区间**：`range(0, 1023)` 对应 `Range: bytes=0-1023`，共 1024
    /// 字节，与 [`U8BytesChunk`](crate::U8BytesChunk) 的区间写法一致。
    ///
    /// # 库不维护分片状态
    ///
    /// 分几段、按什么顺序发、失败怎么重试、是否带 `If-Range` 做版本校验，全部由
    /// 调用方决定。本方法只写 `Range` 这一个头，一次调用对应一次请求。
    ///
    /// # 答复形态
    ///
    /// 服务端是否支持按段取，答案在响应里：接受时回 `206 Partial Content` 并带
    /// `Content-Range`，忽略 `Range` 时回 `200` 加整份内容。本库不把两者分开，
    /// 调用方按状态码判定。
    ///
    /// # Errors
    ///
    /// 起点大于终点时返回 [`GetError::Range`]。
    ///
    /// ```
    /// use webdav_raw::{Client, GetBuilder, Url};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let request = GetBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?)
    ///     .relative_path("big.bin")?
    ///     .range(0, 1023)?
    ///     .build()?;
    ///
    /// assert_eq!(request.headers().get("range").unwrap(), "bytes=0-1023");
    /// # Ok(())
    /// # }
    /// ```
    pub fn range(mut self, start: u64, end: u64) -> Result<Self, GetError> {
        if start > end {
            return Err(GetError::Range { start, end });
        }

        let value = HeaderValue::from_str(&format!("bytes={start}-{end}"))?;

        self.headers.insert(RANGE, value);

        Ok(self)
    }

    pub fn build(&self) -> Result<Request, GetError> {
        let url = self.absolute_url.as_ref();
        let request = self
            .client
            .get(url)
            .headers(self.headers.clone())
            .build()?;
        Ok(request)
    }

    /// 发送 GET 请求并返回原始的 `reqwest::Response`。
    ///
    /// 此方法返回一个“文件句柄”式的响应对象，调用者可以完全控制如何处理响应体：
    /// - 使用 `response.bytes_stream()` 实现流式读取，适合大文件或分块下载。
    /// - 使用 `response.bytes()` 或 `response.text()` 一次性获取全部内容。
    /// - 通过 `response.headers()` 和 `response.status()` 检查元数据。
    ///
    /// 调用者可根据实际需求选择最合适的读取方式，此库不干涉任何 I/O 逻辑。
    ///
    /// 按段请求的答复同样原样交出，不做任何判断，见 [`range`](Self::range)。
    pub async fn send(self) -> Result<Response, GetError> {
        let request = self.build()?;
        let response = self.client.execute(request).await?;
        Ok(response)
    }
}

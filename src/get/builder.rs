use reqwest::{Client, Request, Response};
use thiserror::Error;
use url::Url;

/// GET Builder 错误。
#[derive(Debug, Error)]
pub enum GetError {
    #[error("URL 格式错误: {0}")]
    Url(#[from] url::ParseError),

    #[error("构建 HTTP 请求失败: {0}")]
    Request(#[from] reqwest::Error),
}

pub struct GetBuilder {
    client: Client,
    base_url: Url,
    absolute_url: Url,
}

impl GetBuilder {
    /// 使用认证根地址创建默认 GET 请求。
    pub fn new(client: Client, base_url: Url) -> Self {
        Self {
            client,
            absolute_url: base_url.clone(),
            base_url,
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
    /// use webdav_core::{Client, GetBuilder, Url};
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
    /// use webdav_core::{Client, GetBuilder, Url};
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

    pub fn build(&self) -> Result<Request, GetError> {
        let url = self.absolute_url.as_ref();
        let request = self.client.get(url).build()?;
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
    pub async fn send(self) -> Result<Response, GetError> {
        let request = self.build()?;
        let response = self.client.execute(request).await?;
        Ok(response)
    }
}

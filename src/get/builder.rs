use reqwest::{Client, Request, Response};
use thiserror::Error;
use url::Url;

/// GET Builder 错误。
#[derive(Debug, Error)]
pub enum GetError {
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
    pub fn relative_url(mut self, relative_path: String) -> Self {
        let joined = self
            .base_url
            .join(relative_path.as_ref())
            .expect("无效的相对路径，请确保路径格式正确");
        self.absolute_url = joined;
        self
    }

    /// 用绝对路径来设置当前请求地址
    pub fn absolute_url(mut self, absolute_url: String) -> Self {
        let parsed = Url::parse(absolute_url.as_ref()).expect("无效的绝对 URL");
        self.absolute_url = parsed;
        self
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

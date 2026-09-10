use std::collections::BTreeSet;

use reqwest::{Client, Method, Request, Response, StatusCode};
use thiserror::Error;
use url::Url;

use crate::propfind::raw_xml::multistatus::MultiStatus;

use super::find_props::{FindProp, PropFindSelector};

/// WebDAV Depth 请求头的值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Depth {
    /// 只查询当前资源。
    Zero,

    /// 查询当前资源以及直接子资源。
    One,

    /// 递归查询所有子资源。
    #[default]
    Infinity,
}

impl Depth {
    /// 转换成 HTTP 请求头中的字符串。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::One => "1",
            Self::Infinity => "infinity",
        }
    }
}

/// PROPFIND Builder 错误。
#[derive(Debug, Error)]
pub enum PropFindError {
    #[error("生成 PROPFIND XML 失败: {0}")]
    Xml(#[from] quick_xml::Error),

    #[error("拼接 WebDAV URL 失败: {0}")]
    Url(#[from] url::ParseError),

    #[error("构建 HTTP 请求失败: {0}")]
    Request(#[from] reqwest::Error),

    #[error("反序列化失败: {0}")]
    DeError(#[from] quick_xml::DeError),

    #[error("期望 207 Multi-Status，实际收到 {0}")]
    UnexpectedStatus(StatusCode),
}

/// WebDAV PROPFIND 请求构建器。
///
/// 默认配置：
///
/// - selector: `allprop`
/// - depth: `infinity`
pub struct PropFindBuilder {
    /// reqwest HTTP Client。
    client: Client,

    /// WebDAV 基础 URL。
    base_url: Url,

    /// 相对于 base_url 的资源路径。
    path: String,

    /// PROPFIND 属性选择器。
    selector: PropFindSelector,

    /// Depth 请求头。
    depth: Depth,
}

impl PropFindBuilder {
    /// 创建新的 PROPFIND Builder。
    pub fn new(client: Client, base_url: Url) -> Self {
        Self {
            client,
            base_url,
            path: String::new(),
            selector: PropFindSelector::AllProp,
            depth: Depth::Infinity,
        }
    }

    /// 设置请求路径。
    ///
    /// 推荐使用相对路径，例如：
    ///
    /// ```rust
    /// use webdav_core::{Client, Url};
    /// use webdav_core::propfind::builder::PropFindBuilder;
    ///
    /// let base_url = Url::parse("https://example.com/webdav/").unwrap();
    /// let builder = PropFindBuilder::new(Client::new(), base_url);
    /// let _builder = builder.path("Documents/");
    /// ```
    ///
    /// 如果传入以 `/` 开头的路径，`Url::join` 会从域名根路径开始拼接，
    /// 可能会覆盖 base_url 中已有的路径前缀。
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// 设置请求路径。
    ///
    /// 这是 `.path()` 的别名，可以让调用风格更接近部分 HTTP Builder API。
    pub fn uri(self, path: impl Into<String>) -> Self {
        self.path(path)
    }

    /// 设置 Depth 请求头。
    pub fn depth(mut self, depth: Depth) -> Self {
        self.depth = depth;
        self
    }

    /// 使用 `allprop` 查询全部属性。
    pub fn allprop(mut self) -> Self {
        self.selector = PropFindSelector::AllProp;
        self
    }

    /// 使用 `propname` 只查询属性名称。
    pub fn prop_name(mut self) -> Self {
        self.selector = PropFindSelector::PropName;
        self
    }

    /// 查询指定属性。
    ///
    /// 传入的属性会自动去重，并按照 `FindProp` 的排序顺序输出。
    ///
    /// 示例：
    ///
    /// ```rust
    /// use webdav_core::{Client, Url};
    /// use webdav_core::propfind::builder::PropFindBuilder;
    /// use webdav_core::propfind::find_props::FindProp;
    ///
    /// let base_url = Url::parse("https://example.com/webdav/").unwrap();
    /// let builder = PropFindBuilder::new(Client::new(), base_url);
    /// let _builder = builder.props([
    ///     FindProp::Getetag,
    ///     FindProp::Getcontenttype,
    ///     FindProp::Getetag,
    /// ]);
    /// ```
    pub fn props<I>(mut self, props: I) -> Self
    where
        I: IntoIterator<Item = FindProp>,
    {
        let props = props.into_iter().collect::<BTreeSet<_>>();

        self.selector = PropFindSelector::Props(props);
        self
    }

    /// 直接设置 PROPFIND 选择器。
    pub fn selector(mut self, selector: PropFindSelector) -> Self {
        self.selector = selector;
        self
    }

    /// 获取最终请求 URL。
    fn request_url(&self) -> Result<Url, PropFindError> {
        Ok(self.base_url.join(&self.path)?)
    }

    /// 获取 PROPFIND XML 请求体。
    fn request_body(&self) -> Result<String, PropFindError> {
        Ok(self.selector.to_xml()?)
    }

    /// 构建 reqwest Request，但不发送请求。
    pub fn build(&self) -> Result<Request, PropFindError> {
        let url = self.request_url()?;
        let body = self.request_body()?;

        let method = Method::from_bytes(b"PROPFIND").expect("PROPFIND is a valid HTTP method");

        let request = self
            .client
            .request(method, url)
            .header("Depth", self.depth.as_str())
            .header("Content-Type", "application/xml; charset=utf-8")
            .header("Accept", "application/xml, text/xml, */*")
            .body(body)
            .build()?;

        Ok(request)
    }

    /// 构建并发送 PROPFIND 请求。
    ///
    /// 这里的 `send(self)` 会消费 Builder，符合 Builder 的常见使用方式。
    /// 由于 `build()` 只借用 `self`，所以调用 `build()` 后仍然可以使用
    /// `self.client`。
    pub async fn send(self) -> Result<Response, PropFindError> {
        let request = self.build()?;
        let response = self.client.execute(request).await?;
        Ok(response)
    }

    pub async fn send_and_deserialize(self) -> Result<MultiStatus, PropFindError> {
        let request = self.build()?;
        let response = self.client.execute(request).await?;
        let status = response.status();
        if status != StatusCode::MULTI_STATUS {
            // 返回自定义错误，或直接尝试解析（有些服务器可能在非207也返回multistatus，但不符合规范）
            return Err(PropFindError::UnexpectedStatus(status)); // 需新增错误变体
        }
        let body = response.text().await?;
        let multistatus = MultiStatus::from_str(&body)?;
        Ok(multistatus)
    }
}

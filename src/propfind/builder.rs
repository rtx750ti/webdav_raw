use std::{collections::BTreeSet, sync::Arc};

use reqwest::{Client, Method, Request, Response};
use thiserror::Error;
use url::Url;

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
    /// .path("Documents/")
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
    /// let builder = builder.props([
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
}

#[cfg(test)]
mod test_propfind_builder {
    use super::*;
    use std::sync::Arc;
    use url::Url;
    use wiremock::matchers::{body_string, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // 辅助函数：创建基础 URL（已规范化为结尾带 /）
    fn base_url() -> Url {
        Url::parse("https://example.com/webdav/").unwrap()
    }

    // 辅助函数：创建默认客户端
    fn client() -> Client {
        Client::new()
    }

    // 辅助函数：创建默认 builder
    fn builder() -> PropFindBuilder {
        PropFindBuilder::new(client(), base_url())
    }

    // ========== Depth 枚举测试 ==========
    #[test]
    fn test_depth_as_str() {
        eprintln!("测试 Depth::as_str() 方法");
        assert_eq!(Depth::Zero.as_str(), "0");
        assert_eq!(Depth::One.as_str(), "1");
        assert_eq!(Depth::Infinity.as_str(), "infinity");
        eprintln!("Depth::as_str() 测试通过");
    }

    // ========== 路径拼接测试（重点） ==========
    // 覆盖 Linux 标准路径的各种情况

    #[test]
    fn test_request_url_empty_path() {
        eprintln!("测试空路径 '' 的 URL 拼接");
        let builder = builder();
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果：{}", url);
        assert_eq!(url.as_str(), "https://example.com/webdav/");
    }

    #[test]
    fn test_request_url_relative_path_no_slash() {
        eprintln!("测试相对路径 'Documents' 的 URL 拼接");
        let builder = builder().path("Documents");
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果：{}", url);
        assert_eq!(url.as_str(), "https://example.com/webdav/Documents");
    }

    #[test]
    fn test_request_url_relative_path_with_slash() {
        eprintln!("测试相对路径 'Documents/' 的 URL 拼接");
        let builder = builder().path("Documents/");
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果：{}", url);
        assert_eq!(url.as_str(), "https://example.com/webdav/Documents/");
    }

    #[test]
    fn test_request_url_absolute_path_overrides_base() {
        eprintln!("测试绝对路径 '/other' 的 URL 拼接");
        let builder = builder().path("/other");
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果：{}", url);
        // Url::join 会用绝对路径替换掉 base_url 的路径
        assert_eq!(url.as_str(), "https://example.com/other");
    }

    #[test]
    fn test_request_url_path_with_dot_segments() {
        eprintln!("测试路径包含 '.' 和 '..' 的 URL 拼接");
        let builder = builder().path("a/./b/../c");
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果：{}", url);
        // Url::join 会自动规范化路径
        assert_eq!(url.as_str(), "https://example.com/webdav/a/c");
    }

    #[test]
    fn test_request_url_path_with_leading_dotdot() {
        eprintln!("测试路径以 '..' 开头");
        let builder = builder().path("../escape");
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果：{}", url);
        // 返回上级目录，但不会越过域名根
        assert_eq!(url.as_str(), "https://example.com/escape");
    }

    #[test]
    fn test_request_url_path_with_spaces_and_unicode() {
        eprintln!("测试路径包含空格和中文");
        let builder = builder().path("我的 文档/报告.pdf");
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果：{}", url);
        // 空格和中文会自动百分号编码
        assert_eq!(
            url.as_str(),
            "https://example.com/webdav/%E6%88%91%E7%9A%84%20%E6%96%87%E6%A1%A3/%E6%8A%A5%E5%91%8A.pdf"
        );
    }

    #[test]
    fn test_request_url_path_with_semicolon_and_colon() {
        eprintln!("测试路径包含 ';' 和 ':'");
        let builder = builder().path("dir/file:1;2");
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果：{}", url);
        assert_eq!(url.as_str(), "https://example.com/webdav/dir/file:1;2");
    }

    #[test]
    fn test_request_url_path_with_multiple_slashes() {
        eprintln!("测试路径包含连续斜杠 '//'");
        let builder = builder().path("a//b///c");
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果：{}", url);
        // Url::join 会保留多余的斜杠
        assert_eq!(url.as_str(), "https://example.com/webdav/a//b///c");
    }

    #[test]
    fn test_request_url_path_with_trailing_dot() {
        eprintln!("测试路径以 '.' 结尾");
        let builder = builder().path("folder.");
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果：{}", url);
        assert_eq!(url.as_str(), "https://example.com/webdav/folder.");
    }

    #[test]
    fn test_request_url_long_path() {
        eprintln!("测试超长路径（2000 字符）");
        let long_segment = "a".repeat(2000);
        let builder = builder().path(&long_segment);
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果长度：{}", url.as_str().len());
        assert!(url.as_str().starts_with("https://example.com/webdav/"));
        assert_eq!(
            url.as_str().len(),
            "https://example.com/webdav/".len() + 2000
        );
    }

    #[test]
    fn test_request_url_error_on_non_absolute_base() {
        eprintln!("测试非法 base_url 导致 Url::join 失败");
        // 构造一个无法作为 base 的 URL（如 data: URL）
        let invalid_base = Url::parse("data:text/plain,hello").unwrap();
        let client = client();
        let builder = PropFindBuilder::new(client, invalid_base).path("anything");
        let result = builder.request_url();
        eprintln!("错误结果：{:?}", result);
        assert!(result.is_err());
        // 确认错误类型是 url::ParseError
        match result {
            Err(PropFindError::Url(_)) => {}
            _ => panic!("期望 Url 错误"),
        }
    }

    // ========== 属性选择器测试 ==========

    #[test]
    fn test_request_body_default_allprop() {
        eprintln!("测试默认 allprop XML 生成");
        let builder = builder();
        let xml = builder.request_body().unwrap();
        eprintln!("生成的 XML：\n{}", xml);
        assert_eq!(
            xml,
            r#"<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#
        );
    }

    #[test]
    fn test_request_body_prop_name() {
        eprintln!("测试 propname XML 生成");
        let builder = builder().prop_name();
        let xml = builder.request_body().unwrap();
        eprintln!("生成的 XML：\n{}", xml);
        assert_eq!(
            xml,
            r#"<D:propfind xmlns:D="DAV:"><D:propname/></D:propfind>"#
        );
    }

    #[test]
    fn test_request_body_props_deduplication_and_order() {
        eprintln!("测试自定义 props 去重和排序");
        let builder = builder().props([
            FindProp::Getetag,
            FindProp::Getcontenttype,
            FindProp::Getetag, // 重复项
            FindProp::Resourcetype,
        ]);
        let xml = builder.request_body().unwrap();
        eprintln!("生成的 XML：\n{}", xml);
        // BTreeSet 排序顺序：creationdate, getcontenttype, getetag, getlastmodified, resourcetype
        // 实际顺序按枚举定义的 Ord 派生，由于枚举变量按声明顺序，所以排序结果为：
        // Resourcetype, Creationdate, Getetag, Getlastmodified, Getcontenttype
        // 注意：Ord derive 按声明顺序，所以 Resourcetype < Creationdate < Getetag < Getlastmodified < Getcontenttype
        // 我们传入的是 Getetag, Getcontenttype, Resourcetype，去重排序后应该是 Resourcetype, Getetag, Getcontenttype
        let expected = r#"<D:propfind xmlns:D="DAV:"><D:prop><D:resourcetype/><D:getetag/><D:getcontenttype/></D:prop></D:propfind>"#;
        assert_eq!(xml, expected);
    }

    #[test]
    fn test_request_body_selector_direct() {
        eprintln!("测试直接设置 selector");
        let selector = PropFindSelector::Props(BTreeSet::from([FindProp::Creationdate]));
        let builder = builder().selector(selector);
        let xml = builder.request_body().unwrap();
        eprintln!("生成的 XML：\n{}", xml);
        assert_eq!(
            xml,
            r#"<D:propfind xmlns:D="DAV:"><D:prop><D:creationdate/></D:prop></D:propfind>"#
        );
    }

    // ========== build() 方法测试 ==========

    #[test]
    fn test_build_basic_request() {
        eprintln!("测试 build() 生成基本 PROPFIND 请求");
        let builder = builder().path("folder").depth(Depth::One).allprop();
        let request = builder.build().unwrap();
        eprintln!("请求方法：{}", request.method());
        eprintln!("请求 URL：{}", request.url());
        eprintln!("请求头：\n{:?}", request.headers());
        let body = request.body().unwrap().as_bytes().unwrap();
        let body_str = std::str::from_utf8(body).unwrap();
        eprintln!("请求体：\n{}", body_str);

        assert_eq!(request.method(), Method::from_bytes(b"PROPFIND").unwrap());
        assert_eq!(request.url().as_str(), "https://example.com/webdav/folder");
        assert_eq!(request.headers().get("Depth").unwrap(), "1");
        assert_eq!(
            request.headers().get("Content-Type").unwrap(),
            "application/xml; charset=utf-8"
        );
        assert_eq!(
            request.headers().get("Accept").unwrap(),
            "application/xml, text/xml, */*"
        );
        assert_eq!(
            body_str,
            r#"<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#
        );
    }

    #[test]
    fn test_build_error_on_invalid_url() {
        eprintln!("测试 build() 在 URL 错误时返回错误");
        let invalid_base = Url::parse("data:text/plain,hello").unwrap();
        let client = client();
        let builder = PropFindBuilder::new(client, invalid_base).path("x");
        let result = builder.build();
        eprintln!("错误结果：{:?}", result);
        assert!(matches!(result, Err(PropFindError::Url(_))));
    }

    // ========== send() 方法集成测试（使用 mock 服务器） ==========

    #[tokio::test]
    async fn test_send_request_to_mock_server() {
        eprintln!("开始 mock 服务器集成测试");

        // 创建 mock 服务器
        let mock_server = MockServer::start().await;
        eprintln!("Mock 服务器地址：{}", mock_server.uri());

        // 设置期望的请求和响应
        Mock::given(method("PROPFIND"))
            .and(path("/webdav/Documents"))
            .and(header("Depth", "1"))
            .and(header("Content-Type", "application/xml; charset=utf-8"))
            .and(body_string(
                r#"<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#,
            ))
            .respond_with(ResponseTemplate::new(207).set_body_string("<D:multistatus/>"))
            .expect(1)
            .mount(&mock_server)
            .await;

        // 使用 mock 服务器地址构建 base_url
        let base_url = Url::parse(&format!("{}/webdav/", mock_server.uri())).unwrap();
        let client = Client::new();
        let builder = PropFindBuilder::new(client, base_url)
            .path("Documents")
            .depth(Depth::One)
            .allprop();

        // 发送请求
        let response = builder.send().await.unwrap();
        eprintln!("响应状态：{}", response.status());
        let status = response.status();
        let body = response.text().await.unwrap();
        eprintln!("响应体：{}", body);
        assert_eq!(status, 207);
        assert_eq!(body, "<D:multistatus/>");
        eprintln!("Mock 服务器集成测试通过");
    }

    #[tokio::test]
    async fn test_send_request_defaults() {
        eprintln!("测试 send() 使用默认配置");

        let mock_server = MockServer::start().await;
        Mock::given(method("PROPFIND"))
            .and(path("/webdav/"))
            .and(header("Depth", "infinity"))
            .and(body_string(
                r#"<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#,
            ))
            .respond_with(ResponseTemplate::new(207))
            .expect(1)
            .mount(&mock_server)
            .await;

        let base_url = Url::parse(&format!("{}/webdav/", mock_server.uri())).unwrap();
        let client = Client::new();
        let builder = PropFindBuilder::new(client, base_url);

        let response = builder.send().await.unwrap();
        assert_eq!(response.status(), 207);
        eprintln!("默认配置 send() 测试通过");
    }

    // ========== uri() 别名测试 ==========
    #[test]
    fn test_uri_alias_sets_path() {
        eprintln!("测试 uri() 别名方法");
        let builder = builder().uri("alias/path");
        let url = builder.request_url().unwrap();
        eprintln!("拼接结果：{}", url);
        assert_eq!(url.as_str(), "https://example.com/webdav/alias/path");
    }

    // ========== 测试真实webdav请求 ==========
    #[test]
    #[cfg(feature = "network-test")]
    fn test_fetch_real_webdav() {
        // 需要网络的测试，仅当启用 feature 时才编译
        assert_eq!(1, 1);
    }
}

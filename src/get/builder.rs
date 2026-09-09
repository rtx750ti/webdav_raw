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
    absolute_url: Url,
}

impl GetBuilder {
    pub fn new(client: Client, absolute_url: &Url) -> Self {
        Self {
            client,
            absolute_url: absolute_url.clone(),
        }
    }

    pub fn absolute_url(mut self, absolute_url: Url) -> Self {
        self.absolute_url = absolute_url;
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// 创建测试客户端。
    fn test_client() -> Client {
        Client::new()
    }

    /// 基于标准 WebDAV base URL 构造绝对 URL。
    fn webdav_url(relative_path: &str) -> Url {
        let base = "https://www.jianguoyun.com/dav/";
        let path = relative_path.trim_start_matches('/');
        Url::parse(&format!("{}{}", base, path)).unwrap()
    }

    /// 测试 build 使用绝对 URL。
    #[test]
    fn test_build_uses_absolute_url() {
        let client = test_client();
        let url = webdav_url("Documents/report.pdf");
        let builder = GetBuilder::new(client, &url);

        let request = builder.build().unwrap();

        eprintln!("url: {}", url);
        eprintln!("method: {}", request.method());
        eprintln!("request_url: {}", request.url());

        assert_eq!(request.method(), "GET");
        assert_eq!(request.url().as_str(), url.as_str());
    }

    /// 测试 absolute_url 修改 URL。
    #[test]
    fn test_absolute_url_changes_url() {
        let client = test_client();
        let url1 = webdav_url("first.txt");
        let url2 = webdav_url("second.txt");
        let builder = GetBuilder::new(client, &url1).absolute_url(url2.clone());

        let request = builder.build().unwrap();

        eprintln!("url1: {}", url1);
        eprintln!("url2: {}", url2);
        eprintln!("request_url: {}", request.url());

        assert_eq!(request.url().as_str(), url2.as_str());
    }

    /// 测试 send 返回响应。
    #[tokio::test]
    async fn test_send_returns_response() {
        let server = MockServer::start().await;
        let expected_body = "Hello, WebDAV!";
        Mock::given(method("GET"))
            .and(path("/dav/resource"))
            .respond_with(ResponseTemplate::new(200).set_body_string(expected_body))
            .mount(&server)
            .await;

        let client = Client::new();
        let url = Url::parse(&format!("{}/dav/resource", server.uri())).unwrap();
        let builder = GetBuilder::new(client, &url);

        let response = builder.send().await.unwrap();

        let res_status = response.status();

        eprintln!("status: {}", res_status);
        let body = response.text().await.unwrap();
        eprintln!("body: {}", body);

        assert_eq!(res_status, 200);
        assert_eq!(body, expected_body);
    }

    /// 测试构建带非标准端口的 URL。
    #[test]
    fn test_build_with_non_standard_port() {
        let client = test_client();
        let url = Url::parse("https://www.jianguoyun.com:8443/dav/test1/file.txt").unwrap();
        let builder = GetBuilder::new(client, &url);

        let request = builder.build().unwrap();

        eprintln!("url: {}", url);
        eprintln!("request_url: {}", request.url());

        assert_eq!(request.url().as_str(), url.as_str());
    }
}

#[cfg(test)]
#[cfg(feature = "network-test")]
mod network_tests {
    use super::*;
    use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
    use std::time::Duration;
    use url::Url;

    // ========== 测试用路径常量 ==========
    /// 服务器上真实存在的文件路径（用于测试下载成功）
    const EXISTING_FILE: &str = "测试文件夹/fast-sync.exe";
    /// 不存在的文件路径（用于测试 404）
    const NONEXISTENT_FILE: &str = "non/existent/file.txt";

    /// 创建带认证和自定义超时的 WebDAV 客户端
    fn setup_webdav(override_password: Option<&str>, timeout_secs: u64) -> (Client, Url) {
        use base64::prelude::*;
        use std::env;

        let url_str = env::var("WEBDAV_URL").expect("WEBDAV_URL not set");
        let account = env::var("WEBDAV_ACCOUNT").expect("WEBDAV_ACCOUNT not set");
        let password = override_password
            .map(|s| s.to_string())
            .unwrap_or_else(|| env::var("WEBDAV_PASSWORD").expect("WEBDAV_PASSWORD not set"));

        let base_url = if url_str.ends_with('/') {
            Url::parse(&url_str).unwrap()
        } else {
            Url::parse(&format!("{}/", url_str)).unwrap()
        };

        let credentials = format!("{}:{}", account, password);
        let auth_value = format!("Basic {}", BASE64_STANDARD.encode(credentials.as_bytes()));
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value).expect("Invalid Authorization header"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to build reqwest client");

        (client, base_url)
    }

    /// 构造绝对 URL
    fn absolute_url(base_url: &Url, relative_path: &str) -> Url {
        let clean = relative_path.trim_start_matches('/');
        base_url.join(clean).unwrap()
    }

    // ========== 测试：下载存在的文件（验证状态码和 Content-Length） ==========
    #[tokio::test]
    async fn test_download_existing_file() {
        let (client, base_url) = setup_webdav(None, 60);
        let url = absolute_url(&base_url, EXISTING_FILE);
        eprintln!("下载 URL: {}", url);

        let builder = GetBuilder::new(client, &url);
        let response = builder.send().await.expect("GET request failed");
        let status = response.status();
        let content_length = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        eprintln!("状态码: {}", status);
        if let Some(len) = content_length {
            eprintln!("Content-Length: {} 字节", len);
        }

        assert_eq!(status, 200);
        assert!(
            content_length.map_or(false, |len| len > 0),
            "文件大小应为正数"
        );
        eprintln!("文件下载测试通过 ✓");
    }

    // ========== 测试：下载不存在的文件 ==========
    #[tokio::test]
    async fn test_download_nonexistent_file() {
        let (client, base_url) = setup_webdav(None, 10);
        let url = absolute_url(&base_url, NONEXISTENT_FILE);
        eprintln!("不存在的 URL: {}", url);

        let builder = GetBuilder::new(client, &url);
        let result = builder.send().await;

        match result {
            Ok(resp) => {
                let status = resp.status();
                eprintln!("意外成功，状态码: {}", status);
                assert_ne!(status, 200, "不应返回 200 OK");
            }
            Err(e) => {
                eprintln!("请求失败（符合预期）: {}", e);
            }
        }
        eprintln!("下载不存在的文件测试通过 ✓");
    }

    // ========== 测试：错误密码（认证失败） ==========
    #[tokio::test]
    async fn test_download_with_wrong_credentials() {
        let (client, base_url) = setup_webdav(Some("wrong_password"), 10);
        // 复用真实存在文件的路径，以便正确触发认证失败（如果文件存在但密码错误）
        let url = absolute_url(&base_url, EXISTING_FILE);

        let builder = GetBuilder::new(client, &url);
        let result = builder.send().await;

        match result {
            Ok(resp) => {
                let status = resp.status();
                eprintln!("意外成功（可能服务器未认证）状态码: {}", status);
                assert!(status == 401 || status == 403, "应返回 401 或 403");
            }
            Err(e) => {
                eprintln!("请求失败（符合预期）: {}", e);
            }
        }
        eprintln!("错误密码测试通过 ✓");
    }
}

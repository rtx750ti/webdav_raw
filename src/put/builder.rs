//! PUT 请求构建与发送。
//!
//! 一个 Builder 只构建一次 PUT；分片上传由调用方循环构建每一次请求。

use crate::put::put_body::{
    PutBody, PutData,
    u8_bytes::U8Bytes, u8_bytes_chunk::U8BytesChunk,
};
use reqwest::{
    Body, Client, Method, Request, Response,
    header::{CONTENT_LENGTH, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use thiserror::Error;
use url::Url;

/// 数据源没有给出内容类型时使用的默认值。
const DEFAULT_CONTENT_TYPE: &str = "application/octet-stream";

/// PUT 请求构建和发送错误。
#[derive(Debug, Error)]
pub enum PutError {
    #[error("构建 HTTP 请求失败: {0}")]
    Request(#[from] reqwest::Error),
    #[error("URL 格式错误: {0}")]
    Url(#[from] url::ParseError),
    /// 文件源取长度失败，即 `File::metadata` 失败。
    ///
    /// 这不是「读取文件内容失败」：内容读取发生在请求发送阶段，其失败由
    /// [`PutError::Request`] 承载。
    #[error("读取文件长度失败: {0}")]
    FileMetadata(#[from] std::io::Error),
    #[error("请求头名称格式错误: {0}")]
    HeaderName(#[from] reqwest::header::InvalidHeaderName),
    #[error("请求头值格式错误: {0}")]
    HeaderValue(#[from] reqwest::header::InvalidHeaderValue),
}

/// 已经解析成可发送形态的请求载荷。
///
/// 只在构建过程中短暂存在，不外露给调用方。
struct PutPayload {
    body: Body,
    /// 写 `Content-Length` 用的字节长度，来自数据自身。
    length: u64,
    /// 写 `Content-Type` 用的内容类型，只有内存源才有。
    content_type: Option<String>,
}

impl PutPayload {
    /// 没有数据源时的空载荷。
    fn empty() -> Self {
        Self {
            body: Body::from(Vec::new()),
            length: 0,
            content_type: None,
        }
    }

    /// 把三种数据源各自转换成 `reqwest::Body`。
    ///
    /// - 内存源：字节已在内存，直接交出所有权。
    /// - 分片源：同内存源；范围属于分片自身，本库默认不写成请求头。
    /// - 文件源：把句柄交给 reqwest 流式发送，字节不整体读入内存；长度通过
    ///   `File::metadata` 取得，这是整个构建过程唯一的一次 I/O。
    ///
    /// # Errors
    ///
    /// 文件源取长度失败时返回 [`PutError::FileMetadata`]。
    async fn resolve(body: Option<PutBody>) -> Result<Self, PutError> {
        let Some(body) = body else {
            return Ok(Self::empty());
        };

        match body.data {
            PutData::U8Bytes(bytes) => Ok(Self::from_memory(bytes)),
            PutData::U8BytesChunk(chunk) => Ok(Self::from_chunk(chunk)),
            PutData::File(handle) => {
                let file = handle.data;
                let length = file.metadata().await?.len();

                Ok(Self {
                    body: Body::from(file),
                    length,
                    content_type: None,
                })
            }
        }
    }

    /// 完整内存二进制：长度取数据自证的值，内容类型随元数据一起带出。
    fn from_memory(bytes: U8Bytes) -> Self {
        // 先取走长度与内容类型，再消费 data 交出字节所有权。
        let length = bytes.data.length;
        let content_type = Some(bytes.metadata.content_type);

        Self {
            body: Body::from(bytes.data.into_data()),
            length,
            content_type,
        }
    }

    /// 单个业务分片：长度同样取数据自证的值；范围不在这里处理。
    fn from_chunk(chunk: U8BytesChunk) -> Self {
        let length = chunk.data.length;

        Self {
            body: Body::from(chunk.data.into_data()),
            length,
            content_type: None,
        }
    }
}

/// 一次 PUT 请求构建器。
///
/// 一个 Builder 构建一个请求：做分片上传时，构建出来的就是那一个分片的请求，
/// 分几片、按什么顺序发，全部由调用方决定。
///
/// 数据源只有文件句柄、完整内存二进制、单个业务分片三种，没有通用字节流。
///
/// 通常由 [`WebdavAuth::put`](crate::auth::WebdavAuth::put) 创建，复用认证
/// Client 与根地址；不设置目标路径时指向根地址本身。
///
/// # 本库负责的请求头
///
/// - `Content-Length`：始终取数据源自身的长度（内存源是字节长度，文件源是
///   `File::metadata` 的长度），**调用方设的值不生效**——长度只有一个可信来源，
///   不提供覆盖入口。
/// - `Content-Type`：优先取调用方设置的值；否则取内存源元数据里的值；
///   都没有就用 `application/octet-stream`。
///
/// 其余请求头一律由调用方通过 [`header`](Self::header) /
/// [`headers`](Self::headers) 设置。
///
/// # 分片与 `Content-Range`
///
/// 本库**不会**自动把分片范围写成 `Content-Range`。标准 WebDAV 的 PUT 是单次
/// 资源写入，服务端是否支持部分上传无法可靠探测——服务端忽略该头时通常回
/// 200/201/204，与"支持并成功"无法区分，会静默把整份资源覆盖成这一个分片。
///
/// 需要发送该头时由调用方显式设置：
///
/// ```
/// use webdav_core::{PutBody, PutBuilder, U8BytesChunk, U8BytesData};
/// # fn example(builder: PutBuilder) -> Result<(), Box<dyn std::error::Error>> {
/// let data = U8BytesData::new(vec![0; 4], None)?;
/// let chunk = U8BytesChunk::new(data, Some(0), Some(3), Some(8), None)?;
/// let range = chunk.to_content_range().expect("分片带范围");
///
/// let builder = builder
///     .body(PutBody::from_chunk(chunk))
///     .header("content-range", &range)?;
/// # let _ = builder;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct PutBuilder {
    client: Client,
    base_url: Url,
    target_path: Url,
    body: Option<PutBody>,
    headers: HeaderMap,
}

impl PutBuilder {
    /// 使用认证根地址创建 PUT Builder。
    ///
    /// 通常不需要直接调用，[`WebdavAuth::put`](crate::auth::WebdavAuth::put)
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
    /// 覆盖掉根地址中已有的路径前缀。
    ///
    /// ```
    /// use webdav_core::PutBuilder;
    /// use webdav_core::{Client, Url};
    ///
    /// let builder = PutBuilder::new(Client::new(), Url::parse("https://example.com/dav/")?);
    /// let builder = builder.relative_path("目录/报告 1.txt")?;
    /// # let _ = builder;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn relative_path(mut self, path: &str) -> Result<Self, PutError> {
        self.target_path = self.base_url.join(path)?;

        Ok(self)
    }

    /// 用完整 URL 设置目标地址，不使用认证根地址。
    pub fn absolute_path(mut self, path: &str) -> Result<Self, PutError> {
        self.target_path = Url::parse(path)?;

        Ok(self)
    }

    /// 设置一次 PUT 的数据源。不设置就发送零长度请求体。
    pub fn body(mut self, body: PutBody) -> Self {
        self.body = Some(body);

        self
    }

    /// 追加一个请求头。
    ///
    /// 这里设的 `Content-Type` 优先于默认值。`Content-Length` 不受影响：它始终
    /// 由数据源自身的长度决定。
    pub fn header(mut self, name: &str, value: &str) -> Result<Self, PutError> {
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
    /// 文件源会在这里读一次文件长度，这是整个构建过程中唯一的一次 I/O；字节本身
    /// 在请求发送阶段才被读取，不会整体进入内存。
    pub async fn build(self) -> Result<Request, PutError> {
        let Self {
            client,
            base_url: _,
            target_path,
            body,
            mut headers,
        } = self;

        let payload = PutPayload::resolve(body).await?;

        apply_headers(&mut headers, &payload)?;

        Ok(client
            .request(Method::PUT, target_path)
            .headers(headers)
            .body(payload.body)
            .build()?)
    }

    /// 构建并发送，返回未经处理的原始响应。
    ///
    /// 状态码、响应头和响应体都原样交给调用方，本库不做任何判断——与 GET 的
    /// 行为保持一致。
    pub async fn send(self) -> Result<Response, PutError> {
        let client = self.client.clone();
        let request = self.build().await?;

        Ok(client.execute(request).await?)
    }
}

/// 补全本库负责的请求头。
///
/// - `Content-Length`：始终按载荷长度写入，不接受调用方覆盖。
/// - `Content-Type`：调用方设过就保留；否则用载荷的内容类型；都没有就用
///   [`DEFAULT_CONTENT_TYPE`]。
///
/// # Errors
///
/// 载荷的内容类型写不成合法请求头取值时返回 [`PutError::HeaderValue`]。
fn apply_headers(headers: &mut HeaderMap, payload: &PutPayload) -> Result<(), PutError> {
    headers.insert(CONTENT_LENGTH, HeaderValue::from(payload.length));

    match headers.entry(CONTENT_TYPE) {
        reqwest::header::Entry::Occupied(_) => {}
        reqwest::header::Entry::Vacant(slot) => {
            let value = match &payload.content_type {
                Some(content_type) => HeaderValue::from_str(content_type)?,
                None => HeaderValue::from_static(DEFAULT_CONTENT_TYPE),
            };

            slot.insert(value);
        }
    }

    Ok(())
}

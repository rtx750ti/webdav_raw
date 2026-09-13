use tokio::fs::File;

/// 由调用方提供的 Tokio 异步文件句柄。
///
/// 字段全部公开，调用方可以自由设置或改写。
///
/// # 为什么只接收 `tokio::fs::File`
///
/// 这里不抽象成通用 `AsyncRead`，是有意的取舍：
///
/// - `tokio::fs::File` 的长度在构建请求时可由 `metadata()` 确定，因此请求带
///   确定的 `Content-Length`，不会退化为 `Transfer-Encoding: chunked`，
///   字节也不会整体读进内存。
/// - 通用 `AsyncRead` 没有已知长度，会强制 chunked，且 `Body::try_clone()` 对
///   流返回 `None`，307/308 重定向无法跟随。要支持它就得额外引入"长度可空"
///   和"禁止重定向"两个概念。
///
/// 需要通用流的调用方，用认证对象公开的原始 Client 直接走 reqwest 更省事。
///
/// # 长度来源
///
/// 本类型不携带长度字段，也不接受调用方声明的长度：长度由本库在构建请求时
/// 通过 `File::metadata()` 读取，与后续实际发送的字节同源。
#[derive(Debug)]
pub struct FileHandle {
    pub data: File,
    /// 调用方追踪标识，给调用方好搜索；不写进请求头。
    pub id: Option<String>,
}

impl FileHandle {
    /// 创建文件句柄数据源。
    ///
    /// 长度不在这里读取，也不由调用方提供：构建请求时由本库通过
    /// `File::metadata()` 获取。
    pub fn new(data: File, id: Option<String>) -> Self {
        Self { data, id }
    }
}

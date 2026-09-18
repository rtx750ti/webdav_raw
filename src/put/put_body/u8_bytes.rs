use crate::put::put_body::u8_bytes_data::U8BytesData;
use reqwest::header::HeaderValue;
use thiserror::Error;

/// 内存二进制元数据构造错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum U8BytesError {
    #[error("文件名不能为空")]
    EmptyName,
    #[error("Content-Type 格式错误: {0}")]
    InvalidContentType(String),
}

/// 完整内存二进制的伴随信息。
///
/// 字段全部公开，调用方可以自由设置或改写。
///
/// # 哪些字段参与 HTTP 请求
///
/// - `content_type`：**参与**。映射为请求的 `Content-Type`。
/// - `name`：不参与。本库不用它推导请求地址，目标路径由 Builder 管理。
/// - `etag`、`last_modified`、`create_time`、`update_time`：不参与，仅供调用方记录。
///   本库不会把 `etag` 自动变成 `If-Match`，要发什么条件请求头由调用方自己设。
///
/// # 声明了内容类型不等于服务端会照此存储
///
/// 本库只保证把 `content_type` 写进请求头，**不保证服务端采纳**。真实验收中已经
/// 遇到 Apache `mod_dav` 完全忽略客户端声明的类型：GET 时按文件扩展名重新推断，
/// PROPFIND 的 `getcontenttype` 甚至固定回占位值。
///
/// 所以「上传的文件被识别成什么类型」主要取决于**文件名**，而不是这里的取值。
/// 要让服务端认对类型，把扩展名写对通常比声明 `Content-Type` 更有效。
///
/// # 内容类型
///
/// [`from_name`](Self::from_name) 提供**默认推断**：按文件名交给 `mime_guess`，
/// 调用方通常不必自己写 `Content-Type`。推断结果永远是合法 MIME。
///
/// 要自定义时直接给 `content_type` 赋值即可，本库不再代为校验——该字段会成为
/// 请求头取值，写错会导致构建或发送请求失败，或让服务端错误处理请求。
/// 需要校验时用 [`validate`](Self::validate) 自行检查。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct U8Metadata {
    /// 文件名。本库不用它推导请求地址。
    pub name: String,
    /// 要写进请求头的内容类型。
    pub content_type: String,
    /// 实体标签，仅供调用方记录。
    pub etag: Option<String>,
    /// 最后修改时间，仅供调用方记录。
    pub last_modified: Option<String>,
    /// 创建时间，仅供调用方记录。
    pub create_time: Option<String>,
    /// 更新时间，仅供调用方记录。
    pub update_time: Option<String>,
}

impl U8Metadata {
    /// 按文件名推断内容类型来创建元数据。
    ///
    /// 这是**本库提供的默认推断**：推断交给 `mime_guess`，本库不维护扩展名表。
    /// 无法识别的扩展名与完全没有扩展名的文件名都会回落为
    /// `application/octet-stream`。
    ///
    /// 推断结果永远是合法 MIME，因此这个入口只在文件名为空时报错。
    ///
    /// ```
    /// use webdav_raw::U8Metadata;
    ///
    /// let archive = U8Metadata::from_name("report.zip".to_owned()).unwrap();
    /// assert_eq!(archive.content_type, "application/zip");
    ///
    /// // 没有注册 MIME 类型的扩展名、以及没有扩展名的文件名都回落为默认值。
    /// let unknown = U8Metadata::from_name("installer".to_owned()).unwrap();
    /// assert_eq!(unknown.content_type, "application/octet-stream");
    /// assert_eq!(unknown.name, "installer");
    ///
    /// // 空文件名被拒绝。
    /// assert!(U8Metadata::from_name("  ".to_owned()).is_err());
    /// ```
    pub fn from_name(name: String) -> Result<Self, U8BytesError> {
        if name.trim().is_empty() {
            return Err(U8BytesError::EmptyName);
        }

        let content_type = mime_guess::from_path(name.as_str())
            .first_or_octet_stream()
            .to_string();

        Ok(Self {
            name,
            content_type,
            ..Self::default()
        })
    }

    /// 校验当前取值：名称非空、内容类型是合法 MIME 且能写成合法请求头取值。
    ///
    /// 字段公开后本库不再在写入时校验，这个方法供调用方自行检查，尤其是直接
    /// 给 `content_type` 赋过值之后。
    ///
    /// 校验只看元数据本身，**不会**检查它与 [`U8Bytes::data`] 是否配套；数据源
    /// 与元数据是正交的两件事。
    ///
    /// ```
    /// use webdav_raw::U8Metadata;
    ///
    /// let mut metadata = U8Metadata::from_name("report.zip".to_owned()).unwrap();
    /// assert!(metadata.validate().is_ok());
    ///
    /// metadata.content_type = "no-slash".to_owned();
    /// assert!(metadata.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), U8BytesError> {
        if self.name.trim().is_empty() {
            return Err(U8BytesError::EmptyName);
        }

        // 内容类型要同时满足两件事：是合法 MIME，并且能写成合法请求头取值。
        //
        // 第二条目前不会独立命中：`mime` 的解析器只接受可见 ASCII，凡是能被它
        // 接受的内容类型，`HeaderValue` 也一定接受。之所以还留着它，是因为
        // "能不能进请求头"的规则属于 `http` crate，不属于 `mime` crate——
        // 哪天 `mime` 放宽了引号参数的取值范围，这里仍然是最后一道拦截。
        // 因此这里合成一个判断，而不是拆成两个各自报错的检查点。
        let is_mime = self.content_type.parse::<mime::Mime>().is_ok();
        let is_header_safe = HeaderValue::from_str(&self.content_type).is_ok();

        if is_mime && is_header_safe {
            return Ok(());
        }

        Err(U8BytesError::InvalidContentType(self.content_type.clone()))
    }
}

/// 一次 PUT 使用的完整内存二进制：数据 + 伴随信息。
///
/// 长度由 [`U8BytesData`] 提供，本类型不再持有长度，也不接受调用方声明长度。
///
/// 追踪标识只保留在 [`U8BytesData::id`] 一处，本类型不再重复持有。
///
/// [`U8BytesData`]: crate::put::put_body::u8_bytes_data::U8BytesData
/// [`U8BytesData::id`]: crate::put::put_body::u8_bytes_data::U8BytesData
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U8Bytes {
    /// 字节本体，长度由它自己的 `length` 字段自证。
    pub data: U8BytesData,
    /// 伴随信息，其中只有 `content_type` 会进入请求头。
    pub metadata: U8Metadata,
}

impl U8Bytes {
    /// 创建完整内存二进制。
    ///
    /// ```
    /// use webdav_raw::{U8Bytes, U8BytesData, U8Metadata};
    ///
    /// let data = U8BytesData::new(vec![1, 2, 3], None).unwrap();
    /// let metadata = U8Metadata::from_name("report.txt".to_owned()).unwrap();
    /// let bytes = U8Bytes::new(data, metadata);
    ///
    /// assert_eq!(bytes.data.length, 3);
    /// assert_eq!(bytes.metadata.content_type, "text/plain");
    /// ```
    pub fn new(data: U8BytesData, metadata: U8Metadata) -> Self {
        Self { data, metadata }
    }
}

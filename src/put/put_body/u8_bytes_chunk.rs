use crate::put::put_body::u8_bytes_data::U8BytesData;
use thiserror::Error;

/// 分片范围构造错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum U8BytesChunkError {
    #[error("分片起止范围必须同时提供")]
    IncompleteRange,
    #[error("分片起点不能大于终点")]
    StartAfterEnd,
    #[error("分片范围长度为 {range_length}，实际数据长度为 {data_length}")]
    LengthMismatch { range_length: u64, data_length: u64 },
    #[error("分片范围长度溢出")]
    RangeOverflow,
    #[error("分片终点超出总长度")]
    EndExceedsTotal,
}

/// 单个业务分片：一段二进制数据 + 它在整份文件中的位置。
///
/// 字段全部公开，调用方可以自由设置或改写。
///
/// # 长度与字节同源
///
/// 数据内嵌 [`U8BytesData`]，因此分片长度与分片字节是同一个值里的两份信息，
/// 本类型不提供"传入分片长度"的入口。
///
/// [`new`](Self::new) 会在构造点校验范围与数据长度是否自洽；直接给公开字段赋值
/// 则不做任何校验，是否自洽由调用方自行保证。
///
/// # 范围语义
///
/// `range_start`、`range_end` 为**闭区间**，`0..=1` 表示两个字节。
/// `total_length` 是调用方声称的整份文件字节总长。
///
/// 三者都是**调用方侧的本地信息**，不是 HTTP 语义：
///
/// - 本库不会默认把它们写成 `Content-Range` 请求头。标准 WebDAV 的 PUT 是
///   单次资源写入，服务端是否支持部分上传无法可靠探测——服务端忽略
///   `Content-Range` 时通常回 200/201/204，与"支持并成功"无法区分，会静默把
///   整份资源覆盖成这一个分片。范围只在调用方明确要求时才转成请求头，
///   用 [`to_content_range`](Self::to_content_range) 取值后自行设置。
/// - 本库也不维护分片集合、不负责顺序、重试、断点续传或服务端合并。
///   分几片、按什么顺序发，全部由调用方决定。
///
/// 追踪标识只保留在 [`U8BytesData::id`] 一处，本类型不再重复持有。
///
/// [`U8BytesData`]: crate::put::put_body::u8_bytes_data::U8BytesData
/// [`U8BytesData::id`]: crate::put::put_body::u8_bytes_data::U8BytesData
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct U8BytesChunk {
    pub data: U8BytesData,
    /// 创建时间，仅供调用方记录。
    pub create_time: Option<String>,
    /// 闭区间起点；与 `range_end` 同时提供或同时缺省。
    pub range_start: Option<u64>,
    /// 闭区间终点；与 `range_start` 同时提供或同时缺省。
    pub range_end: Option<u64>,
    /// 调用方声称的整份文件字节总长，用于校验终点不越过文件末尾。
    pub total_length: Option<u64>,
}

impl U8BytesChunk {
    /// 创建分片并校验范围；范围缺省时表示调用方只提供分片数据、不描述位置。
    ///
    /// 校验规则：起点与终点要么都有要么都没有；起点不得大于终点；范围长度必须
    /// 等于数据长度；提供总长时终点不得越过文件末尾。
    ///
    /// 不想校验时直接用结构体字面量赋值即可。
    ///
    /// ```
    /// use webdav_raw::{U8BytesChunk, U8BytesData};
    ///
    /// let data = U8BytesData::new(vec![1, 2], None).unwrap();
    /// let chunk = U8BytesChunk::new(data, Some(0), Some(1), Some(2), None).unwrap();
    /// assert_eq!(chunk.range_end, Some(1));
    /// assert_eq!(chunk.data.length, 2);
    /// assert_eq!(chunk.to_content_range().as_deref(), Some("bytes 0-1/2"));
    /// ```
    pub fn new(
        data: U8BytesData,
        range_start: Option<u64>,
        range_end: Option<u64>,
        total_length: Option<u64>,
        create_time: Option<String>,
    ) -> Result<Self, U8BytesChunkError> {
        match (range_start, range_end) {
            (Some(start), Some(end)) => {
                if start > end {
                    return Err(U8BytesChunkError::StartAfterEnd);
                }

                let range_length = end
                    .checked_sub(start)
                    .and_then(|length| length.checked_add(1))
                    .ok_or(U8BytesChunkError::RangeOverflow)?;

                if range_length != data.length {
                    return Err(U8BytesChunkError::LengthMismatch {
                        range_length,
                        data_length: data.length,
                    });
                }

                if total_length.is_some_and(|total| end >= total) {
                    return Err(U8BytesChunkError::EndExceedsTotal);
                }
            }
            (None, None) => {}
            _ => return Err(U8BytesChunkError::IncompleteRange),
        }

        Ok(Self {
            data,
            create_time,
            range_start,
            range_end,
            total_length,
        })
    }

    /// 把闭区间换算成 `Content-Range` 请求头的取值，例如 `bytes 0-1023/2048`。
    ///
    /// 这是本库唯一的闭区间到 HTTP 表示换算点。范围缺省时返回 `None`。
    ///
    /// **本库不会自动把它写进请求**：服务端是否支持部分上传无法可靠探测，
    /// 默认发送会让"服务端忽略该头并整份覆盖资源"看起来像成功。要发就由调用方
    /// 显式设置：
    ///
    /// ```
    /// use webdav_raw::{U8BytesChunk, U8BytesData};
    ///
    /// let data = U8BytesData::new(vec![0; 4], None).unwrap();
    /// let chunk = U8BytesChunk::new(data, Some(0), Some(3), Some(8), None).unwrap();
    ///
    /// // 调用方明确要求时才转成请求头：
    /// // builder.header("content-range", chunk.to_content_range().unwrap())?
    /// assert_eq!(chunk.to_content_range().as_deref(), Some("bytes 0-3/8"));
    /// ```
    pub fn to_content_range(&self) -> Option<String> {
        match (self.range_start, self.range_end, self.total_length) {
            (Some(start), Some(end), Some(total)) => Some(format!("bytes {start}-{end}/{total}")),
            (Some(start), Some(end), None) => Some(format!("bytes {start}-{end}")),
            _ => None,
        }
    }
}

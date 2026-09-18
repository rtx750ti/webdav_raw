use thiserror::Error;

/// 调用方的追踪标识。
///
/// 只用于调用方在自己的流程里检索这一次数据，**不进入任何 HTTP 请求头**。
pub type BytesDataId = Option<String>;

/// 二进制数据构造错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum U8BytesDataError {
    #[error("二进制数据长度超出 u64 范围")]
    LengthOverflow,
}

/// 二进制数据及其字节长度。
///
/// # 长度：唯一信任源
///
/// `length` 由 [`new`](Self::new) 从数据本身算出，构造之后调用方仍可通过公开
/// 字段直接改写。**本类型不提供"传入长度"的构造入口**，因此不存在"声明的长度
/// 与真实字节不一致"这种需要校验的错误。
///
/// 直接改 `length` 由调用方自己负责：本库按 `length` 写 `Content-Length`，
/// 写错会让服务端按错误的长度读取请求体。
///
/// # 字节本体：不可随意读改
///
/// `data` **不公开**，这是有意的：
///
/// - **防泄漏**：上传内容可能是敏感文件，不应被其它代码顺手读走或复制一份。
/// - **防出错**：一旦允许随意改动字节，`length` 与实际字节就会脱节，而本库无法
///   再发现这种脱节。
///
/// 需要取出字节时走 [`into_data`](Self::into_data)（消费式，交出所有权）或
/// [`take`](Self::take)（清空并交出）。需要知道大小时用 [`len`](Self::len)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct U8BytesData {
    data: Vec<u8>,
    /// 由 `data.len()` 算出，不使用 `Option`：长度在构造时总是已知。
    pub length: u64,
    /// 调用方追踪标识，给调用方好搜索；不写进请求头。
    pub id: BytesDataId,
}

impl U8BytesData {
    /// 根据实际字节数据创建，长度由数据本身算出。
    ///
    /// ```
    /// use webdav_raw::U8BytesData;
    ///
    /// let data = U8BytesData::new(vec![1, 2, 3], Some("payload".to_owned())).unwrap();
    /// assert_eq!(data.length, 3);
    /// assert_eq!(data.len(), 3);
    /// assert_eq!(data.id.as_deref(), Some("payload"));
    ///
    /// // 取字节必须交出所有权。
    /// assert_eq!(data.into_data(), vec![1, 2, 3]);
    /// ```
    pub fn new(data: Vec<u8>, id: BytesDataId) -> Result<Self, U8BytesDataError> {
        let length = u64::try_from(data.len()).map_err(|_| U8BytesDataError::LengthOverflow)?;

        Ok(Self { data, length, id })
    }

    /// 获取字节长度，不暴露字节内容。
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 是否为空数据。
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 消费对象并交出字节。
    ///
    /// 这是取出字节的常规方式：交出所有权后本对象不再存在，不存在"复制出去一份
    /// 而原对象还在"的泄漏路径。
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    /// 取出字节并清空本对象，同时把 `length` 归零。
    ///
    /// `length` 归零是为了保持"长度与字节同源"：本对象之后的行为与新构造的空
    /// 数据完全一致。
    pub fn take(&mut self) -> Vec<u8> {
        self.length = 0;

        std::mem::take(&mut self.data)
    }
}

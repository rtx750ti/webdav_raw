use serde::{Deserialize, Serialize};

/// `<resourcetype>` 节点
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ResourceType {
    /// `<collection/>` 存在表示是目录，否则是文件
    ///
    /// `default` 是必需的：`propname` 查询按定义只回属性名、不回属性值，服务端因此
    /// 返回空元素 `<resourcetype/>`，而不是 `<resourcetype><collection/></resourcetype>`
    /// 或 `<resourcetype></resourcetype>`。没有默认值时，这种完全合法的响应会让整份
    /// multistatus 反序列化失败。
    #[serde(
        rename = "collection",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub is_collection: Option<EmptyElement>,
}

/// 空元素的占位结构，例如 `<collection/>`
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmptyElement {}

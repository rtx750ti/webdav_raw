use serde::{Deserialize, Serialize};

/// `<resourcetype>` 节点
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ResourceType {
    /// `<collection/>` 存在表示是目录，否则是文件
    #[serde(rename = "collection")]
    pub is_collection: Option<EmptyElement>,
}

/// 空元素的占位结构，例如 `<collection/>`
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmptyElement {}
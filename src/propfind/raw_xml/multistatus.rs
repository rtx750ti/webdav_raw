use crate::propfind::raw_xml::response::Response;
use quick_xml::DeError;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// 资源目录顶层节点
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct MultiStatus {
    #[serde(rename = "response", default)]
    pub response: VecDeque<Response>,
}

impl MultiStatus {
    pub fn from_str(raw_xml: &str) -> Result<Self, DeError> {
        let multistatus: MultiStatus = quick_xml::de::from_str(&raw_xml)?;
        Ok(multistatus)
    }

    /// 将当前对象序列化为 XML 字符串
    pub fn serialize(&self) -> Result<String, quick_xml::SeError> {
        quick_xml::se::to_string(&self)
    }
}

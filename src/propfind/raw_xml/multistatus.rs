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

#[cfg(test)]
mod test_multistatus {
    use crate::propfind::raw_xml::multistatus::MultiStatus;

    /// 获取当前案例原始xml内容
    fn get_example_raw_xml() -> String {
        use std::env::current_dir;
        let current_path = current_dir().unwrap();
        let example_file = current_path
            .join("test_files")
            .join("propfind")
            .join("raw_result1.xml");
        eprintln!("路径：{}", example_file.to_path_buf().to_string_lossy());
        let raw_xml_string = std::fs::read_to_string(example_file).unwrap();
        eprintln!("原始内容：{}", raw_xml_string);
        raw_xml_string
    }

    #[test]
    fn test_deserialize() {
        let a = get_example_raw_xml();
        let multistatus = MultiStatus::from_str(&a);

        eprintln!("解析结果：\n{:#?}", multistatus);
    }

    #[test]
    fn test_serialize() {
        let a = get_example_raw_xml();
        let multistatus = MultiStatus::from_str(&a).unwrap();

        let xml = multistatus.serialize().unwrap();
        eprintln!("序列化结果：\n{}", xml);

        // 可选：验证序列化结果能再次被解析
        let roundtrip = MultiStatus::from_str(&xml).unwrap();
        eprintln!("重新解析结果：\n{:#?}", roundtrip);
    }
}

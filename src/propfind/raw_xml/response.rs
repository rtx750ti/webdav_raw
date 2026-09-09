use percent_encoding::{NON_ALPHANUMERIC, percent_decode, utf8_percent_encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::propfind::raw_xml::propstat::PropStat;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Response {
    #[serde(
        deserialize_with = "deserialize_href",
        serialize_with = "serialize_href",
        default
    )]
    pub href: String,
    #[serde(rename = "propstat", default)]
    pub propstat: Vec<PropStat>,
}

/// 从字符串反序列化为 Url
fn deserialize_href<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    percent_decode(&s.as_bytes())
        .decode_utf8()
        .map(|cow| cow.into_owned())
        .map_err(serde::de::Error::custom)
}

/// 将 Url 序列化为字符串
fn serialize_href<S>(href: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let encoded = utf8_percent_encode(href, NON_ALPHANUMERIC).to_string();
    serializer.serialize_str(&encoded)
}

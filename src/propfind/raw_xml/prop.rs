use chrono::{DateTime, FixedOffset};
use mime::Mime;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::propfind::raw_xml::resourcetype::ResourceType;

/// 对应 `<D:prop>` 节点，列出资源的所有属性
///
/// `Default` 是必需的：`PropStat::prop` 用 `#[serde(default)]`，服务端省略 `<prop>`
/// 时需要构造一个「所有属性都没有」的空 `Prop`。
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Prop {
    /// `<resourcetype>`：资源类型（文件/目录）
    #[serde(rename = "resourcetype")]
    pub resource_type: Option<ResourceType>,

    /// `<getcontentlength>`：文件大小（字节），目录一般没有此字段
    #[serde(
        rename = "getcontentlength",
        deserialize_with = "de_optional_u64",
        default
    )]
    pub content_length: Option<u64>,

    /// `<getlastmodified>`：最后修改时间（HTTP-date 格式）
    #[serde(
        rename = "getlastmodified",
        deserialize_with = "de_http_date",
        serialize_with = "se_http_date",
        default
    )]
    pub last_modified: Option<DateTime<FixedOffset>>,

    /// `<getcontenttype>`：MIME 类型（如 "text/plain" 或 "application/pdf"）
    #[serde(
        rename = "getcontenttype",
        deserialize_with = "de_content_type",
        serialize_with = "se_content_type",
        default
    )]
    pub content_type: Option<Mime>,

    /// `<creationdate>`：资源创建时间（ISO8601，通常以 Z 结尾表示 UTC）
    #[serde(
        rename = "creationdate",
        deserialize_with = "de_non_empty_string",
        default
    )]
    pub creation_date: Option<String>,

    /// `<getetag>`：实体标签（文件内容的标识符，可用于缓存或变更检测）
    #[serde(rename = "getetag", deserialize_with = "de_non_empty_string", default)]
    pub etag: Option<String>,

    /// `<displayname>`：显示名（用户友好的文件/目录名）
    #[serde(
        rename = "displayname",
        deserialize_with = "de_non_empty_string",
        default
    )]
    pub display_name: Option<String>,

    /// `<owner>`：资源所有者（例如邮箱账号）
    #[serde(deserialize_with = "de_non_empty_string", default)]
    pub owner: Option<String>,
    // current-user-privilege-set属性可按需设计，暂时不用
}

/// 反序列化：空值或空串都归一成 `None`。
///
/// 服务端在 `propname` 响应里只给属性名，属性值写成空元素（`<getetag/>`）。这种
/// 情况下拿到空字符串是误导：调用方会以为存在一个空 ETag 或空的创建时间。
/// 统一归一成 `None`，让「有这个属性名但没给值」与「没有这个属性」在模型里同义。
fn de_non_empty_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;

    Ok(s.filter(|value| !value.trim().is_empty()))
}

/// 反序列化：仅解析 RFC 2822 格式（与原始 XML 一致）
fn de_http_date<'de, D>(deserializer: D) -> Result<Option<DateTime<FixedOffset>>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        // 空值等价于「没有这个属性」：`propname` 只回属性名，服务端会把每个属性都
        // 写成空元素（`<lp1:getlastmodified/>`），此时解析不出日期是正常的，
        // 不能当成格式错误。
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => DateTime::parse_from_rfc2822(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

/// 序列化：输出 RFC 2822 格式，保持与输入一致
fn se_http_date<S>(date: &Option<DateTime<FixedOffset>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match date {
        Some(dt) => serializer.serialize_str(&dt.to_rfc2822()),
        None => serializer.serialize_none(),
    }
}

// 反序列化Mime类型
fn de_content_type<'de, D>(deserializer: D) -> Result<Option<Mime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) => match s.parse::<Mime>() {
            Ok(mime) => Ok(Some(mime)),
            Err(_) => Ok(None),
        },
        None => Ok(None),
    }
}

// 序列化Mime类型
fn se_content_type<S>(value: &Option<Mime>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(mime) => serializer.serialize_str(mime.as_ref()), // Mime 实现了 AsRef<str>
        None => serializer.serialize_none(),
    }
}

/// 处理空字符串或缺失值，返回 Option<u64>
fn de_optional_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => s.parse::<u64>().map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

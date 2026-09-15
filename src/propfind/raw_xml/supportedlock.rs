use serde::{Deserialize, Serialize};

use crate::propfind::raw_xml::resourcetype::EmptyElement;

/// `<supportedlock>` 节点：这个资源支持哪些锁。
///
/// RFC 4918 把它写成一串 `lockentry`，每条声明一个「范围 + 类型」的组合。Apache 与
/// 大多数实现给两条：独占写锁与共享写锁。
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct SupportedLock {
    /// `<lockentry>` 列表。
    ///
    /// `default` 是必需的：属性名查询（`propname`）按定义只回属性名，服务端会写成
    /// 空元素 `<supportedlock/>`，没有默认值整份响应会解析失败。
    #[serde(rename = "lockentry", default)]
    pub lock_entry: Vec<LockEntry>,
}

/// 一条锁声明。
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct LockEntry {
    /// `<lockscope>`：独占还是共享。
    ///
    /// 标签名要显式写：`kebab-case` 会把 `lock_scope` 变成 `lock-scope`，与线上的
    /// `lockscope` 对不上。
    #[serde(rename = "lockscope", default)]
    pub lock_scope: LockScope,

    /// `<locktype>`：锁的类型，RFC 4918 只定义了 `write`。
    #[serde(rename = "locktype", default)]
    pub lock_type: LockType,
}

/// `<lockscope>`：锁的范围。
///
/// 服务端写了别的范围时两个字段都是 `None`——不硬塞进任一边。
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct LockScope {
    /// `<exclusive/>`：独占。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusive: Option<EmptyElement>,

    /// `<shared/>`：共享。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<EmptyElement>,
}

/// `<locktype>`：锁的类型。
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct LockType {
    /// `<write/>`：写锁。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write: Option<EmptyElement>,
}

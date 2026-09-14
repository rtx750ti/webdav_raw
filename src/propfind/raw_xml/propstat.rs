use serde::{Deserialize, Serialize};

use crate::propfind::raw_xml::prop::Prop;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct PropStat {
    /// `<prop>` 节点。
    ///
    /// `default` 是必需的：部分失败时服务端可能省略 `<prop>` 只给 `<status>`，
    /// 例如 COPY/MOVE 的 207 里每个失败项写成
    /// `<propstat><status>HTTP/1.1 423 Locked</status></propstat>`。
    /// 没有默认值时整份响应会解析失败，调用方连 `href` 与状态码都拿不到。
    #[serde(default)]
    pub prop: Prop,
    pub status: String,
}

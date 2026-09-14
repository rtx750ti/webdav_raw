//! `DeleteDepth` 的取值边界。

use webdav_core::DeleteDepth;

/// 未设置时默认递归删除。
#[test]
fn default_depth_is_infinity() {
    assert_eq!(DeleteDepth::default(), DeleteDepth::Infinity);
    assert_eq!(DeleteDepth::default().as_str(), "infinity");
}

/// 两个合法取值各自映射到 RFC 4918 规定的字符串。
#[test]
fn depth_values_map_to_rfc_strings() {
    assert_eq!(DeleteDepth::Zero.as_str(), "0");
    assert_eq!(DeleteDepth::Infinity.as_str(), "infinity");
}

//! `CopyDepth` 协议值的边界。
//!
//! COPY 与 MOVE 允许 `0`、`1`、`infinity` 三种取值（RFC 4918 §9.8.3），
//! 与 DELETE 只允许两种取值不同，因此这里有三个合法值要钉。

use webdav_raw::CopyDepth;

/// 未设置时默认递归复制。
#[test]
fn default_depth_is_infinity() {
    assert_eq!(CopyDepth::default(), CopyDepth::Infinity);
    assert_eq!(CopyDepth::default().as_str(), "infinity");
}

/// 三个合法取值各自映射到 RFC 规定的字符串。
#[test]
fn depth_values_map_to_rfc_strings() {
    assert_eq!(CopyDepth::Zero.as_str(), "0");
    assert_eq!(CopyDepth::One.as_str(), "1");
    assert_eq!(CopyDepth::Infinity.as_str(), "infinity");
}

/// 取值可以复制、比较。
#[test]
fn depth_is_copy_and_comparable() {
    let chosen = CopyDepth::One;
    let copied = chosen;

    assert_eq!(chosen, copied);
    assert_ne!(CopyDepth::Zero, CopyDepth::One);
    assert_ne!(CopyDepth::One, CopyDepth::Infinity);
}

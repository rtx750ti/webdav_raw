//! `MoveDepth` 协议值的边界。

use webdav_raw::MoveDepth;

/// 未设置时默认递归移动。
#[test]
fn default_depth_is_infinity() {
    assert_eq!(MoveDepth::default(), MoveDepth::Infinity);
    assert_eq!(MoveDepth::default().as_str(), "infinity");
}

/// 三个合法取值各自映射到 RFC 4918 §9.9.3 规定的字符串。
#[test]
fn depth_values_map_to_rfc_strings() {
    assert_eq!(MoveDepth::Zero.as_str(), "0");
    assert_eq!(MoveDepth::One.as_str(), "1");
    assert_eq!(MoveDepth::Infinity.as_str(), "infinity");
}

/// 取值可以复制、比较。
#[test]
fn depth_is_copy_and_comparable() {
    let chosen = MoveDepth::One;
    let copied = chosen;

    assert_eq!(chosen, copied);
    assert_ne!(MoveDepth::Zero, MoveDepth::One);
    assert_ne!(MoveDepth::One, MoveDepth::Infinity);
}

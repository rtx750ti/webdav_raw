//! `Overwrite` 协议值的边界。

use webdav_raw::Overwrite;

/// 默认值是按 RFC 默认处理的 `True`，也就是「不发送该请求头」。
#[test]
fn default_overwrite_is_true() {
    assert_eq!(Overwrite::default(), Overwrite::True);
    assert_eq!(Overwrite::default().as_str(), "T");
}

/// 两个取值各自映射到 RFC 4918 §10.6 规定的字符串。
#[test]
fn overwrite_values_map_to_rfc_strings() {
    assert_eq!(Overwrite::True.as_str(), "T");
    assert_eq!(Overwrite::False.as_str(), "F");
}

/// 取值可以复制、比较，用于调用方在装配前做判断。
#[test]
fn overwrite_is_copy_and_comparable() {
    let chosen = Overwrite::False;
    let copied = chosen;

    assert_eq!(chosen, copied);
    assert_ne!(Overwrite::True, Overwrite::False);
}

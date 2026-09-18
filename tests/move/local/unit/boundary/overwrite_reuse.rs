//! MOVE 复用的是 COPY 领域的 `Overwrite` 类型。
//!
//! MOVE 的语义是「COPY + 删源」，两者对「目标已存在时怎么办」的回答一致，
//! 因此共用一个协议值类型。这里把这个跨领域引用固定下来：如果将来 COPY 改了
//! `Overwrite` 的取值，MOVE 会一起受影响，这是有意的。

use webdav_raw::{MoveBuilder, Overwrite};

use crate::support::fixtures::{base_url, client};

/// `Overwrite` 仍然只有两个取值，映射到 RFC 的 `T` / `F`。
#[test]
fn overwrite_values_are_shared_with_copy() {
    assert_eq!(Overwrite::True.as_str(), "T");
    assert_eq!(Overwrite::False.as_str(), "F");
    assert_eq!(Overwrite::default(), Overwrite::True);
}

/// MOVE 的 `overwrite()` 接受 COPY 定义的类型。
#[test]
fn move_accepts_the_copy_overwrite_type() {
    let overwrite: Overwrite = Overwrite::False;

    let request = MoveBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .overwrite(overwrite)
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("overwrite").unwrap(), "F");
}

/// 默认 `Overwrite::True` 时不发该头，行为与 COPY 一致。
#[test]
fn default_omits_the_overwrite_header() {
    let request = MoveBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert!(request.headers().get("overwrite").is_none());
}

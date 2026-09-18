//! 目标地址到 `Destination` 请求头的转换。
//!
//! MOVE 与 COPY 在这条语义上完全一致，用例也逐条对应，便于两边对照维护。

use webdav_raw::{MoveBuilder, Url};

use crate::support::fixtures::{base_url, client};

/// 相对目标被补全成带 scheme 与 authority 的完整 URI。
#[test]
fn relative_target_becomes_absolute_uri() {
    let request = MoveBuilder::new(client(), base_url())
        .move_from_path("a.txt")
        .expect("合法相对路径应被接受")
        .target_path("b.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    let destination = request.headers().get("destination").unwrap();
    let parsed = Url::parse(destination.to_str().unwrap()).expect("Destination 应是完整 URI");

    assert_eq!(parsed.scheme(), "https");
    assert_eq!(parsed.host_str(), Some("example.com"));
    assert_eq!(parsed.path(), "/dav/b.txt");
}

/// 空格与中文被百分号编码，调用方不必自己编码。
#[test]
fn unicode_and_spaces_are_percent_encoded() {
    let request = MoveBuilder::new(client(), base_url())
        .move_from_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path("归档/报告 2026.pdf")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/dav/%E5%BD%92%E6%A1%A3/%E6%8A%A5%E5%91%8A%202026.pdf"
    );
}

/// 已经编码过的输入不会被二次编码。
#[test]
fn already_encoded_input_is_not_double_encoded() {
    let target = "%E5%BD%92%E6%A1%A3.txt";

    let request = MoveBuilder::new(client(), base_url())
        .move_from_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path(target)
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    let destination = request.headers().get("destination").unwrap().to_str().unwrap();
    assert!(
        !destination.contains("%25"),
        "不应出现二次编码的 %25，实际: {destination}"
    );
}

/// 调用方自己设置的 `Destination` 会被目标地址生成的值覆盖。
#[test]
fn caller_destination_header_is_overridden() {
    let request = MoveBuilder::new(client(), base_url())
        .move_from_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path("real-target.txt")
        .expect("合法相对路径应被接受")
        .header("destination", "https://evil.example.com/other.txt")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/dav/real-target.txt"
    );
    assert_eq!(request.headers().get_all("destination").iter().count(), 1);
}

/// 点路径与重复斜杠遵循 `Url` 的既有语义。
#[test]
fn target_normalizes_dot_segments_and_keeps_repeated_slashes() {
    let normalized = MoveBuilder::new(client(), base_url())
        .target_path("a/./b/../c.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");
    let repeated = MoveBuilder::new(client(), base_url())
        .target_path("a//b///c.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        normalized.headers().get("destination").unwrap(),
        "https://example.com/dav/a/c.txt"
    );
    assert_eq!(
        repeated.headers().get("destination").unwrap(),
        "https://example.com/dav/a//b///c.txt"
    );
}

/// COPY 与 MOVE 对同一个目标地址生成的 `Destination` 完全一致。
///
/// 两个领域的这段逻辑是分别实现的，这里用同一输入对照，防止两边漂移。
#[test]
fn destination_matches_copy_for_identical_input() {
    let target = "备份/报告 2026.pdf";

    let copied = webdav_raw::CopyBuilder::new(client(), base_url())
        .source_path("source.pdf")
        .expect("合法相对路径应被接受")
        .target_path(target)
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");
    let moved = MoveBuilder::new(client(), base_url())
        .move_from_path("source.pdf")
        .expect("合法相对路径应被接受")
        .target_path(target)
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        copied.headers().get("destination"),
        moved.headers().get("destination"),
        "COPY 与 MOVE 应生成相同的 Destination"
    );
}

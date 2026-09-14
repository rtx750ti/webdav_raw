//! 源与目标两个地址的边界：解析、编码与非法输入。

use webdav_core::{MoveBuilder, MoveError};

use crate::support::fixtures::{base_url, client};

/// 空源路径与空目标路径都落回认证根地址。
#[test]
fn empty_paths_keep_auth_root() {
    let request = MoveBuilder::new(client(), base_url())
        .source_path("")
        .expect("空相对路径应被接受")
        .target_path("")
        .expect("空相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/dav/");
    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/dav/"
    );
}

/// 源路径中的空格与中文被百分号编码。
#[test]
fn source_path_encodes_spaces_and_unicode() {
    let request = MoveBuilder::new(client(), base_url())
        .source_path("我的 文档/报告.pdf")
        .expect("合法相对路径应被接受")
        .target_path("archived.pdf")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.url().as_str(),
        "https://example.com/dav/%E6%88%91%E7%9A%84%20%E6%96%87%E6%A1%A3/%E6%8A%A5%E5%91%8A.pdf"
    );
}

/// 源与目标互相独立：设置一个不影响另一个。
#[test]
fn source_and_target_are_independent() {
    let request = MoveBuilder::new(client(), base_url())
        .source_path("old.txt")
        .expect("合法相对路径应被接受")
        .target_path("new.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/dav/old.txt");
    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/dav/new.txt"
    );
}

/// 后设置的源覆盖先设置的源，目标的设置不受影响。
#[test]
fn later_source_setting_overrides_previous_one() {
    let request = MoveBuilder::new(client(), base_url())
        .source_path("first.txt")
        .expect("合法相对路径应被接受")
        .source_url("https://example.com/second.txt")
        .expect("合法绝对地址应被接受")
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/second.txt");
    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/dav/target.txt"
    );
}

/// 后设置的目标覆盖先设置的目标，源的设置不受影响。
#[test]
fn later_target_setting_overrides_previous_one() {
    let request = MoveBuilder::new(client(), base_url())
        .source_path("source.txt")
        .expect("合法相对路径应被接受")
        .target_path("first-target.txt")
        .expect("合法相对路径应被接受")
        .target_url("https://example.com/second-target.txt")
        .expect("合法绝对地址应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/dav/source.txt");
    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/second-target.txt"
    );
}

/// `move_from_path` 与 `source_path` 完全等价。
#[test]
fn move_from_path_is_equivalent_to_source_path() {
    let via_source = MoveBuilder::new(client(), base_url())
        .source_path("a.txt")
        .expect("合法相对路径应被接受")
        .target_path("b.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");
    let via_move = MoveBuilder::new(client(), base_url())
        .move_from_path("a.txt")
        .expect("合法相对路径应被接受")
        .target_path("b.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(via_source.url(), via_move.url());
    assert_eq!(
        via_source.headers().get("destination"),
        via_move.headers().get("destination")
    );
    assert_eq!(via_source.method(), via_move.method());
}

/// 以斜杠开头的相对路径从主机根开始，覆盖认证根里已有的前缀。
#[test]
fn leading_slash_paths_replace_auth_root_prefix() {
    let request = MoveBuilder::new(client(), base_url())
        .source_path("/other/source.txt")
        .expect("合法相对路径应被接受")
        .target_path("/other/target.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/other/source.txt");
    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/other/target.txt"
    );
}

/// 源路径语法非法时报 `Url` 错误。
#[test]
fn malformed_source_returns_url_error() {
    let error = MoveBuilder::new(client(), base_url())
        .source_path("http://[::1")
        .expect_err("非法 URL 应被拒绝");

    assert!(matches!(error, MoveError::Url(_)));
}

/// 目标路径语法非法时报 `Url` 错误。
#[test]
fn malformed_target_returns_url_error() {
    let error = MoveBuilder::new(client(), base_url())
        .target_path("http://[::1")
        .expect_err("非法 URL 应被拒绝");

    assert!(matches!(error, MoveError::Url(_)));
}

/// `move_from_path` 同样拒绝语法非法的输入。
#[test]
fn malformed_move_from_path_returns_url_error() {
    let error = MoveBuilder::new(client(), base_url())
        .move_from_path("http://[::1")
        .expect_err("非法 URL 应被拒绝");

    assert!(matches!(error, MoveError::Url(_)));
}

/// `source_url` 与 `target_url` 同样拒绝语法非法的输入。
#[test]
fn malformed_absolute_urls_return_url_error() {
    let source_error = MoveBuilder::new(client(), base_url())
        .source_url("not a url")
        .expect_err("非法 URL 应被拒绝");
    let target_error = MoveBuilder::new(client(), base_url())
        .target_url("not a url")
        .expect_err("非法 URL 应被拒绝");

    assert!(matches!(source_error, MoveError::Url(_)));
    assert!(matches!(target_error, MoveError::Url(_)));
}

/// 未设置目标地址时构建失败，不发出请求。
#[test]
fn missing_target_returns_missing_target_error() {
    let error = MoveBuilder::new(client(), base_url())
        .source_path("source.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect_err("缺目标时应报错");

    assert!(matches!(error, MoveError::MissingTarget));
}

/// 只设置目标、不设置源也能构建：源落回认证根地址。
#[test]
fn missing_source_falls_back_to_auth_root() {
    let request = MoveBuilder::new(client(), base_url())
        .target_path("target.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.url().as_str(), "https://example.com/dav/");
}

/// 源与目标相同时本库不拦，照常构建（服务端可能回 403）。
#[test]
fn identical_source_and_target_are_not_blocked() {
    let request = MoveBuilder::new(client(), base_url())
        .move_from_path("same.txt")
        .expect("合法相对路径应被接受")
        .target_path("same.txt")
        .expect("合法相对路径应被接受")
        .build()
        .expect("本库不替服务端判定自移动");

    assert_eq!(
        request.headers().get("destination").unwrap(),
        "https://example.com/dav/same.txt"
    );
}

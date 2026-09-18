//! 请求体与内容类型之间的映射关系。
//!
//! MKCOL 的序列化不涉及 XML 回环（本库不解析 MKCOL 请求体），验证的是
//! 「body 的有无」到「请求语义」的转换：字节一致、`Content-Type` 只在需要时出现、
//! 调用方的显式设置优先。

use webdav_raw::{MkcolBuilder, HeaderMap, HeaderValue};

use crate::support::fixtures::{base_url, client};

/// 请求体字节与传入的字符串逐字节一致，包括非 ASCII。
#[test]
fn body_bytes_match_input_exactly() {
    let body = "<D:mkcol xmlns:D=\"DAV:\"><D:set><D:prop><D:displayname>目录</D:displayname>\
                </D:prop></D:set></D:mkcol>";

    let request = MkcolBuilder::new(client(), base_url())
        .body(body)
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.body().and_then(|body| body.as_bytes()),
        Some(body.as_bytes())
    );
}

/// 调用方显式设置的 `Content-Type` 优先于本库的默认 XML 类型。
#[test]
fn caller_content_type_wins_over_default() {
    let request = MkcolBuilder::new(client(), base_url())
        .header("content-type", "text/xml")
        .expect("合法请求头应被接受")
        .body("<D:mkcol xmlns:D=\"DAV:\"/>")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("content-type").unwrap(), "text/xml");
    assert_eq!(request.headers().get_all("content-type").iter().count(), 1);
}

/// 通过批量请求头设置的 `Content-Type` 同样优先。
#[test]
fn batch_content_type_wins_over_default() {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", HeaderValue::from_static("application/xml"));

    let request = MkcolBuilder::new(client(), base_url())
        .headers(headers)
        .body("<D:mkcol xmlns:D=\"DAV:\"/>")
        .build()
        .expect("请求应构建成功");

    assert_eq!(
        request.headers().get("content-type").unwrap(),
        "application/xml"
    );
}

/// 无 body 时调用方自己设的 `Content-Type` 仍然保留。
#[test]
fn caller_content_type_is_kept_without_body() {
    let request = MkcolBuilder::new(client(), base_url())
        .header("content-type", "text/plain")
        .expect("合法请求头应被接受")
        .build()
        .expect("请求应构建成功");

    assert_eq!(request.headers().get("content-type").unwrap(), "text/plain");
}

/// 本库不校验 body 是否为合法 XML：非法 XML 也能通过构建。
#[test]
fn malformed_xml_body_is_not_validated() {
    let request = MkcolBuilder::new(client(), base_url())
        .body("<not-closed")
        .build()
        .expect("构建阶段不校验 XML");

    assert_eq!(
        request.body().and_then(|body| body.as_bytes()),
        Some(&b"<not-closed"[..])
    );
}

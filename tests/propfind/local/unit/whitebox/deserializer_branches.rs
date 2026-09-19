//! 覆盖率补点：`src/propfind/raw_xml/prop.rs` 里自定义反序列化器的错误分支。
//!
//! `de_non_empty_string`、`de_http_date`、`de_content_type`、`de_optional_u64` 都以
//! `let s: Option<String> = Option::deserialize(deserializer)?;` 开头。既有测试只喂
//! 文本值和空元素，这四处 `?` 的 `Err` 传播区域从未命中（llvm-cov 报告中标红）。
//!
//! 命中条件是「属性值写成嵌套元素而不是文本」：quick-xml 交出 `Event::Start`，
//! `Option<String>` 反序列化失败，函数按错误上报，而不是猜一个默认值糊过去。

use webdav_raw::MultiStatus;

/// 把一条属性包进最小可解析的 207 响应体。
fn body_with_prop(prop_xml: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/dav/a.txt</D:href>
    <D:propstat>
      <D:prop>{prop_xml}</D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#
    )
}

/// `getetag` 的值写成嵌套元素时按反序列化错误上报。
#[test]
fn nested_getetag_value_returns_deserialization_error() {
    let body = body_with_prop("<D:getetag><D:child/></D:getetag>");

    assert!(MultiStatus::from_str(&body).is_err());
}

/// `getlastmodified` 的值写成嵌套元素时按反序列化错误上报。
#[test]
fn nested_getlastmodified_value_returns_deserialization_error() {
    let body = body_with_prop("<D:getlastmodified><D:child/></D:getlastmodified>");

    assert!(MultiStatus::from_str(&body).is_err());
}

/// `getcontenttype` 的值写成嵌套元素时按反序列化错误上报。
#[test]
fn nested_getcontenttype_value_returns_deserialization_error() {
    let body = body_with_prop("<D:getcontenttype><D:child/></D:getcontenttype>");

    assert!(MultiStatus::from_str(&body).is_err());
}

/// `getcontentlength` 的值写成嵌套元素时按反序列化错误上报。
#[test]
fn nested_getcontentlength_value_returns_deserialization_error() {
    let body = body_with_prop("<D:getcontentlength><D:child/></D:getcontentlength>");

    assert!(MultiStatus::from_str(&body).is_err());
}

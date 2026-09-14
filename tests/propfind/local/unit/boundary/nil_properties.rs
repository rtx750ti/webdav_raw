//! `xsi:nil` 形态的属性：显式声明「没有值」时必须解析成 `None`。
//!
//! RFC 4918 允许服务端用 `xsi:nil="true"` 表示某个属性这次没有值，这跟
//! `propname` 的空元素是两回事：空元素里至少有文本事件，`nil` 连文本都没有。
//! 解析层必须把两者都归一成 `None`，否则调用方会拿到一个「存在但无意义」的值。
//!
//! 这组用例同时覆盖**序列化**方向：模型里为 `None` 的属性再次序列化时不能崩，
//! 也不能凭空造出一个空字符串。

use webdav_core::MultiStatus;

/// 三个带 `xsi:nil` 的属性：时间、MIME、长度各覆盖一个反序列化分支。
const NIL_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<D:response xmlns:lp1="DAV:">
<D:href>/dav/file.txt</D:href>
<D:propstat>
<D:prop>
<lp1:getlastmodified xsi:nil="true"/>
<D:getcontenttype xsi:nil="true"/>
<lp1:getcontentlength xsi:nil="true"/>
</D:prop>
<D:status>HTTP/1.1 200 OK</D:status>
</D:propstat>
</D:response>
</D:multistatus>"#;

/// 取第一份 propstat 里的属性集合。
fn first_prop(multistatus: &MultiStatus) -> webdav_core::Prop {
    multistatus
        .response
        .front()
        .expect("应有一条 response")
        .propstat
        .first()
        .expect("应有一条 propstat")
        .prop
        .clone()
}

/// `xsi:nil="true"` 的属性应解析成 `None`，而不是空字符串或错误。
#[test]
fn nil_properties_are_parsed_as_none() {
    let multistatus = MultiStatus::from_str(NIL_BODY).expect("含 xsi:nil 的响应应能解析");
    let prop = first_prop(&multistatus);

    assert!(prop.last_modified.is_none(), "nil 的修改时间应为 None");
    assert!(prop.content_type.is_none(), "nil 的 MIME 类型应为 None");
    assert!(prop.content_length.is_none(), "nil 的长度应为 None");
}

/// 值为 `None` 的属性应能序列化，并且再次解析后仍是 `None`。
#[test]
fn nil_properties_survive_serialization_roundtrip() {
    let original = MultiStatus::from_str(NIL_BODY).expect("含 xsi:nil 的响应应能解析");
    let serialized = original.serialize().expect("含 None 属性的模型应能序列化");

    let roundtrip = MultiStatus::from_str(&serialized).expect("序列化结果应能再次解析");
    let prop = first_prop(&roundtrip);

    assert!(
        prop.last_modified.is_none(),
        "序列化不该把 None 变成空日期，实际: {:?}",
        prop.last_modified
    );
    assert!(
        prop.content_type.is_none(),
        "序列化不该把 None 变成空 MIME，实际: {:?}",
        prop.content_type
    );
    assert!(
        prop.content_length.is_none(),
        "序列化不该把 None 变成空长度，实际: {:?}",
        prop.content_length
    );
}

/// 属性整体缺失时同样应是 `None`（走 `default`，与 `nil` 结果一致）。
#[test]
fn missing_properties_are_also_none() {
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
<D:response>
<D:href>/dav/file.txt</D:href>
<D:propstat>
<D:prop><D:getetag>"abc"</D:getetag></D:prop>
<D:status>HTTP/1.1 200 OK</D:status>
</D:propstat>
</D:response>
</D:multistatus>"#;

    let multistatus = MultiStatus::from_str(body).expect("缺属性的响应应能解析");
    let prop = first_prop(&multistatus);

    assert_eq!(prop.etag.as_deref(), Some("\"abc\""), "给出的属性应解析");
    assert!(prop.last_modified.is_none(), "缺失的修改时间应为 None");
    assert!(prop.content_type.is_none(), "缺失的 MIME 类型应为 None");
    assert!(prop.content_length.is_none(), "缺失的长度应为 None");
}

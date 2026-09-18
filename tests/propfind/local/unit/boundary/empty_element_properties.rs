//! 空元素形态的属性：`propname` 响应必须能解析。
//!
//! `propname` 按定义只回属性名、不回属性值，因此服务端把每个属性都给成空元素，
//! 例如 `<lp1:resourcetype/>`。这类响应在结构上完全合法，解析必须成功。
//!
//! 这组用例先用固定的响应文本独立复现，不依赖真实服务器，便于定位与回归。

use webdav_raw::MultiStatus;

/// 真实服务器对 `propname` 返回的响应形态（节选自实际响应）。
const PROPNAME_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:" xmlns:ns0="DAV:">
<D:response xmlns:lp1="DAV:" xmlns:lp2="http://apache.org/dav/props/">
<D:href>/dav/</D:href>
<D:propstat>
<D:prop>
<lp1:resourcetype/>
<lp1:creationdate/>
<lp1:getlastmodified/>
<lp1:getetag/>
<D:supportedlock/>
<D:lockdiscovery/>
<D:getcontenttype/>
</D:prop>
<D:status>HTTP/1.1 200 OK</D:status>
</D:propstat>
</D:response>
</D:multistatus>"#;

/// 逐个属性单独出现时都必须能解析。
///
/// 这条用例来自一次真实排障：`propname` 响应整体解析失败，但一时看不出是哪个属性
/// 造成的。把属性拆开逐个试，才能定位到具体字段（当时是 `getlastmodified` 的空值被
/// 当成日期格式错误）。保留它作为回归：任何一个属性单独出现都不该让解析失败。
#[test]
fn each_empty_property_element_is_parseable_on_its_own() {
    let cases: [(&str, &str); 7] = [
        ("resourcetype", "<lp1:resourcetype/>"),
        ("creationdate", "<lp1:creationdate/>"),
        ("getlastmodified", "<lp1:getlastmodified/>"),
        ("getetag", "<lp1:getetag/>"),
        ("supportedlock", "<D:supportedlock/>"),
        ("lockdiscovery", "<D:lockdiscovery/>"),
        ("getcontenttype", "<D:getcontenttype/>"),
    ];

    for (name, element) in cases {
        let body = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:" xmlns:ns0="DAV:">
<D:response xmlns:lp1="DAV:" xmlns:lp2="http://apache.org/dav/props/">
<D:href>/dav/</D:href>
<D:propstat>
<D:prop>{element}</D:prop>
<D:status>HTTP/1.1 200 OK</D:status>
</D:propstat>
</D:response>
</D:multistatus>"#
        );

        assert!(
            MultiStatus::from_str(&body).is_ok(),
            "只有空元素 {name} 时也应能解析"
        );
    }
}

/// `propname` 的空元素响应应能解析，且属性值都为空。
#[test]
fn propname_empty_elements_are_parseable() {
    let multistatus =
        MultiStatus::from_str(PROPNAME_BODY).expect("propname 的空元素响应应能解析");

    let prop = &multistatus
        .response
        .front()
        .expect("应有一条 response")
        .propstat
        .first()
        .expect("应有一条 propstat")
        .prop;

    assert!(
        prop.resource_type
            .as_ref()
            .and_then(|resource_type| resource_type.is_collection.as_ref())
            .is_none(),
        "只回属性名时不应被判定成集合"
    );
    assert!(prop.content_length.is_none());
    assert!(prop.etag.is_none());
    assert!(prop.content_type.is_none());
    assert!(prop.last_modified.is_none());
    assert!(prop.creation_date.is_none());
    assert!(prop.display_name.is_none());
    assert!(prop.owner.is_none());
}

/// 集合的空元素 `<resourcetype><D:collection/></resourcetype>` 仍应解析成集合。
#[test]
fn collection_resourcetype_still_parses_as_collection() {
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
<D:response>
<D:href>/dav/dir/</D:href>
<D:propstat>
<D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop>
<D:status>HTTP/1.1 200 OK</D:status>
</D:propstat>
</D:response>
</D:multistatus>"#;

    let multistatus = MultiStatus::from_str(body).expect("集合响应应能解析");
    let prop = &multistatus
        .response
        .front()
        .expect("应有一条 response")
        .propstat
        .first()
        .expect("应有一条 propstat")
        .prop;
    let resource_type = prop.resource_type.as_ref().expect("应解析出资源类型");

    assert!(
        resource_type.is_collection.is_some(),
        "含 <collection/> 时应识别为集合"
    );
}

/// 文件的空 `<resourcetype/>`（作为属性值出现，不是 propname）应解析成空类型。
#[test]
fn empty_resourcetype_element_parses_as_no_type() {
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
<D:response>
<D:href>/dav/file.txt</D:href>
<D:propstat>
<D:prop><D:resourcetype/><D:getcontentlength>12</D:getcontentlength></D:prop>
<D:status>HTTP/1.1 200 OK</D:status>
</D:propstat>
</D:response>
</D:multistatus>"#;

    let multistatus = MultiStatus::from_str(body).expect("文件响应应能解析");
    let prop = &multistatus
        .response
        .front()
        .expect("应有一条 response")
        .propstat
        .first()
        .expect("应有一条 propstat")
        .prop;

    assert_eq!(prop.content_length, Some(12), "长度属性应能解析");
    assert!(
        prop.resource_type
            .as_ref()
            .and_then(|resource_type| resource_type.is_collection.as_ref())
            .is_none(),
        "空 <resourcetype/> 不应被判定成集合"
    );
}

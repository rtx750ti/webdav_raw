use std::collections::BTreeSet;

use webdav_core::PropFindBuilder;
use webdav_core::{FindProp, PropFindSelector};
use webdav_core::{Client, Url};

/// 从 Request 中读取构建器生成的 XML 请求体。
fn request_body(builder: PropFindBuilder) -> String {
    let request = builder.build().expect("请求应构建成功");
    let body = request.body().expect("请求应包含 XML 请求体");
    std::str::from_utf8(body.as_bytes().expect("请求体应是字节内容"))
        .expect("请求体应是 UTF-8")
        .to_owned()
}

/// 验证三种属性选择器序列化为预期的 PROPFIND XML。
#[test]
fn request_selectors_serialize_to_protocol_xml() {
    let base_url = Url::parse("https://example.com/dav/").expect("固定地址必须有效");
    let allprop = request_body(PropFindBuilder::new(Client::new(), base_url.clone()).allprop());
    let propname = request_body(PropFindBuilder::new(Client::new(), base_url.clone()).prop_name());
    let props = request_body(PropFindBuilder::new(Client::new(), base_url).selector(
        PropFindSelector::Props(BTreeSet::from([FindProp::Resourcetype, FindProp::Getetag])),
    ));

    assert_eq!(
        allprop,
        r#"<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#
    );
    assert_eq!(
        propname,
        r#"<D:propfind xmlns:D="DAV:"><D:propname/></D:propfind>"#
    );
    assert_eq!(
        props,
        r#"<D:propfind xmlns:D="DAV:"><D:prop><D:resourcetype/><D:getetag/></D:prop></D:propfind>"#
    );
}

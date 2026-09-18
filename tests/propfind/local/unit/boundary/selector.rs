use std::collections::BTreeSet;

use webdav_raw::{FindProp, PropFindSelector};

/// 验证 allprop、propname 和空属性集都生成各自的 XML 结构。
#[test]
fn selector_variants_generate_expected_xml() {
    let allprop = PropFindSelector::AllProp.to_xml().expect("XML 应生成成功");
    let propname = PropFindSelector::PropName.to_xml().expect("XML 应生成成功");
    let empty_props = PropFindSelector::Props(BTreeSet::new())
        .to_xml()
        .expect("XML 应生成成功");

    assert_eq!(
        allprop,
        r#"<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>"#
    );
    assert_eq!(
        propname,
        r#"<D:propfind xmlns:D="DAV:"><D:propname/></D:propfind>"#
    );
    assert_eq!(
        empty_props,
        r#"<D:propfind xmlns:D="DAV:"><D:prop></D:prop></D:propfind>"#
    );
}

/// 验证 `Getcontentlength` 在稳定顺序里排在最后（它是后补的变体）。
#[test]
fn content_length_follows_existing_properties_in_order() {
    let selector = PropFindSelector::Props(BTreeSet::from([
        FindProp::Getcontentlength,
        FindProp::Getcontenttype,
        FindProp::Resourcetype,
    ]));

    let xml = selector.to_xml().expect("XML 应生成成功");

    let resourcetype = xml.find("<D:resourcetype/>").expect("属性应存在");
    let contenttype = xml.find("<D:getcontenttype/>").expect("属性应存在");
    let contentlength = xml.find("<D:getcontentlength/>").expect("属性应存在");

    assert!(resourcetype < contenttype);
    assert!(contenttype < contentlength);
}

/// 验证指定属性自动去重并按稳定顺序输出。
#[test]
fn repeated_properties_are_deduplicated_in_stable_order() {
    let selector = PropFindSelector::Props(BTreeSet::from([
        FindProp::Getetag,
        FindProp::Getcontenttype,
        FindProp::Getetag,
        FindProp::Resourcetype,
        FindProp::Creationdate,
        FindProp::Getlastmodified,
    ]));

    let xml = selector.to_xml().expect("XML 应生成成功");

    assert_eq!(xml.matches("<D:getetag/>").count(), 1);
    assert!(
        xml.find("<D:resourcetype/>").expect("属性应存在")
            < xml.find("<D:creationdate/>").expect("属性应存在")
    );
    assert!(
        xml.find("<D:creationdate/>").expect("属性应存在")
            < xml.find("<D:getetag/>").expect("属性应存在")
    );
    assert!(
        xml.find("<D:getetag/>").expect("属性应存在")
            < xml.find("<D:getlastmodified/>").expect("属性应存在")
    );
    assert!(
        xml.find("<D:getlastmodified/>").expect("属性应存在")
            < xml.find("<D:getcontenttype/>").expect("属性应存在")
    );
}

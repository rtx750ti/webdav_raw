use webdav_core::propfind::raw_xml::multistatus::MultiStatus;

/// 第一份真实 PROPFIND XML 固件。
pub const RAW_RESULT_1: &str = include_str!("../../../test_files/propfind/raw_result1.xml");
/// 第二份真实 PROPFIND XML 固件。
pub const RAW_RESULT_2: &str = include_str!("../../../test_files/propfind/raw_result2.xml");

/// 将 PROPFIND XML 固件解析为完整响应列表。
pub fn parse_fixture(raw_xml: &str) -> MultiStatus {
    MultiStatus::from_str(raw_xml).expect("XML 固件必须可解析")
}

/// 提取回环比较所需的响应语义。
pub fn response_semantics(
    multistatus: &MultiStatus,
) -> Vec<(
    String,
    bool,
    Option<u64>,
    Option<String>,
    Option<String>,
    Option<String>,
)> {
    multistatus
        .response
        .iter()
        .map(|response| {
            let prop = response.propstat.first().map(|propstat| &propstat.prop);
            (
                response.href.clone(),
                prop.and_then(|prop| prop.resource_type.as_ref())
                    .and_then(|resource_type| resource_type.is_collection.as_ref())
                    .is_some(),
                prop.and_then(|prop| prop.content_length),
                prop.and_then(|prop| prop.last_modified.as_ref())
                    .map(|date| date.to_rfc2822()),
                prop.and_then(|prop| prop.content_type.as_ref())
                    .map(ToString::to_string),
                prop.and_then(|prop| prop.etag.clone()),
            )
        })
        .collect()
}

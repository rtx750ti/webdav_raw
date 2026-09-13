use webdav_core::MultiStatus;

use crate::support::fixtures::{RAW_RESULT_1, RAW_RESULT_2, parse_fixture};

/// 验证两份固件保留完整响应列表和主要 WebDAV 字段。
#[test]
fn fixtures_preserve_href_type_length_date_mime_etag_and_order() {
    let first = parse_fixture(RAW_RESULT_1);
    let second = parse_fixture(RAW_RESULT_2);
    let root = first.response.front().expect("第一份固件应包含根资源");
    let file = first.response.get(2).expect("第一份固件应包含文件资源");
    let decoded_folder = first.response.get(3).expect("第一份固件应包含中文目录");
    let file_prop = &file.propstat.first().expect("文件应有属性状态").prop;
    let root_prop = &root.propstat.first().expect("根资源应有属性状态").prop;

    assert_eq!(first.response.len(), 5);
    assert_eq!(second.response.len(), 11);
    assert_eq!(root.href, "/dav/");
    assert!(
        root_prop
            .resource_type
            .as_ref()
            .expect("根资源应有类型")
            .is_collection
            .is_some()
    );
    assert_eq!(file.href, "/dav/test.txt");
    assert!(file_prop.resource_type.is_some());
    assert!(
        file_prop
            .resource_type
            .as_ref()
            .expect("文件应有类型")
            .is_collection
            .is_none()
    );
    assert_eq!(file_prop.content_length, Some(12));
    assert_eq!(
        file_prop
            .content_type
            .as_ref()
            .expect("文件应有 MIME")
            .as_ref(),
        "text/plain"
    );
    assert_eq!(file_prop.etag.as_deref(), Some("\"c-63c79387773b2\""));
    assert!(file_prop.last_modified.is_some());
    assert_eq!(decoded_folder.href, "/dav/测试文件夹/");
}

/// 验证非法日期和非法内容长度会让 XML 反序列化返回错误。
#[test]
fn invalid_date_and_content_length_return_deserialization_error() {
    let invalid_date = r#"<multistatus><response><href>/dav/file</href><propstat><prop><getlastmodified>invalid</getlastmodified></prop><status>HTTP/1.1 200 OK</status></propstat></response></multistatus>"#;
    let invalid_length = r#"<multistatus><response><href>/dav/file</href><propstat><prop><getcontentlength>invalid</getcontentlength></prop><status>HTTP/1.1 200 OK</status></propstat></response></multistatus>"#;

    assert!(MultiStatus::from_str(invalid_date).is_err());
    assert!(MultiStatus::from_str(invalid_length).is_err());
}

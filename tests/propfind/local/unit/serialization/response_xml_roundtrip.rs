use webdav_raw::MultiStatus;

use crate::support::fixtures::{RAW_RESULT_1, RAW_RESULT_2, parse_fixture, response_semantics};

/// 验证两份真实 XML 固件经过序列化和再次解析后保持模型语义。
#[test]
fn response_xml_roundtrip_preserves_complete_response_semantics() {
    for raw_xml in [RAW_RESULT_1, RAW_RESULT_2] {
        let original = parse_fixture(raw_xml);
        let serialized = original.serialize().expect("模型应序列化成功");
        let roundtrip = MultiStatus::from_str(&serialized).expect("序列化 XML 应再次解析成功");

        assert_eq!(
            response_semantics(&roundtrip),
            response_semantics(&original)
        );
    }
}

use webdav_raw::Depth;

/// 验证三种 WebDAV Depth 值映射为协议请求头。
#[test]
fn each_depth_maps_to_protocol_header_value() {
    assert_eq!(Depth::Zero.as_str(), "0");
    assert_eq!(Depth::One.as_str(), "1");
    assert_eq!(Depth::Infinity.as_str(), "infinity");
}

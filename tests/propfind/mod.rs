#[path = "local/integration/allprop_contract.rs"]
mod allprop_contract;
#[path = "local/unit/whitebox/builder_paths.rs"]
mod builder_paths;
#[path = "local/unit/boundary/depth.rs"]
mod depth;
#[path = "support/fixtures.rs"]
mod fixtures;
#[path = "local/integration/properties_contract.rs"]
mod properties_contract;
#[path = "local/integration/propname_contract.rs"]
mod propname_contract;
#[path = "local/unit/boundary/request_path.rs"]
mod request_path;
#[path = "local/unit/serialization/request_xml.rs"]
mod request_xml;
#[path = "local/integration/response_207.rs"]
mod response_207;
#[path = "local/unit/boundary/response_fields.rs"]
mod response_fields;
#[path = "local/integration/response_status.rs"]
mod response_status;
#[path = "local/unit/serialization/response_xml_roundtrip.rs"]
mod response_xml_roundtrip;
#[path = "local/unit/boundary/selector.rs"]
mod selector;

#[cfg(feature = "network-test")]
#[path = "network/read_only_contract.rs"]
mod read_only_contract;

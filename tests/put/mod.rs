mod support;

#[path = "local/unit/boundary/data.rs"]
mod data;
#[path = "local/unit/whitebox/errors.rs"]
mod errors;
#[path = "local/unit/boundary/payload.rs"]
mod payload;
#[path = "local/unit/serialization/payload_conversion.rs"]
mod payload_conversion;
#[path = "local/integration/request_contract.rs"]
mod request_contract;
#[path = "local/integration/response_status.rs"]
mod response_status;
#[path = "local/integration/streamed_file.rs"]
mod streamed_file;
#[path = "local/unit/whitebox/builder_paths.rs"]
mod builder_paths;
#[path = "local/unit/whitebox/content_type.rs"]
mod content_type;

#[cfg(feature = "network-test")]
#[path = "network/real_upload.rs"]
mod real_upload;

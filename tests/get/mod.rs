mod support;

#[path = "local/unit/boundary/absolute_url.rs"]
mod absolute_url;
#[path = "local/unit/whitebox/builder_paths.rs"]
mod builder_paths;
#[path = "local/unit/boundary/relative_url.rs"]
mod relative_url;
#[path = "local/integration/request_contract.rs"]
mod request_contract;
#[path = "local/integration/response_status.rs"]
mod response_status;
#[path = "local/integration/response_success.rs"]
mod response_success;

#[cfg(feature = "network-test")]
#[path = "network/read_only_contract.rs"]
mod read_only_contract;

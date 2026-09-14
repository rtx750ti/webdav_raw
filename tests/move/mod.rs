//! MOVE 领域测试入口：只登记子模块，不放用例。
//!
//! 模块树与目录树一一对应，因此不需要任何 `#[path]`。

mod common;
mod local;
mod support;

#[cfg(feature = "network-test")]
mod network;

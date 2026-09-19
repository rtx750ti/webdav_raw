//! GET 领域的覆盖率补点测试。
//!
//! 这里只放必须经过完整 HTTP 链路才能命中的缺口；可用单测直接命中的缺口放在
//! `tests/get/local/unit/whitebox`。模块树与目录树一一对应，不使用 `#[path]`。

mod send_errors;

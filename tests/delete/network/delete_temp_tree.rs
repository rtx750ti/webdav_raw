//! 真实服务的 DELETE 验收（默认不运行）。
//!
//! # 红线
//!
//! 本用例**只删除自己创建的临时集合**，服务端既有内容一律不碰：
//!
//! - 临时集合名唯一（时间戳 + 进程号），不会撞到既有内容，也不会撞到上一次残留。
//! - 用例先创建它、确认它存在，删除后确认它消失；不存在任何对服务端原有资源的
//!   DELETE。
//! - 删除失败时的残留只打印名字，不做断言，也不掩盖失败。
//!
//! 运行方式：
//!
//! ```text
//! cargo test --features network-test --test delete -- --ignored --nocapture
//! ```
//!
//! 需要 `WEBDAV_URL`、`WEBDAV_ACCOUNT`、`WEBDAV_PASSWORD`。输出里不会出现账号、
//! 密码或 Authorization。

use std::time::{SystemTime, UNIX_EPOCH};

use webdav_core::{DeleteDepth, WebdavAuth};

use crate::common::network_config;

/// 本用例专属的临时命名空间名字。
fn temp_collection_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时间应晚于 Unix 纪元")
        .as_nanos();
    let pid = std::process::id();

    format!("__webdav_core_selftest_delete_{nanos}_{pid}/")
}

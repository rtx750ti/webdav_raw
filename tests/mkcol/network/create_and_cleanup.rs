//! 真实服务的 MKCOL 验收（默认不运行）。
//!
//! # 红线
//!
//! 本用例**只在自己创建的临时命名空间内建集合**，服务端既有内容一律不碰：
//!
//! - 命名空间名唯一（时间戳 + 进程号），不会撞到既有内容，也不会撞到上一次残留。
//! - 创建后用 PROPFIND 核对服务端自己的记录（`<resourcetype><collection/>`）。
//! - 用完后删除自己建的那一层。
//!
//! 运行方式：
//!
//! ```text
//! cargo test --features network-test --test mkcol -- --ignored --nocapture
//! ```
//!
//! 需要 `WEBDAV_URL`、`WEBDAV_ACCOUNT`、`WEBDAV_PASSWORD`。输出里不会出现账号、
//! 密码或 Authorization。

use std::time::{SystemTime, UNIX_EPOCH};

use webdav_core::{DeleteDepth, Depth, FindProp, WebdavAuth};

use crate::common::network_config;

/// 本用例专属的临时命名空间名字。
fn temp_collection_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时间应晚于 Unix 纪元")
        .as_nanos();
    let pid = std::process::id();

    format!("__webdav_core_selftest_mkcol_{nanos}_{pid}/")
}

/// 验证真实服务能创建集合，且 PROPFIND 能看到它，最后清理。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn created_collection_is_visible_then_cleaned_up() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let name = temp_collection_name();

    let created = auth
        .mkcol()
        .target_path(&name)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功");
    assert!(
        created.status().is_success(),
        "创建自己的临时集合应成功，实际: {}",
        created.status()
    );

    // 用服务端自己的记录核对：这一层确实是集合。
    //
    // 这里显式用 `Depth::Zero`：本项目的验收服务器（Teracloud）对根目录下的
    // `Depth: infinity` 递归查询返回 403，只接受 `0` 与 `1`。核对单个集合本身
    // 只需要 `0`，因此不受该策略影响。
    let listing = auth
        .propfind()
        .path(&name)
        .depth(Depth::Zero)
        .props([FindProp::Resourcetype])
        .send_and_deserialize()
        .await
        .expect("PROPFIND 应成功");
    assert_eq!(listing.response.len(), 1, "刚创建的集合应只有自身一项");
    let is_collection = listing
        .response
        .front()
        .and_then(|item| item.propstat.first())
        .and_then(|propstat| propstat.prop.resource_type.as_ref())
        .and_then(|resource_type| resource_type.is_collection.as_ref())
        .is_some();
    assert!(is_collection, "PROPFIND 应显示 resourcetype 为 collection");

    // 重复创建同一个集合应失败（服务端通常回 405）。
    let again = auth
        .mkcol()
        .target_path(&name)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功");
    assert!(
        !again.status().is_success(),
        "重复创建同一个集合应失败，实际: {}",
        again.status()
    );

    // 清理：只删自己刚建的那一层。
    //
    // 用 `DeleteDepth::Infinity`：本项目的验收服务器（Teracloud）对集合的
    // `Depth: 0` 删除返回 400，无限深度删除正常返回 204。
    let removed = auth
        .delete()
        .target_path(&name)
        .expect("相对路径应被接受")
        .depth(DeleteDepth::Infinity)
        .send()
        .await
        .expect("清理用的 DELETE 应发送成功");
    if !removed.status().is_success() {
        eprintln!("清理未成功，残留临时集合: {name}，状态码 {}", removed.status());
    }
}

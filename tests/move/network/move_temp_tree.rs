//! 真实服务的 MOVE 验收（默认不运行）。
//!
//! # 红线
//!
//! 本用例**只操作自己创建的临时文件**，服务端既有内容一律不碰：
//!
//! - 源文件由本用例用 PUT 现造。
//! - 目标是同目录下的新名字，不使用任何既有路径。
//! - 用 HEAD/PROPFIND 核对「源消失、目标出现」，再删除自己造的文件。
//! - 清理失败只打印残留名字，不做断言。
//!
//! 运行方式：
//!
//! ```text
//! cargo test --features network-test --test move -- --ignored --nocapture
//! ```
//!
//! 需要 `WEBDAV_URL`、`WEBDAV_ACCOUNT`、`WEBDAV_PASSWORD`。输出里不会出现账号、
//! 密码或 Authorization。

use std::time::{SystemTime, UNIX_EPOCH};

use webdav_core::{
    DeleteDepth, Overwrite, PutBody, U8Bytes, U8BytesData, U8Metadata, WebdavAuth,
};

use crate::common::network_config;

/// 本用例专属的临时名字前缀。
fn temp_prefix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时间应晚于 Unix 纪元")
        .as_nanos();
    let pid = std::process::id();

    format!("__webdav_core_selftest_move_{nanos}_{pid}")
}

/// 验证真实服务能移动自己造的文件：源消失、目标出现，内容一致，最后清理。
#[tokio::test]
#[ignore = "会向受控 WebDAV 服务写入临时文件，且只清理自己创建的内容"]
async fn moved_file_leaves_source_and_appears_at_target() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let prefix = temp_prefix();
    let source = format!("{prefix}-source.txt");
    let target = format!("{prefix}-target.txt");
    let content = format!("webdav-core move self-test {prefix}");

    // 1. 造源文件。
    let data = U8BytesData::new(content.as_bytes().to_vec(), None).expect("应构造成功");
    let metadata = U8Metadata::from_name("source.txt".to_owned()).expect("应构造成功");
    let uploaded = auth
        .put()
        .relative_path(&source)
        .expect("相对路径应被接受")
        .body(PutBody::from_bytes(U8Bytes::new(data, metadata)))
        .send()
        .await
        .expect("上传源文件应成功");
    assert!(uploaded.status().is_success());

    // 2. 移动。
    let moved = auth
        .mv()
        .move_from_path(&source)
        .expect("相对路径应被接受")
        .target_path(&target)
        .expect("相对路径应被接受")
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("MOVE 应发送成功");
    assert!(
        moved.status().is_success(),
        "移动自己造的文件应成功，实际: {}",
        moved.status()
    );

    // 3. 用 HEAD 核对：源消失、目标出现。
    let source_after = auth
        .head()
        .target_path(&source)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");
    assert!(
        !source_after.status().is_success(),
        "移动后源应查不到，实际: {}",
        source_after.status()
    );

    let target_after = auth
        .head()
        .target_path(&target)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");
    assert!(
        target_after.status().is_success(),
        "移动后目标应存在，实际: {}",
        target_after.status()
    );

    // 4. 内容应随资源一起搬过去。
    let fetched = auth
        .get()
        .relative_path(&target)
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("读取目标文件应成功");
    let fetched_body = fetched.text().await.expect("响应体应可读");
    assert_eq!(fetched_body, content, "移动后内容应与源一致");

    // 5. 清理：只删自己造的那个文件。
    // 用 `DeleteDepth::Infinity`：验收服务器对 `Depth: 0` 的 DELETE 会拒。
    let removed = auth
        .delete()
        .target_path(&target)
        .expect("相对路径应被接受")
        .depth(DeleteDepth::Infinity)
        .send()
        .await
        .expect("清理用的 DELETE 应发送成功");
    if !removed.status().is_success() {
        eprintln!("清理未成功，残留临时文件: {target}，状态码 {}", removed.status());
    }
}

/// 验证 `Overwrite::False` 对已存在的目标会失败，且不破坏既有目标。
#[tokio::test]
#[ignore = "会向受控 WebDAV 服务写入临时文件，且只清理自己创建的内容"]
async fn move_without_overwrite_refuses_existing_target() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let prefix = temp_prefix();
    let source = format!("{prefix}-source.txt");
    let target = format!("{prefix}-target.txt");
    let original = "existing target content, must survive";
    let incoming = "incoming content";

    for (path, body) in [(&source, incoming), (&target, original)] {
        let data = U8BytesData::new(body.as_bytes().to_vec(), None).expect("应构造成功");
        let metadata = U8Metadata::from_name("file.txt".to_owned()).expect("应构造成功");
        auth.put()
            .relative_path(path)
            .expect("相对路径应被接受")
            .body(PutBody::from_bytes(U8Bytes::new(data, metadata)))
            .send()
            .await
            .expect("上传应成功");
    }

    let moved = auth
        .mv()
        .move_from_path(&source)
        .expect("相对路径应被接受")
        .target_path(&target)
        .expect("相对路径应被接受")
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("MOVE 应发送成功");
    assert!(
        !moved.status().is_success(),
        "目标已存在且禁止覆盖时应失败，实际: {}",
        moved.status()
    );

    // 目标内容必须没变，源必须还在。
    let fetched = auth
        .get()
        .relative_path(&target)
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("读取目标文件应成功");
    assert_eq!(
        fetched.text().await.expect("响应体应可读"),
        original,
        "禁止覆盖时目标内容必须保持不变"
    );

    let source_after = auth
        .head()
        .target_path(&source)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");
    assert!(
        source_after.status().is_success(),
        "移动失败时源必须仍然存在"
    );

    for path in [&target, &source] {
        let removed = auth
            .delete()
            .target_path(path)
            .expect("相对路径应被接受")
            .depth(DeleteDepth::Infinity)
            .send()
            .await
            .expect("清理用的 DELETE 应发送成功");
        if !removed.status().is_success() {
            eprintln!("清理未成功，残留临时文件: {path}，状态码 {}", removed.status());
        }
    }
}

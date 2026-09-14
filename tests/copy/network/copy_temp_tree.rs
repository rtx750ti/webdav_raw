//! 真实服务的 COPY 验收（默认不运行）。
//!
//! # 红线
//!
//! 本用例**只操作自己创建的临时文件**，服务端既有内容一律不碰：
//!
//! - 源文件由本用例用 PUT 现造，内容是带时间戳的随机串。
//! - 目标是同目录下的新名字，不使用任何既有路径。
//! - 用 GET 逐字节核对复制结果，再删除两个自己造的文件。
//! - 清理失败只打印残留名字，不做断言。
//!
//! 运行方式：
//!
//! ```text
//! cargo test --features network-test --test copy -- --ignored --nocapture
//! ```
//!
//! 需要 `WEBDAV_URL`、`WEBDAV_ACCOUNT`、`WEBDAV_PASSWORD`。输出里不会出现账号、
//! 密码或 Authorization。

use std::time::{SystemTime, UNIX_EPOCH};

use webdav_core::{
    DeleteDepth, Overwrite, PutBody, U8Bytes, U8BytesData, U8Metadata, WebdavAuth,
};

use crate::common::network_config;

/// 本用例专属的临时目录名，所有操作都限制在它之内。
fn temp_prefix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时间应晚于 Unix 纪元")
        .as_nanos();
    let pid = std::process::id();

    format!("__webdav_core_selftest_copy_{nanos}_{pid}")
}

/// 验证真实服务能复制自己造的文件，字节一致，之后清理。
#[tokio::test]
#[ignore = "会向受控 WebDAV 服务写入临时文件，且只清理自己创建的内容"]
async fn copied_file_has_identical_bytes_then_cleaned_up() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let prefix = temp_prefix();
    let source = format!("{prefix}-source.txt");
    let target = format!("{prefix}-target.txt");
    let content = format!("webdav-core copy self-test {prefix}");

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
    assert!(
        uploaded.status().is_success(),
        "上传源文件应成功，实际: {}",
        uploaded.status()
    );

    // 2. 复制到目标。
    let copied = auth
        .copy()
        .source_path(&source)
        .expect("相对路径应被接受")
        .target_path(&target)
        .expect("相对路径应被接受")
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("COPY 应发送成功");
    assert!(
        copied.status().is_success(),
        "复制自己造的文件应成功，实际: {}",
        copied.status()
    );

    // 3. 用 GET 逐字节核对目标内容。
    let fetched = auth
        .get()
        .relative_url(target.clone())
        .send()
        .await
        .expect("读取目标文件应成功");
    assert_eq!(fetched.status(), 200);
    let fetched_body = fetched.text().await.expect("响应体应可读");
    assert_eq!(fetched_body, content, "复制出来的内容应与源一致");

    // 4. 源文件应仍然存在（COPY 不删源）。
    let source_after = auth
        .head()
        .target_path(&source)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");
    assert!(
        source_after.status().is_success(),
        "COPY 之后源文件应仍然存在，实际: {}",
        source_after.status()
    );

    // 5. 清理：只删自己造的两个文件。
    for path in [&target, &source] {
        let removed = auth
            .delete()
            .target_path(path)
            .expect("相对路径应被接受")
            .depth(DeleteDepth::Zero)
            .send()
            .await
            .expect("清理用的 DELETE 应发送成功");
        if !removed.status().is_success() {
            eprintln!("清理未成功，残留临时文件: {path}，状态码 {}", removed.status());
        }
    }
}

/// 验证 `Overwrite::False` 对已存在的目标会失败，且不覆盖既有内容。
#[tokio::test]
#[ignore = "会向受控 WebDAV 服务写入临时文件，且只清理自己创建的内容"]
async fn copy_without_overwrite_refuses_existing_target() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let prefix = temp_prefix();
    let source = format!("{prefix}-source.txt");
    let target = format!("{prefix}-target.txt");
    let original = "original content, must not be overwritten";
    let incoming = "incoming content";

    // 源与目标都由本用例现造，目标是先写好的「既有内容」。
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

    let copied = auth
        .copy()
        .source_path(&source)
        .expect("相对路径应被接受")
        .target_path(&target)
        .expect("相对路径应被接受")
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("COPY 应发送成功");
    assert!(
        !copied.status().is_success(),
        "目标已存在且禁止覆盖时应失败，实际: {}",
        copied.status()
    );

    // 核对目标内容没有被改动。
    let fetched = auth
        .get()
        .relative_url(target.clone())
        .send()
        .await
        .expect("读取目标文件应成功");
    let fetched_body = fetched.text().await.expect("响应体应可读");
    assert_eq!(fetched_body, original, "禁止覆盖时目标内容必须保持不变");

    for path in [&target, &source] {
        let removed = auth
            .delete()
            .target_path(path)
            .expect("相对路径应被接受")
            .depth(DeleteDepth::Zero)
            .send()
            .await
            .expect("清理用的 DELETE 应发送成功");
        if !removed.status().is_success() {
            eprintln!("清理未成功，残留临时文件: {path}，状态码 {}", removed.status());
        }
    }
}

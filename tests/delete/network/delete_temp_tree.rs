//! 真实服务的 DELETE 验收（默认不运行）。
//!
//! # 红线
//!
//! 本用例**只删除自己创建的临时资源**，服务端既有内容一律不碰：
//!
//! - 临时集合名唯一（时间戳 + 进程号），不会撞到既有内容，也不会撞到上一次残留。
//! - 用例先创建它、确认它存在，删除后确认它消失。
//! - 删除失败时的残留只打印名字，不做断言，也不掩盖失败。
//!
//! # 覆盖的两个边界
//!
//! 删除集合时分**空集合**与**非空集合**两种情况，服务端行为可能不同：
//! 非空集合只有递归删除才能整体删掉。用例分别验证结果。
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

use webdav_core::{
    DeleteDepth, FindProp, PutBody, U8Bytes, U8BytesData, U8Metadata, WebdavAuth,
};

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

/// 确认某个路径在服务端已经不存在。
///
/// 用 `Depth: Zero` 查询该路径本身：本项目的验收服务器（Teracloud）对根目录下的
/// `Depth: infinity` 会返回 403，而对单层路径的 `0` 正常返回 207 或 404。
async fn assert_absent(auth: &WebdavAuth, path: &str) {
    let result = auth
        .propfind()
        .path(path)
        .depth(webdav_core::Depth::Zero)
        .props([FindProp::Resourcetype])
        .send_and_deserialize()
        .await;

    assert!(
        result.is_err(),
        "删除后 {path} 不应还能被 PROPFIND 查到"
    );
}

/// 空文件夹：删掉之后服务端查不到它。
#[tokio::test]
#[ignore = "会向受控 WebDAV 服务创建并删除自己造的临时集合"]
async fn empty_collection_is_removed() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let name = temp_collection_name();

    // 建一个空集合。
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

    // 空集合删得掉。
    let removed = auth
        .delete()
        .target_path(&name)
        .expect("相对路径应被接受")
        .depth(DeleteDepth::Infinity)
        .send()
        .await
        .expect("DELETE 应发送成功");
    assert!(
        removed.status().is_success(),
        "删除空集合应成功，实际: {}",
        removed.status()
    );

    // 核对真的没了。
    assert_absent(&auth, &name).await;
}

/// 非空文件夹：连同成员一起删掉，成员也查不到。
#[tokio::test]
#[ignore = "会向受控 WebDAV 服务创建并删除自己造的临时集合"]
async fn non_empty_collection_is_removed_with_its_members() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let name = temp_collection_name();
    let member = format!("{name}inner.txt");

    // 建集合，并在里面放一个文件。
    let created = auth
        .mkcol()
        .target_path(&name)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功");
    assert!(created.status().is_success());

    let body = format!("webdav-core delete self-test {name}");
    let data = U8BytesData::new(body.as_bytes().to_vec(), None).expect("应构造成功");
    let metadata = U8Metadata::from_name("inner.txt".to_owned()).expect("应构造成功");
    let uploaded = auth
        .put()
        .relative_path(&member)
        .expect("相对路径应被接受")
        .body(PutBody::from_bytes(U8Bytes::new(data, metadata)))
        .send()
        .await
        .expect("上传成员文件应成功");
    assert!(uploaded.status().is_success());

    // 递归删除整个集合。
    let removed = auth
        .delete()
        .target_path(&name)
        .expect("相对路径应被接受")
        .depth(DeleteDepth::Infinity)
        .send()
        .await
        .expect("DELETE 应发送成功");
    assert!(
        removed.status().is_success(),
        "删除非空集合应成功，实际: {}",
        removed.status()
    );

    // 集合与成员都应查不到。
    assert_absent(&auth, &name).await;
    assert_absent(&auth, &member).await;
}

/// 单个文件：删掉之后服务端查不到它。
#[tokio::test]
#[ignore = "会向受控 WebDAV 服务创建并删除自己造的临时文件"]
async fn single_file_is_removed() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let file = format!(
        "__webdav_core_selftest_delete_file_{}_{}.txt",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("系统时间应晚于 Unix 纪元")
            .as_nanos(),
        std::process::id()
    );

    let data = U8BytesData::new(b"to be deleted".to_vec(), None).expect("应构造成功");
    let metadata = U8Metadata::from_name("doomed.txt".to_owned()).expect("应构造成功");
    let uploaded = auth
        .put()
        .relative_path(&file)
        .expect("相对路径应被接受")
        .body(PutBody::from_bytes(U8Bytes::new(data, metadata)))
        .send()
        .await
        .expect("上传应成功");
    assert!(uploaded.status().is_success());

    let removed = auth
        .delete()
        .target_path(&file)
        .expect("相对路径应被接受")
        .depth(DeleteDepth::Infinity)
        .send()
        .await
        .expect("DELETE 应发送成功");
    assert!(
        removed.status().is_success(),
        "删除文件应成功，实际: {}",
        removed.status()
    );

    assert_absent(&auth, &file).await;
}

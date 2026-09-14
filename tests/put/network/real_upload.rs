//! 真实 WebDAV 服务的 PUT 验收（默认不运行）。
//!
//! 按 `docs/测试/测试规范.md`，真实网络测试同时使用 `network-test` 功能开关与
//! `#[ignore]`，只在受控环境人工验收时执行。
//!
//! # 这个用例会真实写远端
//!
//! 它上传一个名字唯一的临时资源，读回来逐字节比对，用文件句柄覆盖同一个资源，
//! 再用本库既有的 PROPFIND 能力核对**服务端自己记录**的长度、类型与 ETag。
//! 验证的是「上传的字节真的落到了服务端」，而不只是「请求发得出去」——所以它比
//! 只断言状态码的写法更有意义，代价是必须真的写一次。
//!
//! **它不删除远端资源**：清理由人工负责。用例会把创建出来的资源名打印到输出，
//! 跑完照这个名字删即可。资源名带时间戳与进程号，不会和上一次的残留互相干扰。
//!
//! # 状态码为什么只断言 2xx
//!
//! 在本项目当前的验收服务器上观察到：新建返回 `201 Created`，覆盖返回
//! `204 No Content`。用例只断言 2xx，因为「新建还是覆盖用哪个码」由服务端实现
//! 决定，逐码断言会让这份验收换一台服务器就失效。状态码的**透传**行为已经由本地
//! MockServer 逐码固定（200/201/204 与 400/401/403/404/409/412/500 都有用例），
//! 这里不必重复；真实服务器要验的是 mock 验不了的那部分——字节到底有没有正确、
//! 完整地落到远端。
//!
//! # `Content-Type` 由服务端说了算
//!
//! 在本项目当前的验收服务器（Apache `mod_dav`）上观察到，同一次上传的资源：
//!
//! - **GET** 响应的 `Content-Type` 是 `text/plain`——按资源名的 `.txt` 扩展名算的。
//! - **PROPFIND** 的 `getcontenttype` 却是 `httpd/unix-directory`。这是 Apache 在
//!   拿不到类型时的占位值，不是集合标记：同一份响应里 `<resourcetype/>` 是空元素，
//!   长度也是文件长度（ETag 前段的十六进制就等于字节数）。该属性位于 `DAV:`
//!   （实时计算）命名空间，而不是 `http://apache.org/dav/props/`（客户端提交后存
//!   下来的死属性）命名空间。
//! - 上传时声明的 `Content-Type: application/x-custom` **两边都没有体现**。
//!
//! 结论：调用方声明的类型进不进服务端的属性、GET 时按什么回，都是服务端的自由，
//! 本库只能保证把它写进请求头（这一点由本地 MockServer 断言）。因此本用例**不断言**
//! `getcontenttype` 的值，只把观察打印出来；断言留给协议保证的部分——长度、是不是
//! 文件、ETag 是否随覆盖变化。要让服务端认对类型，把文件扩展名写对通常比声明
//! `Content-Type` 更有效。
//!
//! 运行方式：
//!
//! ```text
//! cargo test --features network-test --test put -- --ignored --nocapture
//! ```
//!
//! 需要 `WEBDAV_URL`、`WEBDAV_ACCOUNT`、`WEBDAV_PASSWORD`。输出里不会出现账号、
//! 密码或 Authorization。

use std::time::{SystemTime, UNIX_EPOCH};

use webdav_core::{
    Depth, FileHandle, MultiStatus, PutBody, StatusCode, U8Bytes, U8BytesData, U8Metadata,
    WebdavAuth,
};

use crate::common::network_config;

/// 内存源的上传内容。
///
/// 故意带上中文、空字节和高位字节：只测 ASCII 会漏掉编码与二进制透明性的问题。
fn memory_payload() -> Vec<u8> {
    let mut payload = "webdav-core PUT 验收：第一版内容".as_bytes().to_vec();
    payload.extend_from_slice(&[0x00, 0xFF, 0x0A, 0x80]);

    payload
}

/// 文件源的上传内容：更长，用于验证覆盖后长度确实跟着变。
fn file_payload() -> Vec<u8> {
    let mut payload = "webdav-core PUT 验收：第二版内容，覆盖前一版"
        .as_bytes()
        .to_vec();
    payload.extend(std::iter::repeat_n(0xA5_u8, 4096));

    payload
}

/// 生成带时间戳与进程号的唯一资源名。
///
/// 唯一是必须的：本用例不清理远端，用固定名字会和上一次的残留互相干扰，
/// 也无法判断读回来的内容究竟是这一次写的还是上一次留下的。
///
/// 后缀用 `.txt` 是为了让内容类型的观察有意义：`.txt` 是服务端一定能映射出来的
/// 类型，能对照出 GET 的类型是按扩展名算的、而 PROPFIND 的 `getcontenttype` 不是。
fn unique_resource_name() -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时间必须晚于 UNIX 纪元")
        .as_millis();

    format!(
        "__webdav_core_put_acceptance_{}_{stamp}.txt",
        std::process::id()
    )
}

/// 上传时由调用方声明的内容类型。
///
/// 故意选一个和 `.txt` 扩展名对不上的值：如果服务端的 `getcontenttype` 回的是
/// `text/plain`，说明它按文件名自己算类型、调用方声明的值不进它的属性；如果回
/// 的是这个值，说明服务端确实采纳了调用方的声明。两种结果都算验收通过——内容
/// 类型归谁定是服务端的自由，本用例只断言协议保证的部分，把观察打印出来。
const DECLARED_CONTENT_TYPE: &str = "application/x-custom";

/// 读回远端资源并返回字节内容，同时打印 GET 响应的 `Content-Type`。
///
/// 打印 GET 的 `Content-Type` 是为了回答一个实际问题：调用方声明的类型在这台
/// 服务端上到底有没有任何可观测效果。PROPFIND 的 `getcontenttype` 已经证明它是
/// 服务端现算的，这里再看 GET 响应头是否与它一致。
async fn read_back(auth: &WebdavAuth, resource: &str, step: &str) -> Vec<u8> {
    let response = auth
        .get()
        .relative_path(resource)
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("读回资源必须发送成功");

    println!("{step} GET 状态码: {}", response.status());
    println!(
        "{step} GET 响应 Content-Type: {:?}",
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
    );

    assert!(
        response.status().is_success(),
        "读回资源应返回成功状态，实际 {}",
        response.status()
    );

    response.bytes().await.expect("读取响应体必须成功").to_vec()
}

/// 服务端自己为这个资源记录下来的元数据。
struct RemoteMetadata {
    content_length: Option<u64>,
    content_type: Option<String>,
    etag: Option<String>,
    is_collection: bool,
}

impl RemoteMetadata {
    /// 便于把观察到的元数据打印出来。
    fn summary(&self) -> String {
        format!(
            "length={:?} type={:?} etag={:?} collection={}",
            self.content_length, self.content_type, self.etag, self.is_collection
        )
    }
}

/// 用本库既有的 PROPFIND 能力（只读）读服务端为该资源记录的元数据。
///
/// 这和 `read_back` 是两件事：GET 读回证明**发出去的字节**落对了，PROPFIND 证明
/// **服务端把它当成一个有正确长度的文件实体存了下来**——长度是服务端自己算的，
/// 不是我们请求头里声称的。两者都过，才算上传真的成立。
///
/// 这里用 `send()` + `MultiStatus::from_str()` 而不是 `send_and_deserialize()`，
/// 只是为了能把原始 XML 打出来：验收面向的是未知实现的服务端，元数据对不上时
/// 没有原始报文就只能猜。
async fn propfind_metadata(auth: &WebdavAuth, resource: &str) -> RemoteMetadata {
    let response = auth
        .propfind()
        .path(resource)
        .depth(Depth::Zero)
        .allprop()
        .send()
        .await
        .expect("PROPFIND 必须发送成功");

    println!("PROPFIND 状态码: {}", response.status());
    assert_eq!(
        response.status(),
        StatusCode::MULTI_STATUS,
        "PROPFIND 应返回 207 Multi-Status"
    );

    let raw = response.text().await.expect("读取 PROPFIND 响应体必须成功");
    println!("PROPFIND 原始报文: {raw}");

    let multistatus = MultiStatus::from_str(&raw).expect("PROPFIND 响应必须能解析成 MultiStatus");

    assert_eq!(
        multistatus.response.len(),
        1,
        "Depth:0 只应返回被查询的那一个资源"
    );

    let response = multistatus.response.front().expect("上一步已断言非空");

    assert!(
        response.href.ends_with(resource),
        "PROPFIND 返回的 href 必须指向本次查询的资源，实际 {}",
        response.href
    );

    let prop = &response
        .propstat
        .first()
        .expect("PROPFIND 响应必须带 propstat")
        .prop;

    RemoteMetadata {
        content_length: prop.content_length,
        content_type: prop.content_type.as_ref().map(|mime| mime.to_string()),
        etag: prop.etag.clone(),
        is_collection: prop
            .resource_type
            .as_ref()
            .is_some_and(|resource_type| resource_type.is_collection.is_some()),
    }
}

/// 内存源上传 → 读回比对 → 文件源覆盖 → 再读回比对，每一步都用 PROPFIND 核对
/// 服务端自己记录的元数据。
///
/// 一次验收只创建一个远端资源，跑完留给人工清理。
#[tokio::test]
#[ignore = "会向受控 WebDAV 服务写入一个真实资源，且不自动清理"]
async fn real_put_round_trip_uploads_and_overwrites_one_resource() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址必须有效");

    let resource = unique_resource_name();
    let memory_bytes = memory_payload();
    let file_bytes = file_payload();

    println!("远端资源名（跑完请手动删除）: {resource}");

    // ── 1. 内存源上传 ──
    // 调用方显式声明内容类型，覆盖掉 `.txt` 的推断结果，用于观察服务端是否采纳。
    let metadata = U8Metadata::from_name(resource.clone()).expect("内容类型推断必须成功");
    let data = U8BytesData::new(memory_bytes.clone(), Some("acceptance-memory".to_owned()))
        .expect("验收数据必须能构造成功");

    let response = auth
        .put()
        .relative_path(&resource)
        .expect("相对路径必须有效")
        .header("content-type", DECLARED_CONTENT_TYPE)
        .expect("合法请求头应被接受")
        .body(PutBody::from_bytes(U8Bytes::new(data, metadata)))
        .send()
        .await
        .expect("内存源 PUT 必须发送成功");

    println!("内存源 PUT 状态码: {}", response.status());
    assert!(
        response.status().is_success(),
        "内存源 PUT 应被服务端接受，实际 {}",
        response.status()
    );

    // ── 2. 读回逐字节比对：证明上传的字节真的落到了服务端 ──
    let stored = read_back(&auth, &resource, "内存源").await;
    assert_eq!(
        stored.len(),
        memory_bytes.len(),
        "服务端存下的长度必须等于上传的字节数"
    );
    assert_eq!(stored, memory_bytes, "服务端存下的字节必须与上传的完全一致");

    // ── 3. PROPFIND 核对服务端自己的记录 ──
    let remote = propfind_metadata(&auth, &resource).await;
    println!("PROPFIND 元数据: {}", remote.summary());
    println!("本次声明的 Content-Type: {DECLARED_CONTENT_TYPE}");

    assert!(!remote.is_collection, "上传出来的是一个文件，不是集合");
    assert_eq!(
        remote.content_length,
        Some(memory_bytes.len() as u64),
        "服务端记录的内容长度必须等于上传的字节数"
    );
    assert!(
        remote.content_type.is_some(),
        "服务端应为这个文件实体报告一个内容类型"
    );
    let first_etag = remote.etag.clone().expect("存成文件实体后必须有 ETag");

    // ── 4. 文件源覆盖同一个资源：验证流式发送与确定的 Content-Length ──
    let local_path = std::env::temp_dir().join(&resource);
    tokio::fs::write(&local_path, &file_bytes)
        .await
        .expect("本地临时文件必须可写");
    let file = tokio::fs::File::open(&local_path)
        .await
        .expect("本地临时文件必须可打开");

    let response = auth
        .put()
        .relative_path(&resource)
        .expect("相对路径必须有效")
        .header("content-type", DECLARED_CONTENT_TYPE)
        .expect("合法请求头应被接受")
        .body(PutBody::from_file(FileHandle::new(file, None)))
        .send()
        .await
        .expect("文件源 PUT 必须发送成功");

    println!("文件源覆盖 PUT 状态码: {}", response.status());
    assert!(
        response.status().is_success(),
        "文件源覆盖 PUT 应被服务端接受，实际 {}",
        response.status()
    );

    // ── 5. 再读回：长度与内容都必须是第二次上传的 ──
    let stored = read_back(&auth, &resource, "覆盖后").await;
    assert_eq!(
        stored.len(),
        file_bytes.len(),
        "覆盖后长度必须等于第二次上传的字节数"
    );
    assert_eq!(stored, file_bytes, "服务端存下的字节必须是第二次上传的内容");

    // ── 6. 再 PROPFIND：长度跟着变，且 ETag 必须变 ──
    let remote = propfind_metadata(&auth, &resource).await;
    println!("覆盖后 PROPFIND 元数据: {}", remote.summary());

    assert_eq!(
        remote.content_length,
        Some(file_bytes.len() as u64),
        "覆盖后服务端记录的长度必须等于第二次上传的字节数"
    );
    assert_ne!(
        remote.etag.as_deref(),
        Some(first_etag.as_str()),
        "内容变了，服务端记录的 ETag 必须跟着变，否则说明覆盖没有真的生效"
    );

    // 本地临时文件由本用例清理；远端资源留给人工。
    let _ = tokio::fs::remove_file(&local_path).await;

    println!("验收通过。远端资源 {resource} 仍存在，请手动删除。");
}

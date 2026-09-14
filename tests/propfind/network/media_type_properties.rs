//! PROPFIND 在真实文件（非目录）上的属性可见性。
//!
//! 这一组用例的存在理由：目录与文件在 WebDAV 里暴露的属性集合不同，而
//! `getcontentlength` 只对文件有意义。用真实文件核对，避免用目录的结论去推断文件。

use webdav_core::{Depth, WebdavAuth};

use crate::common::network_config;

/// 受控环境中已存在且只读的测试文件路径。
const EXISTING_FILE: &str = "测试文件夹/fast-sync.exe";

/// `allprop` 查询真实文件：应能看到 `getcontentlength`，并能解析出长度。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn allprop_on_file_exposes_content_length() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let multistatus = auth
        .propfind()
        .path(EXISTING_FILE)
        .depth(Depth::Zero)
        .allprop()
        .send_and_deserialize()
        .await
        .expect("文件 allprop 应成功");

    let prop = multistatus
        .response
        .front()
        .and_then(|item| item.propstat.first())
        .map(|propstat| propstat.prop.clone())
        .expect("应返回一条 propstat");

    assert!(
        prop.content_length.is_some_and(|length| length > 0),
        "真实文件应有正的 getcontentlength，实际: {:?}",
        prop.content_length
    );
    assert!(
        prop.etag.is_some(),
        "真实文件应有 getetag，实际: {:?}",
        prop.etag
    );
}

/// 查询真实文件：按名请求 `getetag` 能取到，但**取不到长度**。
///
/// 因为 `FindProp` 里没有 `getcontentlength` 这一项，按名请求根本无法把长度
/// 属性写进请求体。这条用例把该限制固定下来，避免以后有人以为按名请求能拿长度。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn named_props_on_file_cannot_request_content_length() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let multistatus = auth
        .propfind()
        .path(EXISTING_FILE)
        .depth(Depth::Zero)
        .props([
            webdav_core::FindProp::Resourcetype,
            webdav_core::FindProp::Getetag,
        ])
        .send_and_deserialize()
        .await
        .expect("按名请求应成功");

    let prop = multistatus
        .response
        .front()
        .and_then(|item| item.propstat.first())
        .map(|propstat| propstat.prop.clone())
        .expect("应返回一条 propstat");

    assert!(
        prop.etag.is_some(),
        "按名请求 getetag 应能取到，实际: {:?}",
        prop.etag
    );
    assert!(
        prop.content_length.is_none(),
        "按名请求没有请求长度属性，因此解析结果应为 None，实际: {:?}",
        prop.content_length
    );
}

/// 按名请求 `getcontentlength`：能取到长度，并与 `allprop` 的结果一致。
///
/// 这条用例依赖 `FindProp::Getcontentlength`（后补的变体）。在此之前按名请求
/// 无法索要长度属性，只能靠 `allprop` 顺带拿回。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn named_content_length_request_returns_length() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let named = auth
        .propfind()
        .path(EXISTING_FILE)
        .depth(Depth::Zero)
        .props([
            webdav_core::FindProp::Resourcetype,
            webdav_core::FindProp::Getcontentlength,
        ])
        .send_and_deserialize()
        .await
        .expect("按名请求真实文件应成功");

    let all = auth
        .propfind()
        .path(EXISTING_FILE)
        .depth(Depth::Zero)
        .allprop()
        .send_and_deserialize()
        .await
        .expect("allprop 请求真实文件应成功");

    let named_length = named
        .response
        .front()
        .and_then(|item| item.propstat.first())
        .and_then(|propstat| propstat.prop.content_length);
    let all_length = all
        .response
        .front()
        .and_then(|item| item.propstat.first())
        .and_then(|propstat| propstat.prop.content_length);

    assert!(
        named_length.is_some_and(|length| length > 0),
        "按名请求 getcontentlength 应取到正的长度，实际: {named_length:?}"
    );
    assert_eq!(
        named_length, all_length,
        "按名请求与 allprop 拿到的长度应一致"
    );
}

/// `propname` 查询真实文件：服务端回属性名清单，本库应能解析。
///
/// `propname` 按定义只回属性名、不回属性值，因此服务端把每个属性都给成空元素
/// （`<lp1:resourcetype/>` 这种形态）。这条用例固定在真实服务器上能解析成功。
///
/// 它曾经是失败的：`ResourceType::is_collection` 与 `PropStat::prop` 缺少
/// `#[serde(default)]`，空元素会让整份响应解析失败。补上默认值后已修好。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn propname_on_real_server_lists_property_names() {
    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    let multistatus = auth
        .propfind()
        .path(EXISTING_FILE)
        .depth(Depth::Zero)
        .prop_name()
        .send_and_deserialize()
        .await
        .expect("propname 响应是合法 multistatus，应能解析");

    assert!(
        !multistatus.response.is_empty(),
        "propname 应至少返回一条 response"
    );

    let prop = multistatus
        .response
        .front()
        .and_then(|item| item.propstat.first())
        .map(|propstat| propstat.prop.clone())
        .expect("应返回一条 propstat");

    // propname 不回属性值，因此解析结果里的属性值应全为空——这正是该选择器的语义。
    // 「有这个属性名但没给值」与「没有这个属性」在模型里同义，都应是 None。
    assert!(
        prop.resource_type
            .as_ref()
            .and_then(|resource_type| resource_type.is_collection.as_ref())
            .is_none()
            && prop.content_length.is_none()
            && prop.etag.is_none()
            && prop.content_type.is_none()
            && prop.last_modified.is_none()
            && prop.creation_date.is_none(),
        "propname 只回属性名，解析出的属性值应全为空，实际: {prop:?}"
    );
}

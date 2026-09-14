//! 真实服务器操作助手。
//!
//! 这里只放「怎么和服务器打交道」，不放断言。步骤函数负责断言，本模块负责取数。
//!
//! 所有写操作都限制在本用例自建的命名空间内，服务端既有内容一律不碰。

use std::collections::BTreeSet;

use webdav_core::{
    DeleteDepth, Depth, FindProp, MultiStatus, PutBody, StatusCode, U8Bytes, U8BytesData,
    U8Metadata, WebdavAuth,
};

/// 本方案创建的命名空间前缀。清理只认这个前缀，绝不碰其他内容。
pub const NAMESPACE_PREFIX: &str = "__webdav_core_workflow_";

/// 列出根目录下的直接成员，返回**相对于认证根地址**的路径集合。
///
/// 例如认证根是 `https://host/dav/`，服务端 href 是 `/dav/MyDocument/`，
/// 返回的就是 `MyDocument`；根自身会以 `dav` 出现（即 auth 根那一段）。
pub async fn list_root(auth: &WebdavAuth) -> BTreeSet<String> {
    let multistatus = auth
        .propfind()
        .depth(Depth::One)
        .send_and_deserialize()
        .await
        .expect("列根目录应成功");

    let base = normalize(auth.get_base_url().path());

    multistatus
        .response
        .iter()
        .map(|item| strip_suffix(&normalize(&item.href), &base))
        .filter(|path| !path.is_empty())
        .collect()
}

/// 按前缀列出根目录下匹配的顶层成员（前缀相对认证根）。
pub async fn list_root_with_prefix(auth: &WebdavAuth, prefix: &str) -> BTreeSet<String> {
    list_root(auth)
        .await
        .into_iter()
        .filter(|path| path.starts_with(prefix))
        .collect()
}

/// 列出某个集合的直接子项，返回**相对于该集合**的成员名集合。
///
/// 集合自身不在结果里。返回集合语义（`BTreeSet`），因为服务端不保证返回顺序。
///
/// `path` 是相对于认证根的路径（与 [`list_root`] 的返回口径一致）。
pub async fn list_children(auth: &WebdavAuth, path: &str) -> BTreeSet<String> {
    let multistatus = propfind_target(auth, path, Depth::One).await;

    // 服务端 href 带认证根的路径段（例如 `dav/`），而 `path` 是相对认证根的，
    // 因此比对前要先把这一层补回来，否则前缀永远匹配不上。
    let base = normalize(auth.get_base_url().path());
    let relative = normalize(path);
    let collection_suffix = if relative.is_empty() {
        base
    } else {
        format!("{base}/{relative}")
    };

    response_paths(&multistatus)
        .into_iter()
        .filter_map(|full| {
            // 集合自身：跳过。
            if full == collection_suffix {
                return None;
            }
            // 只保留直接子项：去掉集合前缀后不应再含 `/`。
            let relative = full
                .strip_prefix(&format!("{collection_suffix}/"))
                .or_else(|| full.strip_prefix(&collection_suffix))?;
            let relative = relative.trim_matches('/');
            if relative.is_empty() || relative.contains('/') {
                return None;
            }

            Some(relative.to_owned())
        })
        .collect()
}

/// 判断路径是否存在。
///
/// 手段是 `Depth: 0` 的 PROPFIND：不存在时服务端返回非 207，`send_and_deserialize`
/// 会报 `UnexpectedStatus`，据此判定「查不到」。
pub async fn exists(auth: &WebdavAuth, path: &str) -> bool {
    auth.propfind()
        .path(path)
        .depth(Depth::Zero)
        .send_and_deserialize()
        .await
        .is_ok()
}

/// 判断路径是否是集合（目录）。
pub async fn is_collection(auth: &WebdavAuth, path: &str) -> bool {
    let multistatus = propfind_target(auth, path, Depth::Zero).await;

    multistatus
        .response
        .front()
        .and_then(|item| item.propstat.first())
        .and_then(|propstat| propstat.prop.resource_type.as_ref())
        .and_then(|resource_type| resource_type.is_collection.as_ref())
        .is_some()
}

/// 读取某个资源的内容。
pub async fn read(auth: &WebdavAuth, path: &str) -> Vec<u8> {
    let response = auth
        .get()
        .relative_path(path)
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("GET 应发送成功");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "GET {path} 应返回 200，实际 {}",
        response.status()
    );

    response
        .bytes()
        .await
        .expect("响应体应可读")
        .to_vec()
}

/// 读取某个资源的 `Content-Length` 响应头。
pub async fn head_length(auth: &WebdavAuth, path: &str) -> Option<u64> {
    let response = auth
        .head()
        .target_path(path)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "HEAD {path} 应返回 200，实际 {}",
        response.status()
    );

    response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
}

/// 用内存源写入一个文件。
pub async fn put_bytes(auth: &WebdavAuth, path: &str, bytes: Vec<u8>) -> StatusCode {
    let data = U8BytesData::new(bytes, None).expect("测试数据应能构造成功");
    let metadata = U8Metadata::from_name("file.txt".to_owned()).expect("元数据应能构造成功");

    auth.put()
        .relative_path(path)
        .expect("相对路径应被接受")
        .body(PutBody::from_bytes(U8Bytes::new(data, metadata)))
        .send()
        .await
        .expect("PUT 应发送成功")
        .status()
}

/// 用文件句柄源写入一个文件。
///
/// 在系统临时目录现造文件（按测试规范，PUT 领域不依赖仓库自带的二进制夹具），
/// 发送完立即删除。
pub async fn put_file(auth: &WebdavAuth, path: &str, bytes: Vec<u8>) -> StatusCode {
    let temp = std::env::temp_dir().join(format!(
        "webdav_core_workflow_{}_{}.bin",
        std::process::id(),
        path.replace('/', "_")
    ));
    tokio::fs::write(&temp, &bytes)
        .await
        .expect("临时文件应可写入");

    // 句柄必须处于文件起始位置，正好满足 FileHandle 的要求。
    let file = tokio::fs::File::open(&temp)
        .await
        .expect("临时文件应可打开");
    let status = auth
        .put()
        .relative_path(path)
        .expect("相对路径应被接受")
        .body(PutBody::from_file(webdav_core::FileHandle::new(file, None)))
        .send()
        .await
        .expect("PUT 应发送成功")
        .status();

    let _ = tokio::fs::remove_file(&temp).await;

    status
}

/// 递归删除一个路径；返回状态码。
pub async fn delete_tree(auth: &WebdavAuth, path: &str) -> StatusCode {
    auth.delete()
        .target_path(path)
        .expect("相对路径应被接受")
        .depth(DeleteDepth::Infinity)
        .send()
        .await
        .expect("DELETE 应发送成功")
        .status()
}

/// 单层删除一个路径；返回状态码。
pub async fn delete_single(auth: &WebdavAuth, path: &str) -> StatusCode {
    auth.delete()
        .target_path(path)
        .expect("相对路径应被接受")
        .depth(DeleteDepth::Zero)
        .send()
        .await
        .expect("DELETE 应发送成功")
        .status()
}

/// 发一次 `Depth: 0` 的 PROPFIND 并返回解析结果。仅用于只读核对。
async fn propfind_target(auth: &WebdavAuth, path: &str, depth: Depth) -> MultiStatus {
    auth.propfind()
        .path(path)
        .depth(depth)
        .props([
            FindProp::Resourcetype,
            FindProp::Getcontenttype,
            FindProp::Getetag,
            FindProp::Getlastmodified,
        ])
        .send_and_deserialize()
        .await
        .unwrap_or_else(|error| panic!("PROPFIND {path} 应成功，实际: {error:?}"))
}

/// 从 `MultiStatus` 里取出所有 href，并规范化为不带首尾斜杠的路径。
fn response_paths(multistatus: &MultiStatus) -> Vec<String> {
    multistatus
        .response
        .iter()
        .map(|item| normalize(&item.href))
        .collect()
}

/// 去掉 `suffix` 前缀；两边都已规范化。不匹配时原样返回。
fn strip_suffix(full: &str, suffix: &str) -> String {
    if suffix.is_empty() {
        return full.to_owned();
    }

    full.strip_prefix(suffix)
        .map(|rest| rest.trim_start_matches('/').to_owned())
        .unwrap_or_else(|| full.to_owned())
}

/// 把 href 规范化为「不带首尾斜杠、已百分号解码」的形式，便于比较。
///
/// 服务端返回的 href 可能是完整 URL、也可能只有路径，且**一定**是百分号编码的
/// （中文目录名会变成 `%E6%B5%8B…`）。`PropFindBuilder` 在解析时会把 href 解码，
/// 因此这里也必须解码，否则中文名字对不上。
fn normalize(value: &str) -> String {
    let without_scheme = match value.split_once("://") {
        Some((_, rest)) => match rest.split_once('/') {
            Some((_, path)) => format!("/{path}"),
            None => String::from("/"),
        },
        None => value.to_owned(),
    };

    let decoded = percent_encoding::percent_decode(without_scheme.as_bytes())
        .decode_utf8()
        .map(|cow| cow.into_owned())
        .unwrap_or(without_scheme);

    decoded.trim_matches('/').to_owned()
}

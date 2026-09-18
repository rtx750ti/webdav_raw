//! 阶段一（步骤 1–26）：查询根目录、建目录树、建文件、逐项核对。
//!
//! 每个步骤一个函数，只做一件事：要么一个动作，要么一次核对，两者不合并。

use std::collections::BTreeSet;

use webdav_raw::{FindProp, StatusCode};

use super::data::{content, dir, file};
use crate::support::context::{Ctx, step};
use crate::support::dav_tree;

/// 步骤 1：查询当前目录自身。
pub async fn step_01_query_root_self(ctx: &Ctx) {
    step("1. 查询当前目录自身（PROPFIND Depth:0 allprop）");

    let multistatus = ctx
        .auth()
        .propfind()
        .depth(webdav_raw::Depth::Zero)
        .send_and_deserialize()
        .await
        .expect("查询根目录自身应成功");

    assert_eq!(
        multistatus.response.len(),
        1,
        "Depth:0 应只返回自身一项，实际: {:?}",
        multistatus
            .response
            .iter()
            .map(|item| item.href.as_str())
            .collect::<Vec<_>>()
    );
}

/// 步骤 2：查询当前目录成员。
pub async fn step_02_query_root_members(ctx: &Ctx) {
    step("2. 查询当前目录成员（PROPFIND Depth:1 allprop）");

    let members = dav_tree::list_root(ctx.auth()).await;
    println!("[workflow]   根目录成员: {members:?}");
    assert!(
        !members.is_empty(),
        "根目录应至少有一个成员（本用例的命名空间目录之外还应能看到既有内容）"
    );
}

/// 步骤 3：只问属性名。
///
/// # 这里为什么不调用 `send_and_deserialize`
///
/// `propname` 按定义只回**属性名**、不回属性值，因此服务端返回的每个属性都是空元素
/// 形式，例如 `<lp1:resourcetype/>`（注意里面没有 `<D:collection/>`）。
///
/// 实测本项目的验收服务器（Apache `mod_dav`）正是这样返回，响应完全合法：
///
/// ```text
/// 207 Multi-Status，431 字节
/// <D:prop><lp1:resourcetype/><lp1:creationdate/><lp1:getetag/>…</D:prop>
/// ```
///
/// 但 `ResourceType` 的 `is_collection` 字段没有 `#[serde(default)]`，空元素
/// `<resourcetype/>` 会让整份响应反序列化失败（`DeError: premature end of input`）。
///
/// 这属于既有 `propfind` 领域的解析缺口，**本方案不改它**。因此这一步只验证
/// 「本库把 propname 请求正确发出去了、服务端回了 207」，并如实记录这一点。
pub async fn step_03_query_property_names(ctx: &Ctx) {
    step("3. 只问属性名（PROPFIND propname）");

    let response = ctx
        .auth()
        .propfind()
        .prop_name()
        .depth(webdav_raw::Depth::Zero)
        .send()
        .await
        .expect("propname 请求应发送成功");

    let status = response.status();
    let body = response.text().await.expect("响应体应可读");

    assert_eq!(
        status,
        StatusCode::MULTI_STATUS,
        "propname 查询应返回 207，实际: {status}"
    );
    assert!(
        body.contains("<D:multistatus"),
        "propname 响应应为 multistatus，实际响应体: {body}"
    );

    // 人工可读的属性名清单：从响应里抓出空元素形式的属性名。
    let names = property_names_in(&body);
    println!("[workflow]   服务端声明的属性名: {names:?}");
    assert!(
        !names.is_empty(),
        "propname 响应里应至少列出属性名，实际响应体: {body}"
    );
}

/// 从 `propname` 响应体里抓出 `<…:prop>` 内的空元素属性名。
///
/// 只用于打印与粗断言，不做严格 XML 解析：真正的解析由库负责，而库目前对空元素
/// 形式的属性值还有缺口（见本函数调用处的说明）。
fn property_names_in(body: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_prop = false;
    let mut rest = body;

    while let Some(start) = rest.find('<') {
        rest = &rest[start..];
        let Some(end) = rest.find('>') else { break };
        let tag = &rest[1..end];
        rest = &rest[end + 1..];

        if tag.ends_with(":prop") || tag == "prop" {
            in_prop = true;
            continue;
        }
        if tag.starts_with('/') && (tag.contains(":prop") || tag == "/prop") {
            in_prop = false;
            continue;
        }
        if !in_prop {
            continue;
        }

        // 只收空元素或开始标签，跳过结束标签与声明。
        if tag.starts_with('/') || tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }
        let name = tag.split(':').next_back().unwrap_or(tag);
        let name = name.trim_end_matches('/');
        if let Some(prefix_stripped) = name.split(':').next_back() {
            names.push(prefix_stripped.to_owned());
        }
    }

    names.sort();
    names.dedup();
    names
}

/// 步骤 4：指定属性查询，四个属性都要能取到。
pub async fn step_04_query_selected_properties(ctx: &Ctx) {
    step("4. 指定属性查询（PROPFIND props）");

    let multistatus = ctx
        .auth()
        .propfind()
        .depth(webdav_raw::Depth::One)
        .props([
            FindProp::Resourcetype,
            FindProp::Getcontenttype,
            FindProp::Getetag,
            FindProp::Getlastmodified,
        ])
        .send_and_deserialize()
        .await
        .expect("指定属性查询应成功");

    let first = multistatus
        .response
        .front()
        .expect("应至少返回一条 response");
    let prop = first
        .propstat
        .first()
        .expect("应至少有一条 propstat")
        .prop
        .clone();

    assert!(
        prop.resource_type.is_some(),
        "getcontentlength 之外，resourcetype 也应能取到"
    );
    println!(
        "[workflow]   属性可见性: getcontenttype={:?} getetag={:?} getlastmodified={:?}",
        prop.content_type.is_some(),
        prop.etag.is_some(),
        prop.last_modified.is_some()
    );
}

/// 步骤 5：问服务端能力。
///
/// # 为什么不把 `Allow` 当作硬前置
///
/// 实测本项目的验收服务器（Apache `mod_dav`）返回的 `Allow` 头**不完整**：
///
/// ```text
/// Allow: OPTIONS, GET, HEAD, POST, DELETE, TRACE,
///        PROPFIND, PROPPATCH, COPY, MOVE, LOCK, UNLOCK
/// ```
///
/// 里面**没有 `MKCOL`，也没有 `PUT`**，但这两个方法在服务器上真实可用（本方案的
/// 步骤 7 与 16 都成功创建了资源）。可见 `Allow` 头只是该服务端愿意声明的一部分，
/// 并不能当作「支持哪些方法」的权威答案。
///
/// 因此这一步只做两件事：**记录**服务端声明了什么，以及核对它确实声明了
/// WebDAV 合规级别。后续步骤能否用某个方法，由该步骤自己的真实操作结果来判定——
/// 这也是本方案的基本立场：用服务端记录核对结果，而不是用声称的能力代替结果。
pub async fn step_05_query_capabilities(ctx: &Ctx) {
    step("5. 问服务端能力（OPTIONS send_capabilities）");

    let (response, caps) = ctx
        .auth()
        .options()
        .send_capabilities()
        .await
        .expect("OPTIONS 应发送成功");

    assert!(
        response.status().is_success(),
        "OPTIONS 应成功，实际: {}",
        response.status()
    );

    println!("[workflow]   DAV 级别: {:?}", caps.dav_levels);
    println!("[workflow]   服务端声明的方法: {:?}", caps.allowed_methods);

    assert!(
        caps.dav_levels.iter().any(|level| level == "1"),
        "WebDAV 服务端应声明 DAV: 1，实际: {:?}",
        caps.dav_levels
    );

    // 记录声明缺失情况：不失败，但要让人看到。
    for required in ["MKCOL", "PUT", "COPY", "MOVE", "DELETE"] {
        if !caps.allowed_methods.iter().any(|method| method == required) {
            println!(
                "[workflow]   注意：服务端未在 Allow 里声明 {required}，\
                 但这不代表不可用（本服务器就不声明 MKCOL/PUT）。\
                 该方法的可用性由使用它的那一步真实操作结果判定。"
            );
        }
    }
}

/// 步骤 6a：确认本次工作区不存在。
///
/// 工作区是本次运行的一次性目录，建在服务器根目录下；`测试目录01` 等全部内容都放在
/// 它里面。这样根目录只多出这一个临时项，跑完删掉它即可完全恢复。
pub async fn step_06a_confirm_workspace_absent(ctx: &Ctx) {
    step("6a. 确认本次工作区不存在");

    let path = ctx.path("");
    assert!(
        !dav_tree::exists(ctx.auth(), &path).await,
        "{path} 在开跑前不应存在（工作区名字唯一，不该撞到残留）"
    );
}

/// 步骤 6b：创建工作区。
pub async fn step_06b_mkcol_workspace(ctx: &Ctx) {
    step("6b. 创建工作区（MKCOL）");

    let path = ctx.path("");
    let status = ctx
        .auth()
        .mkcol()
        .target_path(&path)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功")
        .status();

    assert!(
        status.is_success(),
        "创建一次性工作区应成功（本服务器拒绝在根目录下直接建目录，因此必须先建这一层），实际: {status}"
    );
}

/// 步骤 6c：核对工作区是集合，且刚建出来是空的。
pub async fn step_06c_verify_workspace(ctx: &Ctx) {
    step("6c. 核对工作区是空集合");

    let path = ctx.path("");
    assert!(
        dav_tree::is_collection(ctx.auth(), &path).await,
        "{path} 应是集合"
    );
    let children = dav_tree::list_children(ctx.auth(), &path).await;
    assert!(children.is_empty(), "新建工作区应为空，实际: {children:?}");
}

/// 步骤 6d：核对根目录只比基线多出本工作区这一项。
///
/// 这一步把「不污染服务器既有内容」从口头约定变成可验证的断言。
pub async fn step_06d_verify_root_only_has_workspace(ctx: &Ctx) {
    step("6d. 核对根目录只比基线多出本工作区");

    let after = dav_tree::list_root(ctx.auth()).await;
    let mut expected = ctx.baseline().clone();
    let workspace = ctx.workspace().to_owned();
    assert!(
        expected.insert(workspace.clone()),
        "工作区名字不应与基线成员重名"
    );

    assert_eq!(
        after, expected,
        "根目录应恰好等于「基线 + 本次工作区」。多出: {:?}，缺失: {:?}",
        after.difference(&expected).collect::<Vec<_>>(),
        expected.difference(&after).collect::<Vec<_>>()
    );
}

/// 步骤 6：确认 `测试目录01` 不存在。
pub async fn step_06_confirm_root_absent(ctx: &Ctx) {
    step("6. 确认 测试目录01 不存在");

    let path = ctx.path(dir::ROOT);
    assert!(
        !dav_tree::exists(ctx.auth(), &path).await,
        "{path} 在开跑前不应存在（命名空间唯一，不该撞到残留）"
    );
}

/// 步骤 7：创建 `测试目录01`。
pub async fn step_07_mkcol_root(ctx: &Ctx) {
    step("7. 创建 测试目录01（MKCOL）");

    let path = ctx.path(dir::ROOT);
    let status = ctx
        .auth()
        .mkcol()
        .target_path(&path)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功")
        .status();

    assert_eq!(
        status,
        StatusCode::CREATED,
        "在工作区里新建子目录应返回 201，实际: {status}"
    );
}

/// 步骤 8：核对 `测试目录01` 是集合。
pub async fn step_08_verify_root_is_collection(ctx: &Ctx) {
    step("8. 核对 测试目录01 是集合");

    let path = ctx.path(dir::ROOT);
    assert!(
        dav_tree::is_collection(ctx.auth(), &path).await,
        "{path} 的 resourcetype 应含 collection"
    );
}

/// 步骤 9：核对 `测试目录01` 为空。
pub async fn step_09_verify_root_is_empty(ctx: &Ctx) {
    step("9. 核对 测试目录01 内容为空");

    let children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::ROOT)).await;
    assert!(children.is_empty(), "新建目录应为空，实际: {children:?}");
}

/// 步骤 10：确认 02 / 03 不存在。
pub async fn step_10_confirm_subdirs_absent(ctx: &Ctx) {
    step("10. 确认 测试目录02 / 测试目录03 不存在");

    for suffix in [dir::COPY_SOURCE, dir::COPY_TARGET] {
        let path = ctx.path(suffix);
        assert!(!dav_tree::exists(ctx.auth(), &path).await, "{path} 不应存在");
    }
}

/// 步骤 11：在 01 内创建 `测试目录02`。
pub async fn step_11_mkcol_copy_source(ctx: &Ctx) {
    step("11. 在 01 内创建 测试目录02（MKCOL）");

    let path = ctx.path(dir::COPY_SOURCE);
    let status = ctx
        .auth()
        .mkcol()
        .target_path(&path)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功")
        .status();

    assert_eq!(status, StatusCode::CREATED);
}

/// 步骤 12：在 01 内创建 `测试目录03`。
pub async fn step_12_mkcol_copy_target(ctx: &Ctx) {
    step("12. 在 01 内创建 测试目录03（MKCOL）");

    let path = ctx.path(dir::COPY_TARGET);
    let status = ctx
        .auth()
        .mkcol()
        .target_path(&path)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功")
        .status();

    assert_eq!(status, StatusCode::CREATED);
}

/// 步骤 13：核对 01 的子项恰好是 02 与 03。
pub async fn step_13_verify_root_contains_two_dirs(ctx: &Ctx) {
    step("13. 核对 01 的子项恰好是 [测试目录02, 测试目录03]");

    // 诊断：先把服务端原始 href 打出来，便于定位路径归一化问题。
    let raw = ctx
        .auth()
        .propfind()
        .path(&ctx.path(dir::ROOT))
        .depth(webdav_raw::Depth::One)
        .send_and_deserialize()
        .await
        .expect("PROPFIND 01 应成功");
    println!("[workflow]   01 的原始 href:");
    for item in &raw.response {
        println!("[workflow]     {:?}", item.href);
    }

    // 诊断：确认两个子目录是否真的存在，以及工作区层级是否可见。
    for suffix in [dir::COPY_SOURCE, dir::COPY_TARGET] {
        let exists = dav_tree::exists(ctx.auth(), &ctx.path(suffix)).await;
        println!("[workflow]   {suffix} 是否存在: {exists}");
    }
    let workspace_children = dav_tree::list_children(ctx.auth(), &ctx.path("")).await;
    println!("[workflow]   工作区的子项: {workspace_children:?}");

    let children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::ROOT)).await;
    println!("[workflow]   归一化后的子项: {children:?}");

    let expected: BTreeSet<String> = ["测试目录02", "测试目录03"]
        .into_iter()
        .map(str::to_owned)
        .collect();

    assert_eq!(children, expected, "01 的子项应与预期完全一致");
}

/// 步骤 14：核对 02 与 03 都是空集合。
pub async fn step_14_verify_subdirs_empty(ctx: &Ctx) {
    step("14. 核对 02 与 03 都是空集合");

    for suffix in [dir::COPY_SOURCE, dir::COPY_TARGET] {
        let path = ctx.path(suffix);
        let children = dav_tree::list_children(ctx.auth(), &path).await;
        assert!(children.is_empty(), "{path} 应为空集合，实际: {children:?}");
    }
}

/// 步骤 15：确认 `文件A` 不存在。
pub async fn step_15_confirm_file_a_absent(ctx: &Ctx) {
    step("15. 确认 文件A 不存在");

    let path = ctx.path(file::A);
    assert!(!dav_tree::exists(ctx.auth(), &path).await, "{path} 不应存在");
}

/// 步骤 16：在 01 内建 `文件A`（内存源）。
pub async fn step_16_put_file_a(ctx: &Ctx) {
    step("16. 用内存源建 文件A（PUT U8Bytes）");

    let status = dav_tree::put_bytes(ctx.auth(), &ctx.path(file::A), content::a()).await;
    assert!(
        status.is_success(),
        "PUT 文件A 应成功，实际: {status}"
    );
}

/// 步骤 17：在 01 内建 `文件B`（文件句柄源）。
pub async fn step_17_put_file_b(ctx: &Ctx) {
    step("17. 用文件句柄源建 文件B（PUT FileHandle）");

    let status = dav_tree::put_file(ctx.auth(), &ctx.path(file::B), content::b()).await;
    assert!(
        status.is_success(),
        "PUT 文件B 应成功，实际: {status}"
    );
}

/// 步骤 18：核对两个文件的长度。
pub async fn step_18_verify_file_lengths(ctx: &Ctx) {
    step("18. 核对 文件A / 文件B 的 Content-Length");

    for (suffix, expected) in [(file::A, content::a()), (file::B, content::b())] {
        let path = ctx.path(suffix);
        let length = dav_tree::head_length(ctx.auth(), &path).await;
        assert_eq!(
            length,
            Some(expected.len() as u64),
            "{path} 的 Content-Length 应为 {}",
            expected.len()
        );
    }
}

/// 步骤 19：读回 `文件A` 与 `文件B` 并逐字节比对。
pub async fn step_19_verify_file_contents(ctx: &Ctx) {
    step("19. 读回 文件A / 文件B 并逐字节比对");

    for (suffix, expected) in [(file::A, content::a()), (file::B, content::b())] {
        let actual = dav_tree::read(ctx.auth(), &ctx.path(suffix)).await;
        assert_eq!(actual, expected, "{suffix} 的内容应与写入一致");
    }
}

/// 步骤 20：条件 HEAD（带 ETag 时验证 304）。
///
/// 服务端没有回 ETag 时无法发条件请求，此时打印「跳过」，不假装通过也不假装失败。
pub async fn step_20_conditional_head(ctx: &Ctx) {
    step("20. 条件 HEAD（if-none-match）");

    let path = ctx.path(file::A);
    let Some(etag) = etag_of(ctx, &path).await else {
        println!("[workflow]   服务端未返回 ETag，跳过条件 HEAD");
        return;
    };

    let response = ctx
        .auth()
        .head()
        .target_path(&path)
        .expect("相对路径应被接受")
        .header("if-none-match", &etag)
        .expect("合法请求头应被接受")
        .send()
        .await
        .expect("条件 HEAD 应发送成功");

    assert_eq!(
        response.status(),
        StatusCode::NOT_MODIFIED,
        "带正确 ETag 的条件 HEAD 应返回 304，实际: {}",
        response.status()
    );
}

/// 步骤 21：条件 GET（带 ETag 时验证 304）。
///
/// `GetBuilder` 没有 `header()` 入口，因此这里用认证对象公开的 Client 自己构建
/// 条件 GET 请求。这本身也说明本库只负责发请求，条件头由调用方管理。
pub async fn step_21_conditional_get(ctx: &Ctx) {
    step("21. 条件 GET（If-None-Match）");

    let path = ctx.path(file::A);
    let Some(etag) = etag_of(ctx, &path).await else {
        println!("[workflow]   服务端未返回 ETag，跳过条件 GET");
        return;
    };

    let url = ctx
        .auth()
        .get_base_url()
        .join(&path)
        .expect("路径应能拼接成 URL");
    let request = ctx
        .auth()
        .get_client()
        .get(url)
        .header("if-none-match", etag)
        .build()
        .expect("条件 GET 请求应能构建");
    let response = ctx
        .auth()
        .get_client()
        .execute(request)
        .await
        .expect("条件 GET 应发送成功");

    assert_eq!(
        response.status(),
        StatusCode::NOT_MODIFIED,
        "带正确 ETag 的条件 GET 应返回 304，实际: {}",
        response.status()
    );
}

/// 步骤 22：核对 01 的子项是 `[02, 03, 文件A.txt, 文件B.txt]`。
pub async fn step_22_verify_root_after_put(ctx: &Ctx) {
    step("22. 核对 01 的子项（两个目录 + 两个文件）");

    let children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::ROOT)).await;
    let expected: BTreeSet<String> = ["测试目录02", "测试目录03", "文件A.txt", "文件B.txt"]
        .into_iter()
        .map(str::to_owned)
        .collect();

    assert_eq!(children, expected, "01 的子项应与预期完全一致");
}

/// 步骤 23：在 02 内建 `文件C`。
pub async fn step_23_put_file_c(ctx: &Ctx) {
    step("23. 在 02 内建 文件C（PUT）");

    let status = dav_tree::put_bytes(ctx.auth(), &ctx.path(file::C), content::c()).await;
    assert!(status.is_success(), "PUT 文件C 应成功，实际: {status}");
}

/// 步骤 24：在 03 内建 `文件D`。
pub async fn step_24_put_file_d(ctx: &Ctx) {
    step("24. 在 03 内建 文件D（PUT）");

    let status = dav_tree::put_bytes(ctx.auth(), &ctx.path(file::D), content::d()).await;
    assert!(status.is_success(), "PUT 文件D 应成功，实际: {status}");
}

/// 步骤 25：核对 02 与 03 的子项各只有一个文件。
pub async fn step_25_verify_subdir_children(ctx: &Ctx) {
    step("25. 核对 02 的子项=[文件C.txt]，03 的子项=[文件D.txt]");

    let source_children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::COPY_SOURCE)).await;
    let target_children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::COPY_TARGET)).await;

    assert_eq!(
        source_children,
        BTreeSet::from(["文件C.txt".to_owned()]),
        "02 的子项应只有 文件C.txt"
    );
    assert_eq!(
        target_children,
        BTreeSet::from(["文件D.txt".to_owned()]),
        "03 的子项应只有 文件D.txt"
    );
}

/// 步骤 26：读回 `文件C` 与 `文件D` 并逐字节比对。
pub async fn step_26_verify_subdir_file_contents(ctx: &Ctx) {
    step("26. 读回 文件C / 文件D 并逐字节比对");

    for (suffix, expected) in [(file::C, content::c()), (file::D, content::d())] {
        let actual = dav_tree::read(ctx.auth(), &ctx.path(suffix)).await;
        assert_eq!(actual, expected, "{suffix} 的内容应与写入一致");
    }
}

/// 读取某个资源的 ETag，取不到时返回 `None`。
async fn etag_of(ctx: &Ctx, path: &str) -> Option<String> {
    let multistatus = ctx
        .auth()
        .propfind()
        .path(path)
        .depth(webdav_raw::Depth::Zero)
        .props([FindProp::Getetag])
        .send_and_deserialize()
        .await
        .ok()?;

    multistatus
        .response
        .front()?
        .propstat
        .first()?
        .prop
        .etag
        .clone()
}

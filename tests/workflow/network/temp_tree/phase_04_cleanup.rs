//! 阶段四（步骤 47–49）：终态逐项核对、清理、根目录与基线比对。
//!
//! 每个步骤一个函数，只做一件事。

use std::collections::BTreeSet;

use webdav_raw::FindProp;

use super::data::{content, dir, file};
use crate::support::context::{Ctx, step};
use crate::support::dav_tree;

/// 步骤 47a：终态核对 01 内每个成员的存在性。
pub async fn step_47a_verify_final_members_present(ctx: &Ctx) {
    step("47a. 终态核对 01 内的每个成员");

    for suffix in [dir::MOVE_TARGET, file::A, file::B, file::C_IN_01] {
        let path = ctx.path(suffix);
        assert!(
            dav_tree::exists(ctx.auth(), &path).await,
            "{path} 在终态应存在"
        );
    }
}

/// 步骤 47b：终态核对每个文件的内容。
pub async fn step_47b_verify_final_file_contents(ctx: &Ctx) {
    step("47b. 终态核对 01 内每个文件的内容");

    for suffix in [file::A, file::B, file::C_IN_01, file::D_IN_04] {
        let expected = content::expected_for(suffix).expect("终态文件都应在这张表里");
        let actual = dav_tree::read(ctx.auth(), &ctx.path(suffix)).await;
        assert_eq!(actual, expected, "{suffix} 的终态内容应与预期一致");
    }
}

/// 步骤 47c：终态核对每个文件的长度与资源类型属性。
///
/// 用 `FindProp::Getcontentlength` 按名请求长度属性：这个变体是后来补上的，在此之前
/// 按名请求无法索要长度，只能靠 `allprop` 顺带拿回。
pub async fn step_47c_verify_final_properties(ctx: &Ctx) {
    step("47c. 终态核对每个文件的长度与资源类型属性");

    for suffix in [file::A, file::B, file::C_IN_01, file::D_IN_04] {
        let expected = content::expected_for(suffix).expect("终态文件都应在这张表里");
        let path = ctx.path(suffix);

        let multistatus = ctx
            .auth()
            .propfind()
            .path(&path)
            .depth(webdav_raw::Depth::Zero)
            .props([
                FindProp::Resourcetype,
                FindProp::Getcontentlength,
                FindProp::Getetag,
            ])
            .send_and_deserialize()
            .await
            .expect("终态 PROPFIND 应成功");

        let prop = multistatus
            .response
            .front()
            .and_then(|item| item.propstat.first())
            .map(|propstat| propstat.prop.clone())
            .expect("应返回一条 propstat");

        assert_eq!(
            prop.content_length,
            Some(expected.len() as u64),
            "{suffix} 的长度属性（getcontentlength）应与内容长度一致"
        );

        let is_collection = prop
            .resource_type
            .as_ref()
            .and_then(|resource_type| resource_type.is_collection.as_ref())
            .is_some();
        assert!(!is_collection, "{suffix} 是文件，不应被标成集合");
    }
}

/// 步骤 47d：终态核对 04 仍然只含被移入的那个文件。
pub async fn step_47d_verify_final_move_target(ctx: &Ctx) {
    step("47d. 终态核对 04 的子项");

    let children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::MOVE_TARGET)).await;
    assert_eq!(
        children,
        BTreeSet::from(["文件D.txt".to_owned()]),
        "04 在终态应只含 文件D.txt"
    );
}

/// 步骤 48：清理本次工作区。
///
/// 工作区是本次运行唯一建在服务器根目录下的东西，递归删掉它，根目录就回到原样。
pub async fn step_48_cleanup(ctx: &Ctx) {
    step("48. 清理本次工作区（DELETE 工作区 Depth:infinity）");

    let status = dav_tree::delete_tree(ctx.auth(), &ctx.path("")).await;
    assert!(
        status.is_success(),
        "清理工作区应成功，实际: {status}。若失败，服务器会残留工作区 {}",
        ctx.workspace()
    );
}

/// 步骤 49a：核对工作区查不到了。
pub async fn step_49a_verify_namespace_gone(ctx: &Ctx) {
    step("49a. 核对本次工作区已查不到");

    let path = ctx.path("");
    assert!(
        !dav_tree::exists(ctx.auth(), &path).await,
        "清理后工作区 {path} 不应还能被查到"
    );
}

/// 步骤 49b：核对根目录没有任何本方案前缀的残留。
pub async fn step_49b_verify_no_leftovers(ctx: &Ctx) {
    step("49b. 核对根目录没有本方案前缀的残留");

    let leftovers = dav_tree::list_root_with_prefix(ctx.auth(), dav_tree::NAMESPACE_PREFIX).await;
    assert!(
        leftovers.is_empty(),
        "根目录不应残留本方案创建的工作区，实际: {leftovers:?}"
    );
}

/// 步骤 49c：核对根目录与开跑前的基线逐项一致。
///
/// 这一步是「服务端既有内容一项未少」的最终证据。
pub async fn step_49c_verify_root_matches_baseline(ctx: &Ctx) {
    step("49c. 核对根目录与基线逐项一致");

    let after = dav_tree::list_root(ctx.auth()).await;
    let baseline = ctx.baseline();

    assert_eq!(
        &after, baseline,
        "根目录应与开跑前完全一致。缺失: {:?}，多出: {:?}",
        baseline.difference(&after).collect::<Vec<_>>(),
        after.difference(baseline).collect::<Vec<_>>()
    );
    println!("[workflow]   根目录与基线一致: {after:?}");
}

//! 阶段三（步骤 37–46）：移动 `文件D` 到 04，核对源消失，再删除 02 与 03。
//!
//! 每个步骤一个函数，只做一件事。

use std::collections::BTreeSet;

use webdav_raw::{MoveDepth, Overwrite};

use super::data::{content, dir, file};
use crate::support::context::{Ctx, step};
use crate::support::dav_tree;

/// 步骤 37a：移动前确认源存在。
pub async fn step_37a_confirm_move_source_present(ctx: &Ctx) {
    step("37a. 移动前确认 03/文件D 存在");

    let path = ctx.path(file::D);
    assert!(
        dav_tree::exists(ctx.auth(), &path).await,
        "{path} 在移动前应存在"
    );
}

/// 步骤 37b：移动前确认目标不存在。
pub async fn step_37b_confirm_move_target_absent(ctx: &Ctx) {
    step("37b. 移动前确认 04/文件D 不存在");

    let path = ctx.path(file::D_IN_04);
    assert!(
        !dav_tree::exists(ctx.auth(), &path).await,
        "{path} 在移动前不应存在"
    );
}

/// 步骤 38：把 03 的 `文件D` 移动到 04。
pub async fn step_38_move_file_d_to_dir04(ctx: &Ctx) {
    step("38. 把 文件D 从 03 移动到 04（MOVE Overwrite:False）");

    let response = ctx
        .auth()
        .mv()
        .move_from_path(&ctx.path(file::D))
        .expect("源路径应合法")
        .target_path(&ctx.path(file::D_IN_04))
        .expect("目标路径应合法")
        .depth(MoveDepth::Zero)
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("MOVE 应发送成功");

    assert!(
        response.status().is_success(),
        "移动到不存在的位置应成功，实际: {}",
        response.status()
    );
}

/// 步骤 39a：核对 04 里的文件存在且内容正确。
pub async fn step_39a_verify_moved_file_content(ctx: &Ctx) {
    step("39a. 核对 04/文件D 的内容与源一致");

    let moved = dav_tree::read(ctx.auth(), &ctx.path(file::D_IN_04)).await;
    assert_eq!(moved, content::d(), "移动后的文件内容应与原内容一致");
}

/// 步骤 39b：核对 04 里的文件长度正确。
pub async fn step_39b_verify_moved_file_length(ctx: &Ctx) {
    step("39b. 核对 04/文件D 的 Content-Length");

    let length = dav_tree::head_length(ctx.auth(), &ctx.path(file::D_IN_04)).await;
    assert_eq!(
        length,
        Some(content::d().len() as u64),
        "移动后的文件长度应与原内容一致"
    );
}

/// 步骤 40a：核对源已消失（HEAD）。
pub async fn step_40a_verify_move_source_gone_by_head(ctx: &Ctx) {
    step("40a. 核对 03/文件D 已查不到（HEAD）");

    let response = ctx
        .auth()
        .head()
        .target_path(&ctx.path(file::D))
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("HEAD 应发送成功");

    assert!(
        !response.status().is_success(),
        "MOVE 之后源应查不到，实际状态码: {}",
        response.status()
    );
}

/// 步骤 40b：核对源已消失（PROPFIND）。
pub async fn step_40b_verify_move_source_gone_by_propfind(ctx: &Ctx) {
    step("40b. 核对 03/文件D 已查不到（PROPFIND）");

    let path = ctx.path(file::D);
    assert!(
        !dav_tree::exists(ctx.auth(), &path).await,
        "MOVE 之后 {path} 不应还能被 PROPFIND 查到"
    );
}

/// 步骤 41：核对 03 与 04 的子项。
pub async fn step_41_verify_structures_after_move(ctx: &Ctx) {
    step("41. 核对 03 的子项=[文件C.txt]，04 的子项=[文件D.txt]");

    let source_children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::COPY_TARGET)).await;
    assert_eq!(
        source_children,
        BTreeSet::from(["文件C.txt".to_owned()]),
        "03 的子项应只剩 文件C.txt"
    );

    let target_children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::MOVE_TARGET)).await;
    assert_eq!(
        target_children,
        BTreeSet::from(["文件D.txt".to_owned()]),
        "04 的子项应只有 文件D.txt"
    );
}

/// 步骤 42：删除前记录 02 与 03 的形态。
///
/// 这一步是删除边界的依据：两者此刻都**非空**，因此接下来的删除验证的是
/// 「非空集合的递归删除」。空集合删除由负向用例 N10 覆盖。
pub async fn step_42_record_shapes_before_delete(ctx: &Ctx) {
    step("42. 删除前记录 02 与 03 的形态（应都非空）");

    let source_children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::COPY_SOURCE)).await;
    let target_children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::COPY_TARGET)).await;

    println!(
        "[workflow]   02 的子项: {source_children:?}（非空? {}）",
        !source_children.is_empty()
    );
    println!(
        "[workflow]   03 的子项: {target_children:?}（非空? {}）",
        !target_children.is_empty()
    );

    assert!(
        !source_children.is_empty(),
        "02 此刻应非空，否则这一步验证的不再是非空集合删除"
    );
    assert!(
        !target_children.is_empty(),
        "03 此刻应非空，否则这一步验证的不再是非空集合删除"
    );
}

/// 步骤 43：递归删除 `测试目录02`（非空集合）。
pub async fn step_43_delete_copy_source(ctx: &Ctx) {
    step("43. 递归删除 测试目录02（DELETE Depth:infinity，非空集合）");

    let status = dav_tree::delete_tree(ctx.auth(), &ctx.path(dir::COPY_SOURCE)).await;
    assert!(
        status.is_success(),
        "删除非空集合应成功，实际: {status}"
    );
}

/// 步骤 44：递归删除 `测试目录03`（非空集合）。
pub async fn step_44_delete_copy_target(ctx: &Ctx) {
    step("44. 递归删除 测试目录03（DELETE Depth:infinity，非空集合）");

    let status = dav_tree::delete_tree(ctx.auth(), &ctx.path(dir::COPY_TARGET)).await;
    assert!(
        status.is_success(),
        "删除非空集合应成功，实际: {status}"
    );
}

/// 步骤 45：核对 02 与 03 都查不到了。
pub async fn step_45_verify_dirs_deleted(ctx: &Ctx) {
    step("45. 核对 02 与 03 都查不到");

    for suffix in [dir::COPY_SOURCE, dir::COPY_TARGET] {
        let path = ctx.path(suffix);
        assert!(
            !dav_tree::exists(ctx.auth(), &path).await,
            "删除后 {path} 不应还能被查到"
        );
    }

    // 连带确认被删目录里的文件也一并消失。
    for suffix in [file::C, file::C_IN_03, file::D_IN_04] {
        let path = ctx.path(suffix);
        let still_there = dav_tree::exists(ctx.auth(), &path).await;
        if suffix == file::D_IN_04 {
            assert!(still_there, "{path} 属于 04，不应被 02/03 的删除波及");
        } else {
            assert!(!still_there, "{path} 应随其父目录一起被删除");
        }
    }
}

/// 步骤 46：核对 01 的子项。
pub async fn step_46_verify_root_after_delete(ctx: &Ctx) {
    step("46. 核对 01 的子项（02 与 03 已消失）");

    let children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::ROOT)).await;
    let expected: BTreeSet<String> = ["测试目录04", "文件A.txt", "文件B.txt", "文件C.txt"]
        .into_iter()
        .map(str::to_owned)
        .collect();

    assert_eq!(children, expected, "01 的子项应与预期一致");
}

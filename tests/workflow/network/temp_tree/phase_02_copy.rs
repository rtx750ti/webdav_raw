//! 阶段二（步骤 27–32）：复制 `文件C` 到 01 与 03，并核对副本与源。
//!
//! 每个步骤一个函数，只做一件事。

use std::collections::BTreeSet;

use webdav_raw::{CopyDepth, Overwrite, StatusCode};

use super::data::{content, dir, file};
use crate::support::context::{Ctx, step};
use crate::support::dav_tree;

/// 步骤 27：确认两个复制目标都不存在。
pub async fn step_27_confirm_copy_targets_absent(ctx: &Ctx) {
    step("27. 确认两个复制目标都不存在");

    for suffix in [file::C_IN_01, file::C_IN_03] {
        let path = ctx.path(suffix);
        assert!(!dav_tree::exists(ctx.auth(), &path).await, "{path} 不应存在");
    }
}

/// 步骤 28：把 02 的 `文件C` 复制到 01。
pub async fn step_28_copy_file_c_to_dir01(ctx: &Ctx) {
    step("28. 把 文件C 从 02 复制到 01（COPY Depth:0 Overwrite:False）");

    let response = ctx
        .auth()
        .copy()
        .source_path(&ctx.path(file::C))
        .expect("源路径应合法")
        .target_path(&ctx.path(file::C_IN_01))
        .expect("目标路径应合法")
        .depth(CopyDepth::Zero)
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("COPY 应发送成功");

    assert!(
        response.status().is_success(),
        "复制到不存在的位置应成功，实际: {}",
        response.status()
    );
}

/// 步骤 29：把 02 的 `文件C` 复制到 03。
pub async fn step_29_copy_file_c_to_dir03(ctx: &Ctx) {
    step("29. 把 文件C 从 02 复制到 03（COPY Depth:0 Overwrite:False）");

    let response = ctx
        .auth()
        .copy()
        .source_path(&ctx.path(file::C))
        .expect("源路径应合法")
        .target_path(&ctx.path(file::C_IN_03))
        .expect("目标路径应合法")
        .depth(CopyDepth::Zero)
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("COPY 应发送成功");

    assert!(
        response.status().is_success(),
        "复制到不存在的位置应成功，实际: {}",
        response.status()
    );
}

/// 步骤 30a：核对 01 里的副本与源内容一致。
pub async fn step_30a_verify_copy_in_dir01(ctx: &Ctx) {
    step("30a. 核对 01 里的副本内容与源一致");

    let copied = dav_tree::read(ctx.auth(), &ctx.path(file::C_IN_01)).await;
    let original = dav_tree::read(ctx.auth(), &ctx.path(file::C)).await;

    assert_eq!(copied, original, "副本内容应与源逐字节一致");
}

/// 步骤 30b：核对 03 里的副本与源内容一致，且长度正确。
pub async fn step_30b_verify_copy_in_dir03(ctx: &Ctx) {
    step("30b. 核对 03 里的副本内容与源一致，并核对长度");

    let copied = dav_tree::read(ctx.auth(), &ctx.path(file::C_IN_03)).await;
    let original = dav_tree::read(ctx.auth(), &ctx.path(file::C)).await;
    assert_eq!(copied, original, "副本内容应与源逐字节一致");

    let length = dav_tree::head_length(ctx.auth(), &ctx.path(file::C_IN_03)).await;
    assert_eq!(
        length,
        Some(content::c().len() as u64),
        "副本的 Content-Length 应与源一致"
    );
}

/// 步骤 31：确认 COPY 不删源。
pub async fn step_31_verify_source_still_present(ctx: &Ctx) {
    step("31. 确认 COPY 之后源仍在");

    let path = ctx.path(file::C);
    assert!(
        dav_tree::exists(ctx.auth(), &path).await,
        "{path} 在 COPY 之后应仍然存在"
    );
    let length = dav_tree::head_length(ctx.auth(), &path).await;
    assert_eq!(length, Some(content::c().len() as u64));
}

/// 步骤 32：核对 01 与 03 的子项。
pub async fn step_32_verify_structures_after_copy(ctx: &Ctx) {
    step("32. 核对 01 与 03 的子项");

    let root_children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::ROOT)).await;
    let expected_root: BTreeSet<String> = ["测试目录02", "测试目录03", "文件A.txt", "文件B.txt", "文件C.txt"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(root_children, expected_root, "01 的子项应与预期一致");

    let target_children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::COPY_TARGET)).await;
    let expected_target: BTreeSet<String> = ["文件C.txt", "文件D.txt"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(target_children, expected_target, "03 的子项应与预期一致");
}

/// 步骤 33：确认 `测试目录04` 不存在。
pub async fn step_33_confirm_move_target_absent(ctx: &Ctx) {
    step("33. 确认 测试目录04 不存在");

    let path = ctx.path(dir::MOVE_TARGET);
    assert!(!dav_tree::exists(ctx.auth(), &path).await, "{path} 不应存在");
}

/// 步骤 34：创建 `测试目录04`。
pub async fn step_34_mkcol_move_target(ctx: &Ctx) {
    step("34. 创建 测试目录04（MKCOL）");

    let path = ctx.path(dir::MOVE_TARGET);
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

/// 步骤 35：核对 04 是空集合。
pub async fn step_35_verify_move_target_empty(ctx: &Ctx) {
    step("35. 核对 04 是空集合");

    let children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::MOVE_TARGET)).await;
    assert!(children.is_empty(), "04 应为空集合，实际: {children:?}");
}

/// 步骤 36：核对 01 的子项。
pub async fn step_36_verify_root_after_mkcol04(ctx: &Ctx) {
    step("36. 核对 01 的子项（新增 测试目录04）");

    let children = dav_tree::list_children(ctx.auth(), &ctx.path(dir::ROOT)).await;
    let expected: BTreeSet<String> = [
        "测试目录02",
        "测试目录03",
        "测试目录04",
        "文件A.txt",
        "文件B.txt",
        "文件C.txt",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();

    assert_eq!(children, expected, "01 的子项应与预期一致");
}

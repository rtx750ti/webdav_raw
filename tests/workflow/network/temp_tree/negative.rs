//! 负向用例 N1–N10：每个用例一个函数，只验证一件事。
//!
//! 负向用例的价值不低于正向：它验证**失败路径没有副作用**。其中 N5、N6、N8 必须核对
//! 「既有内容一点没变」，这是最容易出错、也最容易造成真实损失的地方。

use webdav_core::{Overwrite, StatusCode};

use super::data::{content, dir, file};
use crate::support::context::{Ctx, step};
use crate::support::dav_tree;

/// 负向用例的准备动作：建出断言所需的既有资源。
///
/// 这些函数本身不是用例，只是搭台；每个用例仍然只验证一件事。
pub async fn setup_fixtures(ctx: &Ctx) {
    step("N0. 准备负向用例所需的既有资源");

    // 先建一次性工作区：与正向链路一样，不把内容直接建在服务器根目录下。
    let workspace = ctx
        .auth()
        .mkcol()
        .target_path(&ctx.path(""))
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功")
        .status();
    assert!(
        workspace.is_success(),
        "准备工作区应成功，实际: {workspace}"
    );

    for suffix in [dir::ROOT, dir::COPY_SOURCE, dir::COPY_TARGET] {
        let status = ctx
            .auth()
            .mkcol()
            .target_path(&ctx.path(suffix))
            .expect("相对路径应被接受")
            .send()
            .await
            .expect("MKCOL 应发送成功")
            .status();
        assert!(
            status.is_success(),
            "准备目录 {suffix} 应成功，实际: {status}"
        );
    }

    for (suffix, bytes) in [
        (file::A, content::a()),
        (file::C, content::c()),
        (file::C_IN_01, content::c()),
    ] {
        let status = dav_tree::put_bytes(ctx.auth(), &ctx.path(suffix), bytes).await;
        assert!(
            status.is_success(),
            "准备文件 {suffix} 应成功，实际: {status}"
        );
    }
}

/// N1：MKCOL 一个已存在的集合应失败，且该集合内容未被破坏。
pub async fn n01_mkcol_existing_collection_fails(ctx: &Ctx) {
    step("N1. MKCOL 已存在的集合应失败，且内容未变");

    let before = dav_tree::list_children(ctx.auth(), &ctx.path(dir::COPY_SOURCE)).await;

    let status = ctx
        .auth()
        .mkcol()
        .target_path(&ctx.path(dir::COPY_SOURCE))
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功")
        .status();

    assert!(
        !status.is_success(),
        "对已存在的集合再发 MKCOL 应失败，实际: {status}"
    );

    let after = dav_tree::list_children(ctx.auth(), &ctx.path(dir::COPY_SOURCE)).await;
    assert_eq!(after, before, "MKCOL 失败不应改变既有集合的内容");
}

/// N2：MKCOL 一个父目录不存在的路径应失败。
pub async fn n02_mkcol_missing_parent_fails(ctx: &Ctx) {
    step("N2. MKCOL 父目录不存在的路径应失败");

    let status = ctx
        .auth()
        .mkcol()
        .target_path(&ctx.path(dir::NEGATIVE_MISSING_PARENT))
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功")
        .status();

    assert!(
        !status.is_success(),
        "父目录不存在时应失败（通常 409），实际: {status}"
    );
    assert!(
        !dav_tree::exists(ctx.auth(), &ctx.path(dir::NEGATIVE_MISSING_PARENT)).await,
        "失败的 MKCOL 不应留下半个目录"
    );
}

/// N3：COPY 不存在的源应失败，且目标未被创建。
pub async fn n03_copy_missing_source_fails(ctx: &Ctx) {
    step("N3. COPY 不存在的源应失败，且目标未创建");

    let missing_source = ctx.path("测试目录01/根本不存在的文件.txt");
    let target = ctx.path("测试目录01/不该出现的副本.txt");

    let status = ctx
        .auth()
        .copy()
        .source_path(&missing_source)
        .expect("源路径应合法")
        .target_path(&target)
        .expect("目标路径应合法")
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("COPY 应发送成功")
        .status();

    assert!(
        !status.is_success(),
        "源不存在时 COPY 应失败，实际: {status}"
    );
    assert!(
        !dav_tree::exists(ctx.auth(), &target).await,
        "COPY 失败不应创建目标，但 {target} 存在了"
    );
}

/// N4：MOVE 不存在的源应失败，且目标未被创建。
pub async fn n04_move_missing_source_fails(ctx: &Ctx) {
    step("N4. MOVE 不存在的源应失败，且目标未创建");

    let missing_source = ctx.path("测试目录01/根本不存在的文件2.txt");
    let target = ctx.path("测试目录01/不该出现的移动目标.txt");

    let status = ctx
        .auth()
        .mv()
        .move_from_path(&missing_source)
        .expect("源路径应合法")
        .target_path(&target)
        .expect("目标路径应合法")
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("MOVE 应发送成功")
        .status();

    assert!(
        !status.is_success(),
        "源不存在时 MOVE 应失败，实际: {status}"
    );
    assert!(
        !dav_tree::exists(ctx.auth(), &target).await,
        "MOVE 失败不应创建目标，但 {target} 存在了"
    );
}

/// N5：COPY 到已存在目标且 `Overwrite::False` 应失败，既有目标内容一点没变。
pub async fn n05_copy_without_overwrite_keeps_target(ctx: &Ctx) {
    step("N5. COPY 撞已存在目标且禁止覆盖：应失败且既有内容逐字节未变");

    let existing_target = ctx.path(file::C_IN_01);
    let before = dav_tree::read(ctx.auth(), &existing_target).await;

    // 源用 文件A（内容与既有目标不同），确保「没被覆盖」这件事可被检出。
    let status = ctx
        .auth()
        .copy()
        .source_path(&ctx.path(file::A))
        .expect("源路径应合法")
        .target_path(&existing_target)
        .expect("目标路径应合法")
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("COPY 应发送成功")
        .status();

    assert_eq!(
        status,
        StatusCode::PRECONDITION_FAILED,
        "禁止覆盖且目标已存在时应返回 412，实际: {status}"
    );

    let after = dav_tree::read(ctx.auth(), &existing_target).await;
    assert_eq!(
        after, before,
        "COPY 被拒绝后目标内容必须逐字节保持不变"
    );
}

/// N6：MOVE 到已存在目标且 `Overwrite::False` 应失败，源仍在、目标未变。
pub async fn n06_move_without_overwrite_keeps_both(ctx: &Ctx) {
    step("N6. MOVE 撞已存在目标且禁止覆盖：应失败且源仍在、目标未变");

    let source = ctx.path(file::C);
    let existing_target = ctx.path(file::C_IN_01);
    let source_before = dav_tree::read(ctx.auth(), &source).await;
    let target_before = dav_tree::read(ctx.auth(), &existing_target).await;

    let status = ctx
        .auth()
        .mv()
        .move_from_path(&source)
        .expect("源路径应合法")
        .target_path(&existing_target)
        .expect("目标路径应合法")
        .overwrite(Overwrite::False)
        .send()
        .await
        .expect("MOVE 应发送成功")
        .status();

    assert!(
        !status.is_success(),
        "禁止覆盖且目标已存在时 MOVE 应失败，实际: {status}"
    );

    assert_eq!(
        dav_tree::read(ctx.auth(), &source).await,
        source_before,
        "MOVE 失败后源必须仍在且内容未变"
    );
    assert_eq!(
        dav_tree::read(ctx.auth(), &existing_target).await,
        target_before,
        "MOVE 失败后目标必须保持不变"
    );
}

/// N7：DELETE 不存在的路径应失败。
pub async fn n07_delete_missing_path_fails(ctx: &Ctx) {
    step("N7. DELETE 不存在的路径应失败");

    let status = dav_tree::delete_tree(ctx.auth(), &ctx.path("测试目录01/根本不存在的东西")).await;
    assert!(
        !status.is_success(),
        "删除不存在的路径应失败，实际: {status}"
    );
}

/// N8：DELETE 非空集合且 `Depth: 0` 应被服务端拒绝，且集合内容未丢。
pub async fn n08_delete_non_empty_with_zero_depth_fails(ctx: &Ctx) {
    step("N8. DELETE 非空集合且 Depth:0：应被拒绝且内容未丢");

    let before = dav_tree::list_children(ctx.auth(), &ctx.path(dir::COPY_SOURCE)).await;
    assert!(!before.is_empty(), "本用例要求 02 此刻非空");

    let status = dav_tree::delete_single(ctx.auth(), &ctx.path(dir::COPY_SOURCE)).await;
    assert!(
        !status.is_success(),
        "非空集合同 Depth:0 删除应被拒绝，实际: {status}"
    );

    assert!(
        dav_tree::exists(ctx.auth(), &ctx.path(dir::COPY_SOURCE)).await,
        "被拒绝的删除不应把集合删掉"
    );
    let after = dav_tree::list_children(ctx.auth(), &ctx.path(dir::COPY_SOURCE)).await;
    assert_eq!(after, before, "被拒绝的删除不应丢内容");
}

/// N9：MOVE 集合到自身子路径应失败。
pub async fn n09_move_collection_into_itself_fails(ctx: &Ctx) {
    step("N9. MOVE 集合到自身子路径应失败");

    let source = ctx.path(dir::COPY_SOURCE);
    let target = ctx.path("测试目录01/测试目录02/不该成功的子路径");

    let status = ctx
        .auth()
        .mv()
        .move_from_path(&source)
        .expect("源路径应合法")
        .target_path(&target)
        .expect("目标路径应合法")
        .send()
        .await
        .expect("MOVE 应发送成功")
        .status();

    assert!(
        !status.is_success(),
        "把集合移进自身应失败（通常 403），实际: {status}"
    );
    assert!(
        dav_tree::exists(ctx.auth(), &source).await,
        "失败的 MOVE 不应删掉源集合"
    );
}

/// N10：DELETE 空集合且 `Depth: infinity` 应成功（与 N8 构成空/非空对照）。
pub async fn n10_delete_empty_collection_succeeds(ctx: &Ctx) {
    step("N10. DELETE 空集合且 Depth:infinity 应成功");

    let path = ctx.path(dir::NEGATIVE_EMPTY);
    let created = ctx
        .auth()
        .mkcol()
        .target_path(&path)
        .expect("相对路径应被接受")
        .send()
        .await
        .expect("MKCOL 应发送成功")
        .status();
    assert!(created.is_success(), "建空集合应成功，实际: {created}");

    let children = dav_tree::list_children(ctx.auth(), &path).await;
    assert!(children.is_empty(), "本用例要求它是空集合");

    let status = dav_tree::delete_tree(ctx.auth(), &path).await;
    assert!(status.is_success(), "删除空集合应成功，实际: {status}");
    assert!(
        !dav_tree::exists(ctx.auth(), &path).await,
        "删除后空集合应查不到"
    );
}

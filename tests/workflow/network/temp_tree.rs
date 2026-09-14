//! 全覆盖工作流：在真实 WebDAV 服务器上依次使用本库的全部能力。
//!
//! # 这个文件里有什么
//!
//! 只有**编排**：按顺序调用各步骤函数，本身不含业务逻辑。每个步骤与每个负向用例
//! 都是独立函数，分别在 `phase_01_setup` / `phase_02_copy` / `phase_03_move` /
//! `phase_04_cleanup` / `negative` 里。
//!
//! # 为什么正向链路只用一个 `#[tokio::test]`
//!
//! 正向链路的 49 个步骤共享同一个命名空间，步骤之间有真实依赖（后一步要看到前一步
//! 创建的资源）。若每个步骤都是一个独立测试，每个测试都得把整套目录树重建一遍，
//! 会给真实服务器造成数倍请求量。因此可读性靠**函数粒度**，请求量靠**共享命名空间**。
//!
//! 也正因为共享命名空间，运行命令**必须带 `--test-threads=1`**，否则同进程内的步骤会
//! 被并行打乱。
//!
//! # 运行
//!
//! ```text
//! cargo test --features network-test --test workflow -- --ignored --nocapture --test-threads=1
//! ```

mod data;
mod negative;
mod phase_01_setup;
mod phase_02_copy;
mod phase_03_move;
mod phase_04_cleanup;

use crate::support::context::{Ctx, step};

/// 正向链路：查询 → 建目录树 → 建文件 → 复制 → 移动 → 删除 → 终态核对。
///
/// 任何一步断言失败都会立即中止，输出里能看到停在哪个步骤。
#[tokio::test]
#[ignore = "对真实 WebDAV 服务器做写操作；只在人工执行时运行"]
async fn main_workflow() {
    let ctx = Ctx::prepare().await;

    println!("\n===== 阶段一：查询根目录、建目录树、建文件 =====");
    phase_01_setup::step_01_query_root_self(&ctx).await;
    phase_01_setup::step_02_query_root_members(&ctx).await;
    phase_01_setup::step_03_query_property_names(&ctx).await;
    phase_01_setup::step_04_query_selected_properties(&ctx).await;
    phase_01_setup::step_05_query_capabilities(&ctx).await;
    phase_01_setup::step_06a_confirm_workspace_absent(&ctx).await;
    phase_01_setup::step_06b_mkcol_workspace(&ctx).await;
    phase_01_setup::step_06c_verify_workspace(&ctx).await;
    phase_01_setup::step_06d_verify_root_only_has_workspace(&ctx).await;
    phase_01_setup::step_06_confirm_root_absent(&ctx).await;
    phase_01_setup::step_07_mkcol_root(&ctx).await;
    phase_01_setup::step_08_verify_root_is_collection(&ctx).await;
    phase_01_setup::step_09_verify_root_is_empty(&ctx).await;
    phase_01_setup::step_10_confirm_subdirs_absent(&ctx).await;
    phase_01_setup::step_11_mkcol_copy_source(&ctx).await;
    phase_01_setup::step_12_mkcol_copy_target(&ctx).await;
    phase_01_setup::step_13_verify_root_contains_two_dirs(&ctx).await;
    phase_01_setup::step_14_verify_subdirs_empty(&ctx).await;
    phase_01_setup::step_15_confirm_file_a_absent(&ctx).await;
    phase_01_setup::step_16_put_file_a(&ctx).await;
    phase_01_setup::step_17_put_file_b(&ctx).await;
    phase_01_setup::step_18_verify_file_lengths(&ctx).await;
    phase_01_setup::step_19_verify_file_contents(&ctx).await;
    phase_01_setup::step_20_conditional_head(&ctx).await;
    phase_01_setup::step_21_conditional_get(&ctx).await;
    phase_01_setup::step_22_verify_root_after_put(&ctx).await;
    phase_01_setup::step_23_put_file_c(&ctx).await;
    phase_01_setup::step_24_put_file_d(&ctx).await;
    phase_01_setup::step_25_verify_subdir_children(&ctx).await;
    phase_01_setup::step_26_verify_subdir_file_contents(&ctx).await;

    println!("\n===== 阶段二：复制 文件C 到 01 与 03，并建 测试目录04 =====");
    phase_02_copy::step_27_confirm_copy_targets_absent(&ctx).await;
    phase_02_copy::step_28_copy_file_c_to_dir01(&ctx).await;
    phase_02_copy::step_29_copy_file_c_to_dir03(&ctx).await;
    phase_02_copy::step_30a_verify_copy_in_dir01(&ctx).await;
    phase_02_copy::step_30b_verify_copy_in_dir03(&ctx).await;
    phase_02_copy::step_31_verify_source_still_present(&ctx).await;
    phase_02_copy::step_32_verify_structures_after_copy(&ctx).await;
    phase_02_copy::step_33_confirm_move_target_absent(&ctx).await;
    phase_02_copy::step_34_mkcol_move_target(&ctx).await;
    phase_02_copy::step_35_verify_move_target_empty(&ctx).await;
    phase_02_copy::step_36_verify_root_after_mkcol04(&ctx).await;

    println!("\n===== 阶段三：移动 文件D 到 04，并删除 02 与 03 =====");
    phase_03_move::step_37a_confirm_move_source_present(&ctx).await;
    phase_03_move::step_37b_confirm_move_target_absent(&ctx).await;
    phase_03_move::step_38_move_file_d_to_dir04(&ctx).await;
    phase_03_move::step_39a_verify_moved_file_content(&ctx).await;
    phase_03_move::step_39b_verify_moved_file_length(&ctx).await;
    phase_03_move::step_40a_verify_move_source_gone_by_head(&ctx).await;
    phase_03_move::step_40b_verify_move_source_gone_by_propfind(&ctx).await;
    phase_03_move::step_41_verify_structures_after_move(&ctx).await;
    phase_03_move::step_42_record_shapes_before_delete(&ctx).await;
    phase_03_move::step_43_delete_copy_source(&ctx).await;
    phase_03_move::step_44_delete_copy_target(&ctx).await;
    phase_03_move::step_45_verify_dirs_deleted(&ctx).await;
    phase_03_move::step_46_verify_root_after_delete(&ctx).await;

    println!("\n===== 阶段四：终态核对、清理、与基线比对 =====");
    phase_04_cleanup::step_47a_verify_final_members_present(&ctx).await;
    phase_04_cleanup::step_47b_verify_final_file_contents(&ctx).await;
    phase_04_cleanup::step_47c_verify_final_properties(&ctx).await;
    phase_04_cleanup::step_47d_verify_final_move_target(&ctx).await;
    phase_04_cleanup::step_48_cleanup(&ctx).await;
    phase_04_cleanup::step_49a_verify_namespace_gone(&ctx).await;
    phase_04_cleanup::step_49b_verify_no_leftovers(&ctx).await;
    phase_04_cleanup::step_49c_verify_root_matches_baseline(&ctx).await;

    step("正向链路全部完成");
}

/// 负向用例：验证失败路径没有副作用。
#[tokio::test]
#[ignore = "对真实 WebDAV 服务器做写操作；只在人工执行时运行"]
async fn negative_cases() {
    let ctx = Ctx::prepare().await;
    negative::setup_fixtures(&ctx).await;

    println!("\n===== 负向用例 N1–N10 =====");
    negative::n01_mkcol_existing_collection_fails(&ctx).await;
    negative::n02_mkcol_missing_parent_fails(&ctx).await;
    negative::n03_copy_missing_source_fails(&ctx).await;
    negative::n04_move_missing_source_fails(&ctx).await;
    negative::n05_copy_without_overwrite_keeps_target(&ctx).await;
    negative::n06_move_without_overwrite_keeps_both(&ctx).await;
    negative::n07_delete_missing_path_fails(&ctx).await;
    negative::n08_delete_non_empty_with_zero_depth_fails(&ctx).await;
    negative::n09_move_collection_into_itself_fails(&ctx).await;
    negative::n10_delete_empty_collection_succeeds(&ctx).await;

    println!("\n===== 负向用例收尾：清理与基线核对 =====");
    phase_04_cleanup::step_48_cleanup(&ctx).await;
    phase_04_cleanup::step_49a_verify_namespace_gone(&ctx).await;
    phase_04_cleanup::step_49b_verify_no_leftovers(&ctx).await;
    phase_04_cleanup::step_49c_verify_root_matches_baseline(&ctx).await;

    step("负向用例全部完成");
}

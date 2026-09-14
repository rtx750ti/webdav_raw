//! 本次运行的上下文：认证、命名空间、基线快照与步骤打印。
//!
//! 上下文只负责「本次运行的环境」与「清理」，不放业务断言。

use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use webdav_core::WebdavAuth;

use super::dav_tree;
use crate::common::network_config;

/// 打印一行步骤标记。
///
/// 配合 `--nocapture` 形成完整执行轨迹，失败时能立刻看到停在哪个步骤。
pub fn step(label: &str) {
    println!("[workflow] {label}");
}

/// 本次运行的上下文。
pub struct Ctx {
    auth: WebdavAuth,
    /// 本次运行独占的**一次性工作区**名字（不带斜杠）。
    ///
    /// 形如 `__webdav_core_workflow_<时间戳>_<进程号>`。它建在服务器根目录下，
    /// 工作流的全部内容都放在它里面；跑完整个删掉，根目录恢复原样。
    ///
    /// # 为什么要多这一层工作区
    ///
    /// 直接把 `测试目录01` 建在服务器根目录下是错的：根目录是服务端的既有工作区，
    /// 实测本项目的验收服务器会直接返回 409 拒绝。自建一层工作区既能建得成功，
    /// 又让「跑完不留痕迹」这件事可以一次性做到（删掉工作区即可）。
    workspace: String,
    /// 开跑前根目录的成员快照，用于最后核对「服务端既有内容一项未少」。
    baseline: BTreeSet<String>,
}

impl Ctx {
    /// 读取环境变量、构造认证对象，并完成准备动作。
    ///
    /// 准备动作包含：清理本前缀的任何残留（含上次失败留下的），记录根目录基线。
    pub async fn prepare() -> Self {
        let config =
            network_config::read_network_config().expect("网络测试配置必须完整：需要 WEBDAV_URL / WEBDAV_ACCOUNT / WEBDAV_PASSWORD");
        let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
            .expect("网络测试地址应有效");

        step("阶段零：清理本前缀残留");
        let removed = cleanup_prefix(&auth).await;
        if removed.is_empty() {
            println!("[workflow]   无残留需要清理");
        } else {
            println!("[workflow]   已清理残留: {removed:?}");
        }

        step("阶段零：记录根目录基线");
        let baseline = dav_tree::list_root(&auth).await;
        let workspace = unique_workspace();
        println!("[workflow]   本次工作区: {workspace}/");
        println!("[workflow]   基线成员: {baseline:?}");

        Self {
            auth,
            workspace,
            baseline,
        }
    }

    /// 认证对象。
    pub fn auth(&self) -> &WebdavAuth {
        &self.auth
    }

    /// 本次工作区在服务器上的路径（不带首尾斜杠）。
    pub fn workspace(&self) -> &str {
        &self.workspace
    }

    /// 把工作区内的相对路径拼成服务器上的完整路径。
    ///
    /// 例如 `path("测试目录01")` → `<工作区>/测试目录01`。
    pub fn path(&self, suffix: &str) -> String {
        let suffix = suffix.trim_start_matches('/');
        if suffix.is_empty() {
            self.workspace.clone()
        } else {
            format!("{}/{suffix}", self.workspace)
        }
    }

    /// 开跑前记录的根目录基线。
    pub fn baseline(&self) -> &BTreeSet<String> {
        &self.baseline
    }
}

/// 生成本次运行独占的工作区名字。
fn unique_workspace() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时间应晚于 Unix 纪元")
        .as_nanos();
    let pid = std::process::id();

    format!("{}{nanos}_{pid}", dav_tree::NAMESPACE_PREFIX)
}

/// 删除根目录下所有以本方案前缀开头的顶层成员，返回被删除的名字。
///
/// 只认 [`dav_tree::NAMESPACE_PREFIX`] 前缀：绝不碰服务端既有内容。
/// 用递归删除（`Depth: infinity`）：本项目的验收服务器对集合的 `Depth: 0` 删除返回 400。
pub async fn cleanup_prefix(auth: &WebdavAuth) -> Vec<String> {
    let leftovers = dav_tree::list_root_with_prefix(auth, dav_tree::NAMESPACE_PREFIX).await;
    let mut removed = Vec::new();

    for name in leftovers {
        // 双保险：只删自己前缀下的东西。
        if !name.starts_with(dav_tree::NAMESPACE_PREFIX) {
            continue;
        }

        let status = dav_tree::delete_tree(auth, &name).await;
        if status.is_success() {
            removed.push(name);
        } else {
            eprintln!("[workflow] 清理未成功: {name}，状态码 {status}");
        }
    }

    removed
}

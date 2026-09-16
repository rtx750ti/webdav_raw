//! 真实下载落盘：把服务端的整份文件取下来写成本地文件，产物落在 crate 根目录。
//!
//! 只在启用 `network-test` 且手动执行 `--ignored` 时运行。落点由
//! `CARGO_MANIFEST_DIR` 决定，也就是本 crate 的根目录，跑完在旁边就能看到下载
//! 出来的文件，不必猜工作目录。
//!
//! 服务端只读：不创建、不修改、不删除任何远端资源。产物是一个本地文件，重复运行
//! 直接覆盖。
//!
//! 慢链路上整份下载可能耗时较久，且本库的 `Client` 没有设置读取超时，因此这里按
//! 步骤打印耗时，便于判断卡在哪一段。配合 `--nocapture` 才看得到实时输出。

use std::path::PathBuf;
use std::time::Instant;

use webdav_core::WebdavAuth;

use crate::common::network_config;

/// 受控环境中已存在且只读的测试文件路径。
const EXISTING_FILE: &str = "测试文件夹/fast-sync.exe";

/// 下载产物的名字，落在 crate 根目录。
const LOCAL_FILE_NAME: &str = "fast-sync.exe";

/// 整份下载写进 crate 根目录，字节数与响应声明的长度一致。
#[tokio::test]
#[ignore = "需要受控 WebDAV 环境和凭据"]
async fn download_to_disk_whole_file_writes_declared_bytes() {
    let trace = Instant::now();
    let step = |mark: &str| eprintln!("[{:>7.3}s] {mark}", trace.elapsed().as_secs_f64());

    let config = network_config::read_network_config().expect("网络测试配置必须完整");
    let auth = WebdavAuth::new(&config.account, &config.password, config.url.as_str())
        .expect("网络测试地址应有效");

    step(&format!("1/4 整份 GET：开始发送 path={EXISTING_FILE}"));
    let response = auth
        .get()
        .relative_path(EXISTING_FILE)
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect("下载请求应发送成功");
    step(&format!("2/4 已拿到响应头: status={}", response.status()));

    assert_eq!(response.status(), 200);

    let declared = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .expect("真实服务应给出 Content-Length");

    step(&format!("     响应声明长度: {declared} 字节"));

    let received = response.bytes().await.expect("响应体应可读取");
    step(&format!("3/4 响应体读完: {} 字节", received.len()));

    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(LOCAL_FILE_NAME);

    tokio::fs::write(&target, &received)
        .await
        .expect("下载产物应写入成功");

    let written = tokio::fs::read(&target).await.expect("下载产物应可读回");
    step(&format!("4/4 已落盘并读回: {}", target.display()));

    assert_eq!(
        received.len() as u64,
        declared,
        "收到的字节数应与声明的长度一致"
    );
    assert_eq!(
        written.len() as u64,
        declared,
        "落盘文件的长度应与声明的长度一致"
    );
    assert_eq!(
        written.as_slice(),
        received.as_ref(),
        "落盘内容应与收到的字节一致"
    );
}

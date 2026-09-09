use webdav_core::auth::WebdavAuth;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use reqwest::header::CONTENT_LENGTH;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ---------- 1. 从环境变量读取配置 ----------
    let username = env::var("WEBDAV_ACCOUNT")
        .expect("请设置环境变量 WEBDAV_ACCOUNT");
    let password = env::var("WEBDAV_PASSWORD")
        .expect("请设置环境变量 WEBDAV_PASSWORD");
    let base_url_str = env::var("WEBDAV_URL")
        .expect("请设置环境变量 WEBDAV_URL");

    let auth = WebdavAuth::new(&username, &password, &base_url_str)?;

    // ---------- 2. 指定要下载的文件（相对路径） ----------
    let remote_path = "测试文件夹/fast-sync.exe";
    let local_filename = "downloaded_fast-sync.exe";

    // ---------- 3. 获取 GetBuilder，设置路径并发送请求 ----------
    let response = auth
        .get()
        .relative_url(remote_path.to_string())
        .send()
        .await?;

    // ---------- 4. 检查状态码 ----------
    if !response.status().is_success() {
        eprintln!("下载失败，状态码: {}", response.status());
        return Ok(());
    }

    // ---------- 5. 获取文件大小（用于进度显示） ----------
    let total_size = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    println!("文件大小: {} 字节", total_size);

    // ---------- 6. 流式写入本地文件 ----------
    let mut file = tokio::fs::File::create(local_filename).await?;
    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let percent = (downloaded as f64 / total_size as f64) * 100.0;
            println!("进度: {:.2}%", percent);
        }
    }

    println!("文件下载完成: {}", local_filename);
    Ok(())
}
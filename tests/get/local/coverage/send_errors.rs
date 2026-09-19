//! 覆盖率补点：GET 发送阶段的错误分支。
//!
//! 对应 `cargo llvm-cov` 报告（`docs/coverage/html`）中的红点：
//! `src/get/builder.rs::send` 的 `self.client.execute(request).await?` 错误传播区域。
//!
//! `tests/get/local/unit/whitebox/errors.rs` 只固定了本库主动返回的四个变体，没有
//! 构造底层连接失败。这里用一个没有监听者的回环端口触发，命中后 `Request` 变体不再
//! 是「无法在本地构造」的分支。

use webdav_raw::{Client, GetBuilder, GetError, Url};

/// 端口 1 上没有监听者，`send()` 把底层连接错误原样交出。
#[tokio::test]
async fn send_returns_request_error_when_server_is_unreachable() {
    let target = Url::parse("http://127.0.0.1:1/dav/").expect("地址必须合法");

    let error = GetBuilder::new(Client::new(), target)
        .relative_path("missing.bin")
        .expect("合法相对路径应被接受")
        .send()
        .await
        .expect_err("不可达地址应报请求错误");

    assert!(
        matches!(error, GetError::Request(_)),
        "应为 Request，实际: {error:?}"
    );
}

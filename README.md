# webdav_raw

面向 Rust 的底层 WebDAV 协议库。

**每个 WebDAV Builder 既可以生成标准 `reqwest::Request`，交给上层检查、修改或统一调度，也可以直接发送请求并取得原始 `reqwest::Response`。**

PROPFIND 返回的 XML 可以解析成 `MultiStatus` 协议模型，并在 WebDAV XML、Rust 类型和 JSON 之间转换。

`webdav_raw` 适合用作 WebDAV 客户端、同步工具、CMP 工具、网盘应用和 AI 文件 Agent 的协议基础层。

## 安装

在使用方项目的 `Cargo.toml` 中添加：

```toml
[dependencies]
webdav_raw = "0.1.0"
```

## 核心能力

| 能力 | 说明 |
| --- | --- |
| 请求构造 | 将 WebDAV 参数构造成可检查的 `reqwest::Request` |
| 原始响应 | 保留 HTTP 状态码、响应头和原始响应体 |
| 协议模型 | 公开 `MultiStatus`、`ResourceResponse`、`PropStat` 和 `Prop` |
| XML 转换 | 支持 WebDAV XML 与 `MultiStatus` 双向转换 |
| JSON 转换 | 协议模型支持 Serde，可以序列化和反序列化 JSON |
| 自定义执行 | 可以使用内置认证客户端，也可以传入自己的 `reqwest::Client` |
| 请求控制 | 支持相对路径、完整 URL、自定义请求头、Depth 和 Overwrite |
| 上传模型 | 支持内存字节、异步文件句柄和单个业务分片 |

## 两种使用层次

### 直接发送

`WebdavAuth` 管理 Basic Auth、根地址和底层 HTTP Client，并提供各方法的 Builder：

```rust
let auth = webdav_raw::WebdavAuth::new(
    "username",
    "password",
    "https://example.com/dav/",
)?;

let response = auth
    .get()
    .relative_path("Documents/report.pdf")?
    .send()
    .await?;
```

### 只构造请求

每个 Builder 都提供 `build`。上层可以先检查请求，再交给自己的执行流程：

```rust
let request = auth
    .get()
    .relative_path("Documents/report.pdf")?
    .range(0, 1023)?
    .build()?;

println!("{} {}", request.method(), request.url());

let response = auth.get_client().execute(request).await?;
```

这种方式适合统一重试、限流、日志、任务编排、中间件和 AI 操作审批。

## 完整的 PROPFIND 案例

下面的程序只发送一次 PROPFIND 请求，同时保留原始响应体，并完成 XML、`MultiStatus` 和 JSON 的转换。

### 示例依赖

在使用方项目的 `Cargo.toml` 中配置：

```toml
[dependencies]
webdav_raw = "0.1.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde_json = "1"
```

### 示例程序

```rust
use std::{error::Error, io};

use webdav_raw::{Depth, FindProp, MultiStatus, StatusCode, WebdavAuth};

/// 查询一层目录，并演示原始报文、协议模型、XML 和 JSON 的转换。
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let auth = WebdavAuth::new(
        "username",
        "password",
        "https://example.com/dav/",
    )?;

    let builder = auth
        .propfind()
        // core 直接使用 URL 解析规则：
        // "Documents/" 相对 WebDAV 根地址解析。
        // "./Documents/" 与上面的目标相同。
        // "../Documents/" 会解析到根地址的上一级。
        // "/Documents/" 会从域名根路径开始，覆盖 WebDAV 路径前缀。
        // 需要业务路径检查时，由上层客户端或应用负责。
        .path("./Documents/")
        .depth(Depth::One)
        .props([
            FindProp::Resourcetype,
            FindProp::Getcontentlength,
            FindProp::Getcontenttype,
            FindProp::Getlastmodified,
            FindProp::Creationdate,
            FindProp::Getetag,
        ]);

    // build() 只生成请求，不发送。可以检查方法、URL、请求头和 XML 请求体。
    let request = builder.build()?;
    println!("{} {}", request.method(), request.url());

    // send() 返回原始 reqwest::Response。
    let response = builder.send().await?;
    let status = response.status();
    let headers = response.headers().clone();
    let raw_body = response.bytes().await?;

    println!("HTTP 状态: {}", status.to_string());
    println!("响应头数量: {}", headers.len());

    if status != StatusCode::MULTI_STATUS {
        return Err(io::Error::other(format!(
            "期望 207 Multi-Status，实际收到 {status}"
        ))
        .into());
    }

    // raw_body 始终保留服务器返回的原始字节。
    let raw_xml = std::str::from_utf8(&raw_body)?;
    println!("原始 WebDAV XML:\n{raw_xml}");

    // XML -> MultiStatus。
    let multistatus = MultiStatus::from_str(raw_xml)?;

    // MultiStatus -> JSON -> MultiStatus。
    let json = serde_json::to_string_pretty(&multistatus)?;
    let restored: MultiStatus = serde_json::from_str(&json)?;
    println!("JSON:\n{json}");

    // MultiStatus -> WebDAV XML。
    let rebuilt_xml = restored.serialize()?;
    println!("重新生成的 XML:\n{rebuilt_xml}");

    Ok(())
}
```

XML 重新序列化以协议数据语义为准，命名空间前缀和排版可能与服务器原文不同。需要逐字节保留报文时，应保存 `Response::bytes()` 的结果。

## 支持的方法

| WebDAV 方法 | Builder | 主要能力 |
| --- | --- | --- |
| `PROPFIND` | `PropFindBuilder` | 属性选择、Depth、原始响应、`MultiStatus` |
| `GET` | `GetBuilder` | 完整下载、Range、自定义请求头 |
| `PUT` | `PutBuilder` | 内存、文件和业务分片上传 |
| `HEAD` | `HeadBuilder` | 资源状态和响应头 |
| `OPTIONS` | `OptionsBuilder` | 原始响应和结构化 DAV 能力 |
| `MKCOL` | `MkcolBuilder` | 创建集合、自定义请求体 |
| `DELETE` | `DeleteBuilder` | 删除资源、设置 Depth |
| `COPY` | `CopyBuilder` | 源目标、Depth、Overwrite、`MultiStatus` |
| `MOVE` | `MoveBuilder` | 移动或重命名、Overwrite、`MultiStatus` |

## Builder 的统一使用方式

多数 Builder 的 `build` 会消费当前实例，因此构造请求和直接发送通常从两个独立 Builder 开始：

```rust
let request = auth
    .head()
    .target_path("Documents/report.pdf")?
    .build()?;

let response = auth
    .head()
    .target_path("Documents/report.pdf")?
    .send()
    .await?;
```

PUT 需要在构建请求时读取文件源长度，因此使用异步的 `build().await?`。其他 Builder 使用同步的 `build()`。

PROPFIND、COPY 和 MOVE 还提供 `send_and_deserialize`，用于把 `207 Multi-Status` 直接解析成协议模型。

HTTP 状态码由调用方处理。普通 `send` 会保留 `207`、`404`、`409`、`412`、`423` 和 `507` 等服务端结果。

`webdav_raw` 负责提供透明、可组合的协议能力，上层应用可以继续定义路径权限、重试策略、缓存、同步状态和用户交互。

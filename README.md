# 这是什么

`webdav_raw` 是 Rust 实现的 WebDAV 协议库，它的 Builder 既可以直接发送请求获取原始响应，也能只构造请求交给上层校验、修改与调度；PROPFIND 返回的 XML 可解析为`MultiStatus`协议模型，支持 WebDAV XML、Rust 类型和 JSON 之间互相转换，适合作为 WebDAV 客户端、同步工具、CMP 工具、网盘应用和 AI 文件 Agent 的协议基础层。

库完整实现各类 WebDAV 方法，提供请求构造、自定义 HTTP 客户端、Range 下载、多形式上传等能力，同时配套大规模高覆盖率测试，覆盖完整工作流与各类异常失败场景，保障操作前后远端数据安全，上层可自行实现重试、权限、缓存等业务逻辑。

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

## 测试与覆盖率

项目包含一条针对真实 WebDAV 服务器的 49 步完整工作流：从查询能力、创建目录和上传文件开始，继续验证下载、复制、移动和删除，最后确认临时资源全部清除，服务器根目录恢复到运行前的状态。

测试还包含 10 个真实服务器失败场景，重点验证操作失败后的数据安全，例如禁止覆盖时目标内容保持不变、移动失败时源文件仍然存在、删除失败时目录内容没有丢失。

### 测试规模

统计工具：`tokei 12.1.2`

| 目录 | 文件 | Code | 注释 | 空行 | 总行 |
| --- | ---: | ---: | ---: | ---: | ---: |
| `src` | 34 | 1667 | 1156 | 783 | 3621 |
| `tests` | 243 | 12346 | 1429 | 2420 | 16195 |

测试代码量约为源码的 7.4 倍（按有效代码），共包含 525 个测试函数，其中 30 个为需要 `network-test` 的真实服务器测试。

### 当前覆盖率

覆盖率生成时间：2026-09-19 16:28  
统计工具：`llvm-cov 20.1.7-rust-1.89.0-stable`

| 指标 | 覆盖率 |
| --- | ---: |
| 函数覆盖率 | **100.00%（167/167）** |
| 行覆盖率 | **99.89%（945/946）** |
| 区域覆盖率 | **97.63%（1481/1517）** |

### 重点测试内容

- **完整 WebDAV 工作流**：在独立临时目录中连续执行 PROPFIND、OPTIONS、MKCOL、PUT、GET、HEAD、COPY、MOVE 和 DELETE；每一步都会检查远端目录结构、文件内容和资源属性，结束后还会与运行前的目录基线进行比对。
- **上传后的数据真实性**：先上传内存字节，再用文件句柄覆盖同一个远端资源；每次上传后使用 GET 逐字节读回，并使用 PROPFIND 核对服务端记录的文件长度，覆盖后还会检查长度和 ETag 的变化。
- **Range 下载协议**：检查实际发出的 `Range` 请求头、206 响应中的 `Content-Range` 和分片长度；服务端忽略 Range 并返回 200 时，也会检查完整响应能否原样交给调用方。
- **文件落盘结果**：从真实服务器下载文件后，核对响应声明长度、接收字节数、落盘文件长度和文件内容完全一致。
- **复制与移动语义**：COPY 成功后源文件仍然存在，MOVE 成功后源文件消失且目标内容一致；禁止覆盖时会核对目标内容未变化，MOVE 失败时还会确认源文件仍然存在。
- **PROPFIND 兼容性**：使用真实响应 XML 检查解析、重新序列化和再次解析后的数据语义；同时覆盖 `propname` 空元素、`xsi:nil` 属性、集合类型及缺失属性。
- **失败后的远端状态**：覆盖重复创建目录、父目录缺失、源文件缺失、目标冲突、非空目录删除失败和目录移动到自身等场景，并检查失败操作没有破坏已有数据。
- **认证与敏感信息**：检查 Basic Auth 请求、错误凭据响应以及调试输出，确保账号、密码和 Authorization 内容不会出现在错误信息中。

覆盖率数据来自本地自动化测试。真实服务器测试默认忽略，需要显式启用 `network-test`，并且只操作测试自己创建的临时资源。

## 快速开始

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

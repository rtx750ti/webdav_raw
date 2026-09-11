pub mod auth;
pub mod copy;
pub mod delete;
pub mod propfind;
pub mod get;
pub mod head;
pub mod mkcol;
pub mod options;
pub mod proppatch;
pub mod put;

/// `move` 是 Rust 关键字，使用原始标识符导出 WebDAV MOVE 模块。
pub mod r#move;

pub use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
/// 集中导出，避免版本问题以及让开发者多次下载依赖
pub use reqwest::{Body, Client, Request, Response, StatusCode};
pub use url::{ParseError, Url};

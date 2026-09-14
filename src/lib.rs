//! # webdav-core
//!
//! WebDAV 客户端核心。提供认证对象，以及 GET、PROPFIND、PUT 三个领域的请求
//! 构建与原样响应透传。
//!
//! # 模块规范
//!
//! 实现模块全部私有，对外只暴露本文件中的再导出。因此调用方只需要写
//! `webdav_core::PutBuilder` 这类短路径，不必了解内部的文件分层。
//!
//! 依赖类型（`reqwest`、`url`）也在这里集中转发，调用方不必自己声明同样的依赖
//! 版本，避免版本错配。

// ── 已实现领域：模块私有，只经下方再导出对外 ──
mod auth;
mod copy;
mod delete;
mod get;
mod head;
mod mkcol;
mod options;
mod propfind;
mod put;

/// `move` 是 Rust 关键字，使用原始标识符声明 WebDAV MOVE 模块。
///
/// 模块名不对外暴露：调用方用的是 `WebdavAuth::mv()`。
mod r#move;

// ── 认证 ──
pub use auth::{WebdavAuth, WebdavAuthError};

// ── GET ──
pub use get::builder::{GetBuilder, GetError};

// ── DELETE ──
pub use delete::builder::{DeleteBuilder, DeleteDepth, DeleteError};

// ── COPY ──
pub use copy::builder::{CopyBuilder, CopyDepth, CopyError};
pub use copy::overwrite::Overwrite;

// ── HEAD ──
pub use head::builder::{HeadBuilder, HeadError};

// ── MKCOL ──
pub use mkcol::builder::{MkcolBuilder, MkcolError};

// ── OPTIONS ──
pub use options::builder::{OptionsBuilder, OptionsCapabilities, OptionsError};

// ── MOVE ──
pub use r#move::builder::{MoveBuilder, MoveDepth, MoveError};

// ── PUT ──
pub use put::builder::{PutBuilder, PutError};
pub use put::put_body::file_handle::FileHandle;
pub use put::put_body::u8_bytes::{U8Bytes, U8BytesError, U8Metadata};
pub use put::put_body::u8_bytes_chunk::{U8BytesChunk, U8BytesChunkError};
pub use put::put_body::u8_bytes_data::{BytesDataId, U8BytesData, U8BytesDataError};
pub use put::put_body::{PutBody, PutData};

// ── PROPFIND ──
pub use propfind::builder::{Depth, PropFindBuilder, PropFindError};
pub use propfind::find_props::{FindProp, PropFindSelector};
pub use propfind::raw_xml::multistatus::MultiStatus;
pub use propfind::raw_xml::prop::Prop;
pub use propfind::raw_xml::propstat::PropStat;
pub use propfind::raw_xml::resourcetype::{EmptyElement, ResourceType};

/// XML `<response>` 元素。
///
/// 与 HTTP 的 [`Response`] 同名，因此在这里用别名导出以示区分。
pub use propfind::raw_xml::response::Response as ResourceResponse;

// ── 依赖类型集中转发，避免版本错配 ──
pub use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
pub use reqwest::{Body, Client, Request, Response, StatusCode};
pub use url::{ParseError, Url};

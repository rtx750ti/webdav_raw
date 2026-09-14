//! 工作流的测试数据：目录名、文件名与文件内容。
//!
//! 所有路径都是**相对于本次运行的一次性工作区**（即命名空间）的，不含工作区本身。
//! 例如 `dir::ROOT` 是 `测试目录01`，它在服务器上的完整路径由
//! `Ctx::path("测试目录01")` 拼成 `<命名空间>/测试目录01`。
//!
//! 放在单独模块是为了让步骤函数只关心「做什么」，而不是「叫什么名字、内容是什么」。

/// 工作流里会用到的目录（相对于本次运行的工作区）。
pub mod dir {
    /// 主测试目录。
    pub const ROOT: &str = "测试目录01";
    /// 被复制与删除的子目录。
    pub const COPY_SOURCE: &str = "测试目录01/测试目录02";
    /// 同时作为复制目标、移动源所在目录、删除对象。
    pub const COPY_TARGET: &str = "测试目录01/测试目录03";
    /// 移动目标目录。
    pub const MOVE_TARGET: &str = "测试目录01/测试目录04";
    /// 负向用例专用的空集合（用于验证「空集合可以删掉」）。
    pub const NEGATIVE_EMPTY: &str = "测试目录01/负向空目录";
    /// 负向用例用的、父目录不存在的路径（用于验证父不存在时被拒绝）。
    pub const NEGATIVE_MISSING_PARENT: &str = "测试目录01/不存在的父目录/子目录";
}

/// 工作流里会用到的文件（相对于本次运行的工作区）。
pub mod file {
    /// 01 内的第一个文件，走内存源。
    pub const A: &str = "测试目录01/文件A.txt";
    /// 01 内的第二个文件，走文件句柄源。
    pub const B: &str = "测试目录01/文件B.txt";
    /// 02 内的文件，复制的源。
    pub const C: &str = "测试目录01/测试目录02/文件C.txt";
    /// 03 内的文件，移动的源。
    pub const D: &str = "测试目录01/测试目录03/文件D.txt";
    /// 02 的文件复制到 01 之后的位置。
    pub const C_IN_01: &str = "测试目录01/文件C.txt";
    /// 02 的文件复制到 03 之后的位置。
    pub const C_IN_03: &str = "测试目录01/测试目录03/文件C.txt";
    /// 03 的文件移动到 04 之后的位置。
    pub const D_IN_04: &str = "测试目录01/测试目录04/文件D.txt";
}

/// 各文件的写入内容。每个文件内容不同，便于区分「复制/移动对不对」。
pub mod content {
    use super::file;

    /// 01/文件A.txt 的内容。
    pub fn a() -> Vec<u8> {
        b"workflow file A: created through in-memory source".to_vec()
    }

    /// 01/文件B.txt 的内容。
    pub fn b() -> Vec<u8> {
        b"workflow file B: created through file-handle source".to_vec()
    }

    /// 02/文件C.txt 的内容。
    pub fn c() -> Vec<u8> {
        b"workflow file C: the source of COPY".to_vec()
    }

    /// 03/文件D.txt 的内容。
    pub fn d() -> Vec<u8> {
        b"workflow file D: the source of MOVE".to_vec()
    }

    /// 按路径给出期望内容；路径不在表内时返回 `None`。
    ///
    /// 用于终态核对：遍历实际存在的成员，逐个比对内容。
    pub fn expected_for(path_suffix: &str) -> Option<Vec<u8>> {
        match path_suffix {
            file::A => Some(a()),
            file::B => Some(b()),
            file::C => Some(c()),
            file::C_IN_01 => Some(c()),
            file::C_IN_03 => Some(c()),
            file::D => Some(d()),
            file::D_IN_04 => Some(d()),
            _ => None,
        }
    }
}

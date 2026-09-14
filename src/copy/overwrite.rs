//! `Overwrite` 请求头的取值类型。
//!
//! 单独一个文件是因为 COPY 与 MOVE 共用这一个协议值：MOVE 的语义是
//! 「COPY + 删源」，两者对「目标已存在时怎么办」的回答完全一致。

/// `Overwrite` 请求头的取值。
///
/// RFC 4918 §10.6 规定该头的默认值是 `T`，因此本库**默认不发送这个头**：
/// 不发等于告诉服务端「按默认来」，避免发一个服务端本来就能自己推断的头。
/// 只有显式设为 [`Overwrite::False`] 时才写出 `Overwrite: F`。
///
/// 这里用枚举而不是 `bool`，是为了让「按默认处理」与「明确不覆盖」在类型上
/// 区分开，也为了给将来的第三个取值留位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overwrite {
    /// 允许覆盖目标；对应 RFC 的默认值，本库不发送该请求头。
    #[default]
    True,

    /// 目标已存在时让请求失败（服务端通常回 412 Precondition Failed）。
    False,
}

impl Overwrite {
    /// 转换成 `Overwrite` 请求头中的字符串。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::True => "T",
            Self::False => "F",
        }
    }
}

//! Xiao 词法 Token、语法树与前端解析器的稳定门面。
//!
//! 具体职责已拆分到诊断、Token、词法器、AST、解析器和选择器模块。门面只负责
//! 模块装配与公开重导出，避免调用方依赖内部文件布局，也禁止语义层反向耦合。

/// AST、源码节点身份和声明数据结构。
mod ast;
/// 词法与解析阶段的稳定诊断编号。
mod diagnostics;
/// UTF-8 源码词法扫描器。
mod lexer;
/// 可恢复的 P0/P1/P2 语法解析器。
mod parser;
/// 索引路径和选择器数据结构。
mod selectors;
/// Token、关键字和字面量类别。
mod token;

/// 重新导出 AST、声明和节点索引类型。
pub use ast::{
    AssignmentOperator, BinaryOperator, Expression, LiteralKind, Name, NodeId, NodeIndex, Program,
    ScalarType, Statement, UnaryOperator,
};
/// 重新导出词法与解析诊断编号。
pub use diagnostics::*;
/// 重新导出词法结果和词法器。
pub use lexer::{LexResult, Lexer};
/// 重新导出解析结果、解析器和便捷解析函数。
pub use parser::{ParseDiagnostic, ParseResult, Parser, parse};
/// 重新导出选择器路径和选择项类型。
pub use selectors::{IndexPath, PathSegment, RandomMode, Selector, SelectorItem};
/// 重新导出 Token、关键字和词法诊断别名。
pub use token::{KeywordKind, LexDiagnostic, Token, TokenKind};

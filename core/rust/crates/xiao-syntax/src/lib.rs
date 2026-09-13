//! Xiao 词法 Token、语法树与前端解析器的稳定门面。
//!
//! 具体职责已拆分到诊断、Token、词法器、AST、解析器和选择器模块。门面只负责
//! 模块装配与公开重导出，避免调用方依赖内部文件布局，也禁止语义层反向耦合。

mod ast;
mod diagnostics;
mod lexer;
mod parser;
mod selectors;
mod token;

pub use ast::{
    AssignmentOperator, BinaryOperator, Expression, LiteralKind, Name, NodeId, NodeIndex, Program,
    ScalarType, Statement, UnaryOperator,
};
pub use diagnostics::*;
pub use lexer::{LexResult, Lexer};
pub use parser::{ParseDiagnostic, ParseResult, Parser, parse};
pub use selectors::{IndexPath, PathSegment, RandomMode, Selector, SelectorItem};
pub use token::{KeywordKind, LexDiagnostic, Token, TokenKind};

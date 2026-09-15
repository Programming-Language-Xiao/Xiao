//! 05-A `import` 和 `from ... import ...` 的语法解析。
//!
//! 该模块是 `Parser` 的实现扩展，只消费 Token 并构造公开 AST。它不访问
//! 文件系统，也不判断模块是否存在；这些工作属于 `xiao-modules`。

use crate::ast::{Name, Statement};
use crate::diagnostics::{
    INVALID_IMPORT_ALIAS_CODE, INVALID_IMPORT_PATH_CODE, INVALID_IMPORT_TARGET_CODE,
    UNSUPPORTED_IMPORT_FORM_CODE,
};
use crate::imports::{ImportPath, ImportStatement, ModuleImport, SelectedImport};
use crate::parser::Parser;
use crate::token::{KeywordKind, TokenKind};
use xiao_source::SourceSpan;

impl<'source> Parser<'source> {
    /// 解析一条模块导入语句。
    pub(super) fn parse_import_statement(
        &mut self,
        leading_docs: Vec<SourceSpan>,
    ) -> Option<Statement> {
        let keyword = self.bump();
        let result = if keyword.kind() == TokenKind::Keyword(KeywordKind::Import) {
            self.parse_module_imports(keyword.span())
        } else {
            self.parse_from_imports(keyword.span())
        };

        let Some((import, end)) = result else {
            self.synchronize_to_boundary();
            return None;
        };
        if !self.is_statement_boundary_current() {
            self.push_error(
                INVALID_IMPORT_TARGET_CODE,
                "x05.parse.import_tail",
                self.current().span(),
                "导入语句后存在未预期内容".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        self.consume_newline();
        Some(Statement::Import {
            span: self.source_span(keyword.span().start(), end),
            import,
            leading_docs,
        })
    }

    /// 解析 `import a.b [as c], d.e` 列表。
    fn parse_module_imports(
        &mut self,
        keyword_span: SourceSpan,
    ) -> Option<(ImportStatement, usize)> {
        let mut imports = Vec::new();
        loop {
            let item = self.parse_module_import_item(keyword_span)?;
            imports.push(item);
            if !self.at(TokenKind::Comma) {
                break;
            }
            self.bump();
            if self.is_statement_boundary_current() {
                self.push_error(
                    INVALID_IMPORT_TARGET_CODE,
                    "x05.parse.trailing_import_comma",
                    self.current().span(),
                    "导入列表末尾不能有逗号".to_string(),
                );
                return None;
            }
        }
        let end = imports.last()?.span.end();
        Some((
            ImportStatement::Modules {
                imports,
                span: self.source_span(keyword_span.start(), end),
            },
            end,
        ))
    }

    /// 解析 `from a.b import X [as Y], Z`。
    fn parse_from_imports(&mut self, keyword_span: SourceSpan) -> Option<(ImportStatement, usize)> {
        let module = self.parse_import_path(keyword_span)?;
        if !self.at(TokenKind::Keyword(KeywordKind::Import)) {
            self.push_error(
                INVALID_IMPORT_TARGET_CODE,
                "x05.parse.missing_from_import",
                self.current().span(),
                "from 路径后必须是 import".to_string(),
            );
            return None;
        }
        self.bump();
        let mut imports = Vec::new();
        loop {
            let item = self.parse_selected_import_item(module.span())?;
            imports.push(item);
            if !self.at(TokenKind::Comma) {
                break;
            }
            self.bump();
            if self.is_statement_boundary_current() {
                self.push_error(
                    INVALID_IMPORT_TARGET_CODE,
                    "x05.parse.trailing_import_comma",
                    self.current().span(),
                    "导入列表末尾不能有逗号".to_string(),
                );
                return None;
            }
        }
        let end = imports.last()?.span.end();
        Some((
            ImportStatement::From {
                module,
                imports,
                span: self.source_span(keyword_span.start(), end),
            },
            end,
        ))
    }

    /// 解析一个模块路径和可选别名。
    fn parse_module_import_item(&mut self, opener: SourceSpan) -> Option<ModuleImport> {
        let path = self.parse_import_path(opener)?;
        let path_start = path.span().start();
        let alias = self.parse_optional_alias(path.span())?;
        let end = alias.map_or(path.span().end(), |name| name.span.end());
        Some(ModuleImport {
            path,
            alias,
            span: self.source_span(path_start, end),
        })
    }

    /// 解析一个选择导入名称和可选别名。
    fn parse_selected_import_item(&mut self, opener: SourceSpan) -> Option<SelectedImport> {
        if self.at(TokenKind::Star) {
            let token = self.bump();
            self.push_error(
                UNSUPPORTED_IMPORT_FORM_CODE,
                "x05.parse.wildcard_import",
                token.span(),
                "当前阶段不支持通配导入".to_string(),
            );
            return None;
        }
        if !is_import_name_token(self.current().kind()) {
            self.push_error(
                INVALID_IMPORT_TARGET_CODE,
                "x05.parse.missing_import_name",
                self.current().span(),
                "from 导入后必须是名称".to_string(),
            );
            return None;
        }
        let name = self.parse_name();
        let alias = self.parse_optional_alias(name.span())?;
        let end = alias.map_or(name.span().end(), |value| value.span.end());
        let _ = opener;
        Some(SelectedImport {
            name,
            alias,
            span: self.source_span(name.span().start(), end),
        })
    }

    /// 解析 `as alias`，并拒绝关键字作为未加反引号的别名。
    fn parse_optional_alias(&mut self, opener: SourceSpan) -> Option<Option<Name>> {
        if !self.at(TokenKind::Keyword(KeywordKind::As)) {
            return Some(None);
        }
        self.bump();
        if !is_import_name_token(self.current().kind()) {
            self.push_error(
                INVALID_IMPORT_ALIAS_CODE,
                "x05.parse.missing_import_alias",
                self.current().span(),
                "as 后必须是名称别名".to_string(),
            );
            return None;
        }
        let alias = self.parse_name();
        if alias.span == opener {
            self.push_error(
                INVALID_IMPORT_ALIAS_CODE,
                "x05.parse.invalid_import_alias",
                alias.span,
                "导入别名不能与导入目标相同".to_string(),
            );
            return None;
        }
        Some(Some(alias))
    }

    /// 解析由点号分隔的绝对 ASCII 模块路径。
    fn parse_import_path(&mut self, opener: SourceSpan) -> Option<ImportPath> {
        if !is_module_segment_token(self.current().kind()) {
            let token = self.current();
            let code = if token.kind() == TokenKind::Dot {
                UNSUPPORTED_IMPORT_FORM_CODE
            } else {
                INVALID_IMPORT_PATH_CODE
            };
            let message_id = if token.kind() == TokenKind::Dot {
                "x05.parse.relative_import"
            } else {
                "x05.parse.missing_import_path"
            };
            self.push_error(
                code,
                message_id,
                token.span(),
                "导入路径必须从 ASCII 模块名称开始".to_string(),
            );
            return None;
        }
        let first = self.parse_name();
        let mut segments = vec![first];
        while self.at(TokenKind::Dot) {
            let dot = self.bump();
            if !is_module_segment_token(self.current().kind()) {
                self.push_error(
                    INVALID_IMPORT_PATH_CODE,
                    "x05.parse.missing_import_segment",
                    dot.span(),
                    "点号后必须是 ASCII 模块名称".to_string(),
                );
                return None;
            }
            segments.push(self.parse_name());
        }
        let end = segments.last()?.span.end();
        let _ = opener;
        Some(ImportPath::new(
            segments,
            self.source_span(first.span.start(), end),
        ))
    }

    /// 判断当前 Token 是否是语句边界。
    fn is_statement_boundary_current(&self) -> bool {
        matches!(
            self.current().kind(),
            TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof
        )
    }
}

/// 模块路径段只能使用非关键字 ASCII 标识符。
fn is_module_segment_token(kind: TokenKind) -> bool {
    kind == TokenKind::Identifier
}

/// 选择名称和别名可以使用普通或反引号名称。
fn is_import_name_token(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Identifier | TokenKind::BacktickIdentifier)
}

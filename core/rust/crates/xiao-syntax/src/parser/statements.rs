//! P0/P1 语句、表头、函数和控制流解析实现。
//!
//! 该模块是 `Parser` 的实现扩展，只消费 Token 并构造公开 AST；通用 Token 游标、
//! 源码区间和错误恢复接口由父级 `parser.rs` 提供。

use std::collections::BTreeSet;

use crate::ast::{
    AssignmentOperator, CatchClause, ElifBranch, Expression, FunctionParameter,
    FunctionParameterKind, FunctionTypeAnnotation, Name, ScalarType, Statement, TableKind,
};
use crate::diagnostics::*;
use crate::parser::{
    Parser, assignment_operator, is_assignable_syntax, is_declaration_name_token,
    is_scalar_type_token, is_statement_boundary, is_table_member_statement,
};
use crate::token::{KeywordKind, Token, TokenKind};
use xiao_source::SourceSpan;

impl<'source> Parser<'source> {
    /// 解析一条 P1 顶层语句，并保留 P0 简单赋值的兼容形状。
    pub(super) fn parse_statement(&mut self, leading_docs: Vec<SourceSpan>) -> Option<Statement> {
        match self.current().kind() {
            TokenKind::Keyword(KeywordKind::Import | KeywordKind::From) => {
                return self.parse_import_statement(leading_docs);
            }
            TokenKind::Keyword(KeywordKind::Def) => {
                return self.parse_function_statement(leading_docs);
            }
            TokenKind::Keyword(KeywordKind::If) => {
                return self.parse_if_statement(leading_docs);
            }
            TokenKind::Keyword(KeywordKind::For) => {
                return self.parse_for_statement(leading_docs);
            }
            TokenKind::Keyword(KeywordKind::While) => {
                return self.parse_while_statement(leading_docs);
            }
            TokenKind::Keyword(KeywordKind::Return) => {
                return self.parse_return_statement(leading_docs);
            }
            TokenKind::Keyword(KeywordKind::Break) => {
                return self.parse_loop_control_statement(leading_docs, true);
            }
            TokenKind::Keyword(KeywordKind::Continue) => {
                return self.parse_loop_control_statement(leading_docs, false);
            }
            TokenKind::Keyword(KeywordKind::Try) => {
                return self.parse_try_statement(leading_docs);
            }
            TokenKind::Keyword(KeywordKind::Raise) => {
                return self.parse_raise_statement(leading_docs);
            }
            TokenKind::LeftBracket if self.starts_table_header() => {
                if self.block_context_depth != 0 {
                    let span = self.current().span();
                    self.push_error(
                        INVALID_TABLE_HEADER_CODE,
                        "x05.parse.nested_table",
                        span,
                        "表声明只能出现在文件顶层".to_string(),
                    );
                    self.synchronize_to_boundary();
                    return None;
                }
                return self.parse_table_statement(leading_docs);
            }
            _ => {}
        }
        if self.starts_declaration() {
            return self.parse_declaration_statement(leading_docs);
        }
        let first = self.current();
        let expression_diagnostics = self.diagnostics.len();
        let expression = match self.parse_expression() {
            Some(expression) => expression,
            None => {
                // 前缀解析器可能已经给出更具体的缺少操作数/分组诊断；
                // 只有在没有任何新诊断时才补充通用的起始 Token 错误。
                if first.kind() != TokenKind::Invalid
                    && self.diagnostics.len() == expression_diagnostics
                {
                    self.push_error(
                        INVALID_EXPRESSION_CODE,
                        "x01.parse.invalid_expression",
                        first.span(),
                        "该 Token 不能开始 P1 表达式".to_string(),
                    );
                }
                self.synchronize_to_boundary();
                return None;
            }
        };

        if let Some(operator) = assignment_operator(self.current().kind()) {
            let operator_token = self.bump();
            if is_statement_boundary(self.current().kind()) {
                self.push_error(
                    MISSING_ASSIGNMENT_VALUE_CODE,
                    "x01.parse.missing_assignment_value",
                    operator_token.span(),
                    "赋值运算符右侧缺少表达式".to_string(),
                );
                self.synchronize_to_boundary();
                return None;
            }
            let value_diagnostics = self.diagnostics.len();
            let value = match self.parse_expression() {
                Some(value) => value,
                None => {
                    if self.current().kind() != TokenKind::Invalid
                        && self.diagnostics.len() == value_diagnostics
                    {
                        self.push_error(
                            INVALID_ASSIGNMENT_CODE,
                            "x01.parse.invalid_assignment",
                            self.current().span(),
                            "赋值运算符右侧不是有效表达式".to_string(),
                        );
                    }
                    self.synchronize_to_boundary();
                    return None;
                }
            };
            if !is_statement_boundary(self.current().kind()) {
                self.push_error(
                    INVALID_ASSIGNMENT_CODE,
                    "x01.parse.invalid_assignment",
                    self.current().span(),
                    "赋值表达式后存在未预期内容".to_string(),
                );
                self.synchronize_to_boundary();
                return None;
            }
            if !is_assignable_syntax(&expression) {
                self.push_error(
                    INVALID_ASSIGNMENT_TARGET_CODE,
                    "x01.parse.invalid_assignment_target",
                    expression.span(),
                    "赋值左侧必须是名称、成员或选择器表达式".to_string(),
                );
                self.synchronize_to_boundary();
                return None;
            }
            let span = self.source_span(expression.span().start(), value.span().end());
            self.consume_newline();
            if operator == AssignmentOperator::Assign {
                if let Expression::Name(target) = expression {
                    return Some(Statement::Assignment {
                        target,
                        value,
                        leading_docs,
                        span,
                    });
                }
            }
            return Some(Statement::ExtendedAssignment {
                target: expression,
                operator,
                value,
                leading_docs,
                span,
            });
        }

        if !is_statement_boundary(self.current().kind()) {
            self.report_unexpected_tail();
            self.synchronize_to_boundary();
            return None;
        }
        let span = expression.span();
        self.consume_newline();
        Some(Statement::Expression {
            span,
            expression,
            leading_docs,
        })
    }

    /// 判断当前位置是否为独立的 `[main]` 程序入口表头。
    pub(super) fn starts_main_header(&self) -> bool {
        if !self.at(TokenKind::LeftBracket) {
            return false;
        }
        let Some(name) = self.tokens.get(self.cursor + 1).copied() else {
            return false;
        };
        let Some(close) = self.tokens.get(self.cursor + 2).copied() else {
            return false;
        };
        let after = self
            .tokens
            .get(self.cursor + 3)
            .map(|token| token.kind())
            .unwrap_or(TokenKind::Eof);
        name.kind() == TokenKind::Identifier
            && self.source.slice(name.span()) == "main"
            && close.kind() == TokenKind::RightBracket
            && is_statement_boundary(after)
    }

    /// 判断当前位置是否为顶层 `[Name]` 或 `[[Name]]` 表头。
    ///
    /// 只有普通 ASCII 标识符可以作为源码表名；反引号名称仍可用于
    /// 字段和方法，但不能绕过跨平台表名规则。表头后的 Token 必须是
    /// 语句边界，避免把数组/比较表达式误判为表声明。
    pub(super) fn starts_table_header(&self) -> bool {
        if !self.at(TokenKind::LeftBracket) {
            return false;
        }
        let first = self.tokens.get(self.cursor + 1).copied();
        let second = self.tokens.get(self.cursor + 2).copied();
        let third = self.tokens.get(self.cursor + 3).copied();
        let fourth = self.tokens.get(self.cursor + 4).copied();
        let is_name = |token: Option<Token>| {
            token.is_some_and(|token| {
                token.kind() == TokenKind::Identifier
                    && KeywordKind::from_word(token.text(self.source)).is_none()
            })
        };
        if first.is_some_and(|token| token.kind() == TokenKind::LeftBracket)
            && is_name(second)
            && third.is_some_and(|token| token.kind() == TokenKind::RightBracket)
            && fourth.is_some_and(|token| token.kind() == TokenKind::RightBracket)
        {
            return self
                .tokens
                .get(self.cursor + 5)
                .is_none_or(|token| is_statement_boundary(token.kind()));
        }
        is_name(first)
            && second.is_some_and(|token| token.kind() == TokenKind::RightBracket)
            && third.is_none_or(|token| is_statement_boundary(token.kind()))
    }

    /// 解析一个 `[Name]` 或 `[[Name]]` 表声明及其缩进体。
    pub(super) fn parse_table_statement(
        &mut self,
        leading_docs: Vec<SourceSpan>,
    ) -> Option<Statement> {
        let start = self.current().span().start();
        let first = self.bump();
        let kind = if self.at(TokenKind::LeftBracket) {
            self.bump();
            TableKind::Instance
        } else {
            TableKind::Singleton
        };
        let name_token = self.current();
        if name_token.kind() != TokenKind::Identifier
            || KeywordKind::from_word(name_token.text(self.source)).is_some()
        {
            self.push_error(
                INVALID_TABLE_HEADER_CODE,
                "x05.parse.invalid_table_name",
                name_token.span(),
                "表名必须是非保留的 ASCII 标识符".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        let name = self.parse_name();
        let Some(close) = self.expect_delimiter(TokenKind::RightBracket, first.span()) else {
            self.synchronize_to_boundary();
            return None;
        };
        let final_close = if kind == TableKind::Instance {
            self.expect_delimiter(TokenKind::RightBracket, close.span())?
        } else {
            close
        };
        if !is_statement_boundary(self.current().kind()) {
            self.push_error(
                INVALID_TABLE_HEADER_CODE,
                "x05.parse.table_header_tail",
                self.current().span(),
                "表头后只能出现换行".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        let header_span = self.source_span(start, final_close.span().end());
        let (body, body_end) = self.parse_table_body(header_span);
        Some(Statement::Table {
            name,
            kind,
            body,
            leading_docs,
            span: self.source_span(start, body_end.max(header_span.end())),
        })
    }

    /// 解析表体；表成员必须是字段赋值/声明或方法定义。
    fn parse_table_body(&mut self, header_span: SourceSpan) -> (Vec<Statement>, usize) {
        if self.at(TokenKind::Newline) {
            self.bump();
        } else if !self.at(TokenKind::Eof) {
            self.push_error(
                MISSING_TABLE_BODY_CODE,
                "x05.parse.table_body_newline",
                header_span,
                "表头后必须换行并跟随缩进体".to_string(),
            );
            return (Vec::new(), header_span.end());
        }
        let mut body_leading_docs = Vec::new();
        loop {
            if self.at(TokenKind::Newline) {
                self.bump();
            } else if self.at(TokenKind::DocComment) {
                body_leading_docs.push(self.bump().span());
            } else {
                break;
            }
        }
        if !self.at(TokenKind::Indent) {
            self.push_error(
                MISSING_TABLE_BODY_CODE,
                "x05.parse.missing_table_indent",
                header_span,
                "表声明必须包含缩进体".to_string(),
            );
            return (Vec::new(), header_span.end());
        }
        self.bump();
        self.block_context_depth = self.block_context_depth.saturating_add(1);
        let mut body = Vec::new();
        let mut pending_docs = body_leading_docs;
        let mut last_end = header_span.end();
        while !self.at(TokenKind::Dedent) && !self.at(TokenKind::Eof) {
            match self.current().kind() {
                TokenKind::Newline => {
                    last_end = self.bump().span().end();
                }
                TokenKind::DocComment => pending_docs.push(self.bump().span()),
                TokenKind::Indent => {
                    let token = self.bump();
                    self.push_error(
                        INVALID_TABLE_MEMBER_CODE,
                        "x05.parse.unexpected_table_indent",
                        token.span(),
                        "表成员不能出现额外缩进".to_string(),
                    );
                }
                _ => {
                    let docs = std::mem::take(&mut pending_docs);
                    let before = self.diagnostics.len();
                    if let Some(statement) = self.parse_statement(docs) {
                        if !is_table_member_statement(&statement) {
                            self.push_error(
                                INVALID_TABLE_MEMBER_CODE,
                                "x05.parse.invalid_table_member",
                                statement.span(),
                                "表体只能包含字段赋值、字段声明或 def 方法".to_string(),
                            );
                        }
                        last_end = statement.span().end();
                        body.push(statement);
                    } else if self.diagnostics.len() == before {
                        self.push_error(
                            INVALID_TABLE_MEMBER_CODE,
                            "x05.parse.invalid_table_member",
                            self.current().span(),
                            "无法解析表成员".to_string(),
                        );
                        self.synchronize_to_boundary();
                    }
                }
            }
        }
        if self.at(TokenKind::Dedent) {
            last_end = self.bump().span().end().max(last_end);
        }
        self.block_context_depth = self.block_context_depth.saturating_sub(1);
        if body.is_empty() {
            self.push_error(
                MISSING_TABLE_BODY_CODE,
                "x05.parse.empty_table_body",
                header_span,
                "表体不能为空".to_string(),
            );
        }
        (body, last_end)
    }

    /// 解析独立的 `[main]` 入口表头并消费其换行。
    pub(super) fn parse_main_header(&mut self) -> Option<SourceSpan> {
        let open = self.bump();
        let name = self.bump();
        let close = self.bump();
        let span = self.source_span(open.span().start(), close.span().end());
        if !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            self.push_error(
                INVALID_ENTRY_CODE,
                "x04.parse.main_tail",
                self.current().span(),
                "[main] 表头后只能出现换行".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        let _ = (name, span);
        self.consume_newline();
        Some(span)
    }

    /// 解析 `def` 函数定义及其缩进体。
    fn parse_function_statement(&mut self, leading_docs: Vec<SourceSpan>) -> Option<Statement> {
        let keyword = self.bump();
        if !is_declaration_name_token(self.current().kind()) {
            self.push_error(
                INVALID_FUNCTION_CODE,
                "x04.parse.missing_function_name",
                self.current().span(),
                "def 后必须是函数名称".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        let name = self.parse_name();
        if !self.at(TokenKind::LeftParen) {
            self.push_error(
                INVALID_FUNCTION_CODE,
                "x04.parse.function_parentheses",
                self.current().span(),
                "函数名称后必须是参数列表".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        let parameters = self.parse_function_parameters();
        let close = self.expect_delimiter(TokenKind::RightParen, name.span);
        let mut return_type = None;
        if self.at(TokenKind::Minus) && self.lookahead_kind(1) == Some(TokenKind::Greater) {
            self.bump();
            self.bump();
            return_type = self.parse_function_type_annotation();
            if return_type.is_none() {
                self.push_error(
                    INVALID_FUNCTION_TYPE_CODE,
                    "x04.parse.invalid_return_type",
                    self.current().span(),
                    "函数返回类型必须是标量或 none".to_string(),
                );
            }
        }
        if !is_statement_boundary(self.current().kind()) {
            self.push_error(
                INVALID_FUNCTION_CODE,
                "x04.parse.function_header_tail",
                self.current().span(),
                "函数头后存在未预期内容".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        let header_end = return_type
            .map(|_| self.previous_span_end())
            .or_else(|| close.map(|token| token.span().end()))
            .unwrap_or(name.span.end());
        let (body, body_end) =
            self.parse_indented_block(self.source_span(keyword.span().start(), header_end));
        let span = self.source_span(keyword.span().start(), body_end.max(header_end));
        Some(Statement::Function {
            name,
            parameters,
            return_type,
            body,
            leading_docs,
            span,
        })
    }

    /// 解析函数参数列表；列表开始位置必须是 `(` 之后。
    fn parse_function_parameters(&mut self) -> Vec<FunctionParameter> {
        self.bump();
        self.soft_newline_depth += 1;
        self.skip_soft_newlines();
        let mut parameters: Vec<FunctionParameter> = Vec::new();
        let mut keyword_only = false;
        let mut seen_positional_marker = false;
        let mut seen_default = false;
        let mut seen_varargs = false;
        let mut seen_varkw = false;
        let mut seen_names = BTreeSet::new();
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            self.skip_soft_newlines();
            if self.at(TokenKind::Comma) {
                self.bump();
                continue;
            }
            if self.at(TokenKind::Slash) {
                let separated = self.previous_non_newline_kind() == Some(TokenKind::Comma);
                let marker = self.bump();
                if !separated {
                    self.parameter_error(marker.span(), "位置参数 `/` 标记前必须有逗号");
                } else if seen_positional_marker || keyword_only || parameters.is_empty() {
                    self.parameter_error(marker.span(), "位置参数 `/` 标记位置无效");
                } else {
                    seen_positional_marker = true;
                    for parameter in &mut parameters {
                        if parameter.kind == FunctionParameterKind::PositionalOrKeyword {
                            parameter.kind = FunctionParameterKind::PositionalOnly;
                        }
                    }
                    // `/` 只修饰它之前的参数；后续参数保持普通位置/关键字语义。
                }
                self.consume_parameter_comma();
                continue;
            }
            if self.at(TokenKind::Star) || self.at(TokenKind::Power) {
                let star = self.bump();
                if star.kind() == TokenKind::Power || self.at(TokenKind::Star) {
                    if star.kind() != TokenKind::Power {
                        self.bump();
                    }
                    if seen_varkw {
                        self.parameter_error(star.span(), "函数只能声明一个 **kwargs 参数");
                        self.recover_parameter_list();
                        break;
                    }
                    let annotation = self.parse_optional_parameter_annotation();
                    let Some(name) = self.parse_parameter_name() else {
                        self.recover_parameter_list();
                        break;
                    };
                    let span = self.source_span(star.span().start(), name.span.end());
                    self.record_parameter_name(&mut seen_names, name);
                    parameters.push(FunctionParameter {
                        name,
                        kind: FunctionParameterKind::VarKeywords,
                        annotation,
                        default: None,
                        span,
                    });
                    seen_varkw = true;
                    self.consume_parameter_comma();
                    continue;
                }
                if seen_varargs || seen_varkw {
                    self.parameter_error(star.span(), "可变参数必须位于参数列表末端区域");
                    self.recover_parameter_list();
                    break;
                }
                if self.at(TokenKind::Comma) || self.at(TokenKind::RightParen) {
                    keyword_only = true;
                    self.consume_parameter_comma();
                    continue;
                }
                let annotation = self.parse_optional_parameter_annotation();
                let Some(name) = self.parse_parameter_name() else {
                    self.recover_parameter_list();
                    break;
                };
                let span = self.source_span(star.span().start(), name.span.end());
                self.record_parameter_name(&mut seen_names, name);
                parameters.push(FunctionParameter {
                    name,
                    kind: FunctionParameterKind::VarArgs,
                    annotation,
                    default: None,
                    span,
                });
                seen_varargs = true;
                keyword_only = true;
                self.consume_parameter_comma();
                continue;
            }
            if seen_varkw {
                self.parameter_error(self.current().span(), "**kwargs 后不能继续声明参数");
                self.recover_parameter_list();
                break;
            }
            let start = self.current().span();
            let annotation = self.parse_optional_parameter_annotation();
            let Some(name) = self.parse_parameter_name() else {
                self.recover_parameter_list();
                break;
            };
            let default = if self.at(TokenKind::Equal) {
                self.bump();
                let value = self.parse_expression();
                if value.is_none() {
                    self.parameter_error(name.span, "默认参数缺少表达式");
                }
                value
            } else {
                None
            };
            if default.is_some() {
                seen_default = true;
            } else if seen_default && !keyword_only {
                self.parameter_error(name.span, "无默认值参数不能位于默认参数之后");
            }
            let kind = if keyword_only {
                FunctionParameterKind::KeywordOnly
            } else {
                FunctionParameterKind::PositionalOrKeyword
            };
            let end = default
                .as_ref()
                .map_or(name.span.end(), Expression::span_end);
            self.record_parameter_name(&mut seen_names, name);
            parameters.push(FunctionParameter {
                name,
                kind,
                annotation,
                default,
                span: self.source_span(start.start(), end),
            });
            self.consume_parameter_comma();
        }
        self.soft_newline_depth = self.soft_newline_depth.saturating_sub(1);
        parameters
    }

    /// 解析可选的参数声明式类型前缀。
    fn parse_optional_parameter_annotation(&mut self) -> Option<FunctionTypeAnnotation> {
        if self.current().kind() == TokenKind::None {
            self.bump();
            return Some(FunctionTypeAnnotation::None);
        }
        if !is_scalar_type_token(self.current().kind())
            || self.lookahead_kind(1) == Some(TokenKind::LeftParen)
        {
            return None;
        }
        let scalar = ScalarType::from_keyword(self.current().kind().keyword()?)?;
        self.bump();
        Some(FunctionTypeAnnotation::Scalar(scalar))
    }

    /// 解析函数返回类型注解。
    fn parse_function_type_annotation(&mut self) -> Option<FunctionTypeAnnotation> {
        self.parse_optional_parameter_annotation()
    }

    /// 解析一个函数参数名称。
    fn parse_parameter_name(&mut self) -> Option<Name> {
        if is_declaration_name_token(self.current().kind()) {
            Some(self.parse_name())
        } else {
            self.parameter_error(self.current().span(), "参数必须是普通或反引号名称");
            None
        }
    }

    /// 消费参数逗号及其后的软换行。
    fn consume_parameter_comma(&mut self) {
        if self.at(TokenKind::Comma) {
            self.bump();
            self.skip_soft_newlines();
        } else if !self.at(TokenKind::RightParen) {
            self.parameter_error(self.current().span(), "参数之间必须使用逗号分隔");
            self.recover_parameter_list();
        }
    }

    /// 将参数列表错误输入消费到下一个逗号或右括号。
    fn recover_parameter_list(&mut self) {
        while !matches!(
            self.current().kind(),
            TokenKind::Comma | TokenKind::RightParen | TokenKind::Eof
        ) {
            self.bump();
        }
        if self.at(TokenKind::Comma) {
            self.bump();
        }
    }

    /// 追加参数结构诊断。
    fn parameter_error(&mut self, span: SourceSpan, message: &str) {
        self.push_error(
            INVALID_PARAMETER_CODE,
            "x04.parse.invalid_parameter",
            span,
            message.to_string(),
        );
    }

    /// 记录参数名称并拒绝同一名称空间中的重复参数。
    ///
    /// 键取自 `unquoted_text` 而不是源码切片：反引号名称必须与类型层、生命周期
    /// 阶段用同一规则归一化，否则 `foo` 与 `` `foo` `` 的去重键会不一致——
    /// 前者是 `ascii:foo`，后者本应是 `backtick:foo` 而不是 `` backtick:`foo` ``。
    fn record_parameter_name(&mut self, seen: &mut BTreeSet<String>, name: Name) {
        let prefix = if name.backticked {
            "backtick:"
        } else {
            "ascii:"
        };
        let key = format!("{prefix}{}", name.unquoted_text(self.source));
        if !seen.insert(key) {
            self.parameter_error(name.span, "函数参数名称不能重复");
        }
    }

    /// 解析一个缩进代码块；返回语句和块末源码偏移。
    fn parse_indented_block(&mut self, header_span: SourceSpan) -> (Vec<Statement>, usize) {
        if self.at(TokenKind::Newline) {
            self.bump();
        } else if !self.at(TokenKind::Eof) {
            self.push_error(
                MISSING_BLOCK_CODE,
                "x04.parse.missing_block_newline",
                header_span,
                "代码块头后必须换行".to_string(),
            );
            return (Vec::new(), header_span.end());
        }
        while self.at(TokenKind::Newline) {
            self.bump();
        }
        if !self.at(TokenKind::Indent) {
            self.push_error(
                MISSING_BLOCK_CODE,
                "x04.parse.missing_indent",
                header_span,
                "代码块必须包含缩进体".to_string(),
            );
            return (Vec::new(), header_span.end());
        }
        self.bump();
        self.block_context_depth = self.block_context_depth.saturating_add(1);
        let mut statements = Vec::new();
        let mut pending_docs = Vec::new();
        let mut last_end = header_span.end();
        while !self.at(TokenKind::Dedent)
            && !self.at(TokenKind::Eof)
            // `elif`/`else` at the parent indentation level is left for the
            // owning `if` parser. Nested blocks have already consumed their
            // own closing `Dedent` tokens before this point.
            && !matches!(
                self.current().kind(),
                TokenKind::Keyword(
                    KeywordKind::Elif
                        | KeywordKind::Else
                        | KeywordKind::Catch
                        | KeywordKind::Finally
                )
            )
        {
            match self.current().kind() {
                TokenKind::Newline => {
                    last_end = self.bump().span().end();
                }
                TokenKind::DocComment => pending_docs.push(self.bump().span()),
                TokenKind::Indent => {
                    let token = self.bump();
                    self.push_error(
                        INVALID_CONTROL_FLOW_CODE,
                        "x04.parse.unexpected_indent",
                        token.span(),
                        "未预期的额外缩进".to_string(),
                    );
                }
                _ => {
                    let docs = std::mem::take(&mut pending_docs);
                    if let Some(statement) = self.parse_statement(docs) {
                        last_end = statement.span().end();
                        statements.push(statement);
                    } else {
                        self.synchronize_to_boundary();
                    }
                }
            }
        }
        if self.at(TokenKind::Dedent) {
            last_end = self.bump().span().end().max(last_end);
        } else {
            pending_docs.clear();
        }
        self.block_context_depth = self.block_context_depth.saturating_sub(1);
        if statements.is_empty() {
            self.push_error(
                MISSING_BLOCK_CODE,
                "x04.parse.empty_block",
                header_span,
                "代码块不能为空".to_string(),
            );
        }
        (statements, last_end)
    }

    /// 解析 `if`/`elif`/`else` 条件链。
    fn parse_if_statement(&mut self, leading_docs: Vec<SourceSpan>) -> Option<Statement> {
        let start = self.bump();
        let condition = self.parse_control_condition("if")?;
        let (body, mut end) = self
            .parse_indented_block(self.source_span(start.span().start(), condition.span().end()));
        let mut elif_branches = Vec::new();
        let mut else_body = None;
        let mut had_alternate = false;
        loop {
            while self.at(TokenKind::Newline) {
                self.bump();
            }
            if self.at(TokenKind::Keyword(KeywordKind::Elif)) {
                had_alternate = true;
                let elif = self.bump();
                let Some(elif_condition) = self.parse_control_condition("elif") else {
                    break;
                };
                let header = self.source_span(elif.span().start(), elif_condition.span().end());
                let (elif_body, elif_end) = self.parse_indented_block(header);
                end = elif_end.max(end);
                elif_branches.push(ElifBranch {
                    condition: elif_condition,
                    body: elif_body,
                    span: self.source_span(elif.span().start(), elif_end),
                    leading_docs: Vec::new(),
                });
            } else if self.at(TokenKind::Keyword(KeywordKind::Else)) {
                had_alternate = true;
                let otherwise = self.bump();
                let header = otherwise.span();
                let (otherwise_body, otherwise_end) = self.parse_indented_block(header);
                end = otherwise_end.max(end);
                else_body = Some(otherwise_body);
                break;
            } else {
                break;
            }
        }
        if had_alternate && self.block_context_depth == 0 && self.at(TokenKind::Dedent) {
            end = self.bump().span().end().max(end);
        }
        Some(Statement::If {
            condition,
            body,
            elif_branches,
            else_body,
            leading_docs,
            span: self.source_span(start.span().start(), end),
        })
    }

    /// 解析 `try`、一个或多个按类型匹配的 `catch` 以及可选 `finally`。
    fn parse_try_statement(&mut self, leading_docs: Vec<SourceSpan>) -> Option<Statement> {
        let keyword = self.bump();
        let (body, mut end) = self.parse_indented_block(keyword.span());
        let mut catches = Vec::new();
        let mut finally_body = None;

        while self.at(TokenKind::Newline) {
            self.bump();
        }
        while self.at(TokenKind::Keyword(KeywordKind::Catch)) {
            let catch_keyword = self.bump();
            let Some(binding) = self.parse_parameter_name() else {
                self.synchronize_to_boundary();
                break;
            };
            if !self.at(TokenKind::Keyword(KeywordKind::As)) {
                self.push_error(
                    INVALID_ERROR_CONTROL_FLOW_CODE,
                    "x07.parse.catch_missing_as",
                    self.current().span(),
                    "catch 绑定名后必须使用 as 指定错误类型".to_string(),
                );
                self.synchronize_to_boundary();
                break;
            }
            self.bump();
            let Some(error_type) = self.parse_parameter_name() else {
                self.synchronize_to_boundary();
                break;
            };
            if !is_statement_boundary(self.current().kind()) {
                self.push_error(
                    INVALID_ERROR_CONTROL_FLOW_CODE,
                    "x07.parse.catch_header_tail",
                    self.current().span(),
                    "catch 头后存在未预期内容".to_string(),
                );
                self.synchronize_to_boundary();
                break;
            }
            let header = self.source_span(catch_keyword.span().start(), error_type.span.end());
            let (catch_body, catch_end) = self.parse_indented_block(header);
            end = end.max(catch_end);
            catches.push(CatchClause {
                binding,
                error_type,
                body: catch_body,
                leading_docs: Vec::new(),
                span: self.source_span(catch_keyword.span().start(), catch_end),
            });
            while self.at(TokenKind::Newline) {
                self.bump();
            }
        }

        if self.at(TokenKind::Keyword(KeywordKind::Finally)) {
            let finally_keyword = self.bump();
            let (body, finally_end) = self.parse_indented_block(finally_keyword.span());
            end = end.max(finally_end);
            finally_body = Some(body);
        }

        if catches.is_empty() && finally_body.is_none() {
            self.push_error(
                MISSING_ERROR_HANDLER_CODE,
                "x07.parse.missing_handler",
                keyword.span(),
                "try 后必须至少包含 catch 或 finally".to_string(),
            );
            return None;
        }
        Some(Statement::Try {
            body,
            catches,
            finally_body,
            leading_docs,
            span: self.source_span(keyword.span().start(), end),
        })
    }

    /// 解析 `raise error_expression`。
    fn parse_raise_statement(&mut self, leading_docs: Vec<SourceSpan>) -> Option<Statement> {
        let keyword = self.bump();
        if is_statement_boundary(self.current().kind()) {
            self.push_error(
                MISSING_RAISE_VALUE_CODE,
                "x07.parse.missing_raise_value",
                keyword.span(),
                "raise 后必须提供错误表达式".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        let value = self.parse_expression()?;
        if !is_statement_boundary(self.current().kind()) {
            self.push_error(
                INVALID_ERROR_CONTROL_FLOW_CODE,
                "x07.parse.raise_tail",
                self.current().span(),
                "raise 表达式后存在未预期内容".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        let span = self.source_span(keyword.span().start(), value.span_end());
        self.consume_newline();
        Some(Statement::Raise {
            value,
            leading_docs,
            span,
        })
    }

    /// 解析控制流头部条件并验证其语句边界。
    fn parse_control_condition(&mut self, keyword: &str) -> Option<Expression> {
        let condition = self.parse_expression();
        let Some(condition) = condition else {
            self.push_error(
                INVALID_CONTROL_FLOW_CODE,
                "x04.parse.missing_condition",
                self.current().span(),
                format!("{keyword} 后缺少条件表达式"),
            );
            self.synchronize_to_boundary();
            return None;
        };
        if !is_statement_boundary(self.current().kind()) {
            self.push_error(
                INVALID_CONTROL_FLOW_CODE,
                "x04.parse.control_header_tail",
                self.current().span(),
                format!("{keyword} 条件后存在未预期内容"),
            );
            self.synchronize_to_boundary();
            return None;
        }
        Some(condition)
    }

    /// 解析 `for name in iterable` 循环。
    fn parse_for_statement(&mut self, leading_docs: Vec<SourceSpan>) -> Option<Statement> {
        let start = self.bump();
        let Some(target) = self.parse_parameter_name() else {
            self.synchronize_to_boundary();
            return None;
        };
        if !self.at(TokenKind::Keyword(KeywordKind::In)) {
            self.push_error(
                INVALID_CONTROL_FLOW_CODE,
                "x04.parse.for_missing_in",
                self.current().span(),
                "for 循环目标后必须是 in".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        self.bump();
        let Some(iterable) = self.parse_expression() else {
            self.push_error(
                INVALID_CONTROL_FLOW_CODE,
                "x04.parse.for_missing_iterable",
                self.current().span(),
                "for 的 in 后缺少可迭代表达式".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        };
        if !is_statement_boundary(self.current().kind()) {
            self.push_error(
                INVALID_CONTROL_FLOW_CODE,
                "x04.parse.for_header_tail",
                self.current().span(),
                "for 头后存在未预期内容".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        let (body, end) = self
            .parse_indented_block(self.source_span(start.span().start(), iterable.span().end()));
        Some(Statement::For {
            target,
            iterable,
            body,
            leading_docs,
            span: self.source_span(start.span().start(), end),
        })
    }

    /// 解析 `while condition` 循环。
    fn parse_while_statement(&mut self, leading_docs: Vec<SourceSpan>) -> Option<Statement> {
        let start = self.bump();
        let condition = self.parse_control_condition("while")?;
        let (body, end) = self
            .parse_indented_block(self.source_span(start.span().start(), condition.span().end()));
        Some(Statement::While {
            condition,
            body,
            leading_docs,
            span: self.source_span(start.span().start(), end),
        })
    }

    /// 解析 `return`，允许省略返回表达式。
    fn parse_return_statement(&mut self, leading_docs: Vec<SourceSpan>) -> Option<Statement> {
        let keyword = self.bump();
        let value = if is_statement_boundary(self.current().kind()) {
            None
        } else {
            let value = self.parse_expression();
            if value.is_none() {
                self.push_error(
                    INVALID_CONTROL_FLOW_CODE,
                    "x04.parse.invalid_return",
                    keyword.span(),
                    "return 后的表达式无效".to_string(),
                );
            }
            value
        };
        let end = value
            .as_ref()
            .map_or(keyword.span().end(), Expression::span_end);
        if !is_statement_boundary(self.current().kind()) {
            self.push_error(
                INVALID_CONTROL_FLOW_CODE,
                "x04.parse.return_tail",
                self.current().span(),
                "return 后存在未预期内容".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        self.consume_newline();
        Some(Statement::Return {
            value,
            leading_docs,
            span: self.source_span(keyword.span().start(), end),
        })
    }

    /// 解析 `break` 或 `continue`。
    fn parse_loop_control_statement(
        &mut self,
        leading_docs: Vec<SourceSpan>,
        is_break: bool,
    ) -> Option<Statement> {
        let keyword = self.bump();
        if !is_statement_boundary(self.current().kind()) {
            self.push_error(
                INVALID_CONTROL_FLOW_CODE,
                "x04.parse.loop_control_tail",
                self.current().span(),
                "break/continue 后不能跟表达式".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        self.consume_newline();
        if is_break {
            Some(Statement::Break {
                leading_docs,
                span: keyword.span(),
            })
        } else {
            Some(Statement::Continue {
                leading_docs,
                span: keyword.span(),
            })
        }
    }
}

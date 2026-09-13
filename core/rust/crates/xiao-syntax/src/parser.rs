//! Xiao P0/P1 解析器与可恢复诊断。
//!
//! 解析器消费词法器提供的稳定 Token 流，构造 AST 并尽可能从错误中恢复；它不做
//! 类型推断、容器边界检查或执行。与词法器分离后，语法策略可以独立演进。

use xiao_diagnostics::Diagnostic;
use xiao_source::{SourceFile, SourceSpan};

use crate::ast::{
    AssignmentOperator, BinaryOperator, Expression, LiteralKind, Name, Program, ScalarType,
    Statement, UnaryOperator,
};
use crate::diagnostics::*;
use crate::lexer::Lexer;
use crate::selectors::{IndexPath, PathSegment, RandomMode, Selector, SelectorItem};
use crate::token::{KeywordKind, Token, TokenKind};

/// 词法和 P1 语法解析的统一结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseResult {
    /// 错误恢复后得到的程序；对可恢复错误尽量返回部分根节点。
    pub program: Option<Program>,
    /// 按阶段顺序收集的词法和语法诊断。
    pub diagnostics: Vec<ParseDiagnostic>,
}

impl ParseResult {
    /// 判断结果中是否包含错误级别诊断。
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// 判断结果是否无错误且包含程序根节点。
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.program.is_some() && !self.has_errors()
    }
}

/// P1 解析诊断沿用统一结构化诊断类型。
pub type ParseDiagnostic = Diagnostic;

/// 从已经验证的源码建立 P1 解析器。
///
/// 构造时会完整运行一次 L2 词法器；词法诊断会保留在最终
/// [`ParseResult::diagnostics`] 中，解析阶段不会重复报告同一个 `Invalid`
/// Token 的词法错误。
pub struct Parser<'source> {
    source: &'source SourceFile,
    tokens: Vec<Token>,
    diagnostics: Vec<ParseDiagnostic>,
    cursor: usize,
    /// 当前处于括号、方括号或步长花括号的层数；其中的换行是软换行。
    soft_newline_depth: usize,
}

impl<'source> Parser<'source> {
    /// 创建位于源码开头的 P1 解析器。
    #[must_use]
    pub fn new(source: &'source SourceFile) -> Self {
        let lexical = Lexer::new(source).tokenize();
        Self {
            source,
            tokens: lexical.tokens,
            diagnostics: lexical.diagnostics,
            cursor: 0,
            soft_newline_depth: 0,
        }
    }

    /// 解析整个源码并返回可恢复的 AST 与诊断。
    #[must_use]
    pub fn parse(mut self) -> ParseResult {
        let mut statements = Vec::new();
        let mut orphan_doc_comments = Vec::new();
        let mut pending_docs = Vec::new();

        while !self.at(TokenKind::Eof) {
            match self.current().kind() {
                TokenKind::Newline => {
                    self.bump();
                }
                TokenKind::DocComment => {
                    pending_docs.push(self.bump().span());
                }
                TokenKind::Indent => {
                    orphan_doc_comments.append(&mut pending_docs);
                    let token = self.bump();
                    self.push_error(
                        UNSUPPORTED_BLOCK_CODE,
                        "x01.parse.unsupported_block",
                        token.span(),
                        "P0 尚不支持缩进代码块".to_string(),
                    );
                    self.skip_indented_region(&mut orphan_doc_comments);
                }
                TokenKind::Dedent => {
                    orphan_doc_comments.append(&mut pending_docs);
                    let token = self.bump();
                    self.push_error(
                        UNSUPPORTED_BLOCK_CODE,
                        "x01.parse.unsupported_block",
                        token.span(),
                        "P0 不支持独立的反缩进 Token".to_string(),
                    );
                }
                _ => {
                    let docs = std::mem::take(&mut pending_docs);
                    if let Some(statement) = self.parse_statement(docs.clone()) {
                        statements.push(statement);
                    } else {
                        orphan_doc_comments.extend(docs);
                    }
                }
            }
        }
        orphan_doc_comments.append(&mut pending_docs);

        let span = self
            .source
            .span(0, self.source.len_bytes())
            .expect("源码整体区间必须有效");
        ParseResult {
            program: Some(Program {
                statements,
                orphan_doc_comments,
                span,
            }),
            diagnostics: self.diagnostics,
        }
    }

    /// 返回当前 Token；词法器始终保证序列末尾存在 `Eof`。
    fn current(&self) -> Token {
        self.tokens
            .get(self.cursor)
            .copied()
            .or_else(|| self.tokens.last().copied())
            .expect("词法结果至少包含 EOF Token")
    }

    /// 消费当前 Token；EOF 不会推进游标。
    fn bump(&mut self) -> Token {
        let token = self.current();
        if token.kind() != TokenKind::Eof && self.cursor < self.tokens.len() {
            self.cursor += 1;
        }
        token
    }

    /// 判断当前 Token 类别。
    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind() == kind
    }

    /// 解析一条 P1 顶层语句，并保留 P0 简单赋值的兼容形状。
    fn parse_statement(&mut self, leading_docs: Vec<SourceSpan>) -> Option<Statement> {
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

    /// 判断当前位置是否明确进入 P2 标量/常量声明语法。
    ///
    /// 标量关键字后只有紧跟名称时才视为声明；`int(value)` 等构造式调用
    /// 继续交给 P1 表达式解析，避免把调用误判为声明。
    fn starts_declaration(&self) -> bool {
        if self.at(TokenKind::Keyword(KeywordKind::Const)) {
            return true;
        }
        is_scalar_type_token(self.current().kind())
            && self.lookahead_kind(1) != Some(TokenKind::LeftParen)
    }

    /// 解析 `type name [= expression]` 或 `const [type] name = expression`。
    fn parse_declaration_statement(&mut self, leading_docs: Vec<SourceSpan>) -> Option<Statement> {
        let start = self.current().span().start();
        let is_const = self.at(TokenKind::Keyword(KeywordKind::Const));
        if is_const {
            self.bump();
        }

        let declared_type = if is_scalar_type_token(self.current().kind()) {
            let token = self.bump();
            ScalarType::from_keyword(
                token
                    .kind()
                    .keyword()
                    .expect("标量类型 Token 必须携带关键字"),
            )
        } else {
            None
        };

        if !is_declaration_name_token(self.current().kind()) {
            let span = self.current().span();
            self.push_error(
                INVALID_DECLARATION_TARGET_CODE,
                "x02.parse.invalid_declaration_target",
                span,
                "类型或 const 后必须是名称".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        let target = self.parse_name();

        if !is_const && is_statement_boundary(self.current().kind()) {
            let span = self.source_span(start, target.span.end());
            self.consume_newline();
            return Some(Statement::Declaration {
                target,
                declared_type: declared_type.expect("普通声明必须有标量类型"),
                value: None,
                leading_docs,
                span,
            });
        }

        if !self.at(TokenKind::Equal) {
            let span = self.current().span();
            let code = if is_const {
                MISSING_CONST_VALUE_CODE
            } else {
                INVALID_DECLARATION_CODE
            };
            let message_id = if is_const {
                "x02.parse.missing_const_value"
            } else {
                "x02.parse.invalid_declaration"
            };
            let message = if is_const {
                "const 声明必须包含初始化表达式"
            } else {
                "类型声明后只能跟初始化赋值或语句结束"
            };
            self.push_error(code, message_id, span, message.to_string());
            self.synchronize_to_boundary();
            return None;
        }
        self.bump();

        if is_statement_boundary(self.current().kind()) {
            let code = if is_const {
                MISSING_CONST_VALUE_CODE
            } else {
                INVALID_DECLARATION_CODE
            };
            let message_id = if is_const {
                "x02.parse.missing_const_value"
            } else {
                "x02.parse.invalid_declaration"
            };
            self.push_error(
                code,
                message_id,
                self.current().span(),
                "声明初始化表达式不能为空".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }

        let value_diagnostics = self.diagnostics.len();
        let value = match self.parse_expression() {
            Some(value) => value,
            None => {
                if self.diagnostics.len() == value_diagnostics {
                    self.push_error(
                        INVALID_DECLARATION_CODE,
                        "x02.parse.invalid_declaration_value",
                        self.current().span(),
                        "声明初始化表达式无效".to_string(),
                    );
                }
                self.synchronize_to_boundary();
                return None;
            }
        };

        if !is_statement_boundary(self.current().kind()) {
            self.push_error(
                INVALID_DECLARATION_CODE,
                "x02.parse.invalid_declaration_tail",
                self.current().span(),
                "声明表达式后存在未预期内容".to_string(),
            );
            self.synchronize_to_boundary();
            return None;
        }
        let span = self.source_span(start, value.span().end());
        self.consume_newline();
        if is_const {
            Some(Statement::ConstDeclaration {
                target,
                declared_type,
                value,
                leading_docs,
                span,
            })
        } else {
            Some(Statement::Declaration {
                target,
                declared_type: declared_type.expect("普通声明必须有标量类型"),
                value: Some(value),
                leading_docs,
                span,
            })
        }
    }

    /// 解析一个名称 Token 或标量类型关键字。
    fn parse_name(&mut self) -> Name {
        let token = self.bump();
        Name {
            span: token.span(),
            backticked: token.kind() == TokenKind::BacktickIdentifier,
        }
    }

    /// 使用最低绑定强度解析一个完整表达式。
    fn parse_expression(&mut self) -> Option<Expression> {
        self.parse_expression_bp(0)
    }

    /// Pratt 解析器的核心递归函数。
    fn parse_expression_bp(&mut self, minimum_binding_power: u8) -> Option<Expression> {
        self.skip_soft_newlines();
        let mut left = self.parse_prefix_expression()?;
        loop {
            self.skip_soft_newlines();
            if let Some((operator, left_bp, right_bp, extra_tokens)) = self.current_infix() {
                if left_bp < minimum_binding_power {
                    break;
                }
                self.bump();
                for _ in 1..extra_tokens {
                    self.bump();
                }
                let right = match self.parse_expression_bp(right_bp) {
                    Some(right) => right,
                    None => {
                        self.push_error(
                            MISSING_EXPRESSION_CODE,
                            "x01.parse.missing_expression",
                            self.current().span(),
                            "二元运算符右侧缺少表达式".to_string(),
                        );
                        break;
                    }
                };
                let span = self.source_span(left.span().start(), right.span().end());
                left = Expression::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
                continue;
            }
            break;
        }
        Some(left)
    }

    /// 解析一个前缀原子并继续处理其后缀。
    fn parse_prefix_expression(&mut self) -> Option<Expression> {
        let token = self.current();
        let expression = match token.kind() {
            kind if LiteralKind::from_token_kind(kind).is_some() => {
                self.bump();
                Expression::Literal {
                    kind: LiteralKind::from_token_kind(token.kind()).expect("已确认是字面量 Token"),
                    span: token.span(),
                }
            }
            kind if is_expression_name_token(kind) => Expression::Name(self.parse_name()),
            TokenKind::Plus | TokenKind::Minus => {
                self.bump();
                let operand = match self.parse_expression_bp(55) {
                    Some(operand) => operand,
                    None => {
                        self.push_error(
                            MISSING_EXPRESSION_CODE,
                            "x01.parse.missing_unary_operand",
                            token.span(),
                            "一元运算符后缺少表达式".to_string(),
                        );
                        return None;
                    }
                };
                let span = self.source_span(token.span().start(), operand.span().end());
                Expression::Unary {
                    operator: if token.kind() == TokenKind::Plus {
                        UnaryOperator::Plus
                    } else {
                        UnaryOperator::Minus
                    },
                    operand: Box::new(operand),
                    span,
                }
            }
            TokenKind::Keyword(KeywordKind::Not) => {
                self.bump();
                let operand = match self.parse_expression_bp(55) {
                    Some(operand) => operand,
                    None => {
                        self.push_error(
                            MISSING_EXPRESSION_CODE,
                            "x01.parse.missing_unary_operand",
                            token.span(),
                            "not 后缺少表达式".to_string(),
                        );
                        return None;
                    }
                };
                let span = self.source_span(token.span().start(), operand.span().end());
                Expression::Unary {
                    operator: UnaryOperator::Not,
                    operand: Box::new(operand),
                    span,
                }
            }
            TokenKind::LeftParen => self.parse_group_expression()?,
            TokenKind::Keyword(KeywordKind::New) => self.parse_new_expression()?,
            _ => return None,
        };
        self.parse_postfix_expression(expression)
    }

    /// 解析括号分组表达式。
    fn parse_group_expression(&mut self) -> Option<Expression> {
        let open = self.bump();
        self.soft_newline_depth += 1;
        self.skip_soft_newlines();
        let expression = self.parse_expression_bp(0);
        self.skip_soft_newlines();
        let close = self.expect_delimiter(TokenKind::RightParen, open.span());
        self.soft_newline_depth = self.soft_newline_depth.saturating_sub(1);
        let expression = match expression {
            Some(expression) => expression,
            None => {
                self.push_error(
                    MISSING_EXPRESSION_CODE,
                    "x01.parse.missing_group_expression",
                    open.span(),
                    "括号中缺少表达式".to_string(),
                );
                return None;
            }
        };
        let end = close.map_or(expression.span().end(), |token| token.span().end());
        Some(Expression::Group {
            expression: Box::new(expression),
            span: self.source_span(open.span().start(), end),
        })
    }

    /// 解析 `new Type(args...)` 构造调用。
    fn parse_new_expression(&mut self) -> Option<Expression> {
        let keyword = self.bump();
        let callee = self.parse_new_callee(keyword.span())?;
        if self.at(TokenKind::LeftParen) {
            let call = self.parse_call_suffix(callee)?;
            let Expression::Call {
                callee,
                arguments,
                span,
            } = call
            else {
                unreachable!("parse_call_suffix 必然返回调用节点");
            };
            Some(Expression::NewCall {
                callee,
                arguments,
                span: self.source_span(keyword.span().start(), span.end()),
            })
        } else {
            let other = callee;
            self.push_error(
                MISSING_DELIMITER_CODE,
                "x01.parse.new_call_parentheses",
                other.span(),
                "new 构造调用必须带括号参数列表".to_string(),
            );
            Some(Expression::NewCall {
                span: self.source_span(keyword.span().start(), other.span().end()),
                callee: Box::new(other),
                arguments: Vec::new(),
            })
        }
    }

    /// 解析 `new` 后的构造目标，并允许用点号组成限定名称。
    fn parse_new_callee(&mut self, keyword_span: SourceSpan) -> Option<Expression> {
        let mut callee = if is_expression_name_token(self.current().kind()) {
            Expression::Name(self.parse_name())
        } else if self.at(TokenKind::LeftParen) {
            self.parse_group_expression()?
        } else {
            self.push_error(
                MISSING_EXPRESSION_CODE,
                "x01.parse.missing_new_callee",
                keyword_span,
                "new 后缺少构造目标".to_string(),
            );
            return None;
        };

        while self.at(TokenKind::Dot) {
            let dot = self.bump();
            if !is_expression_name_token(self.current().kind()) {
                self.push_error(
                    INVALID_EXPRESSION_CODE,
                    "x01.parse.missing_member_name",
                    dot.span(),
                    "点号后缺少成员名称".to_string(),
                );
                break;
            }
            let member = self.parse_name();
            let span = self.source_span(callee.span().start(), member.span.end());
            callee = Expression::Member {
                object: Box::new(callee),
                member,
                span,
            };
        }
        Some(callee)
    }

    /// 循环解析调用、成员、转换、步长和选择器后缀。
    fn parse_postfix_expression(&mut self, mut expression: Expression) -> Option<Expression> {
        loop {
            match self.current().kind() {
                TokenKind::LeftParen => {
                    expression = self.parse_call_suffix(expression)?;
                }
                TokenKind::Dot => {
                    let dot = self.bump();
                    if !is_expression_name_token(self.current().kind()) {
                        self.push_error(
                            INVALID_EXPRESSION_CODE,
                            "x01.parse.missing_member_name",
                            dot.span(),
                            "点号后缺少成员名称".to_string(),
                        );
                        break;
                    }
                    let member = self.parse_name();
                    let span = self.source_span(expression.span().start(), member.span.end());
                    expression = Expression::Member {
                        object: Box::new(expression),
                        member,
                        span,
                    };
                }
                TokenKind::Keyword(KeywordKind::As) => {
                    let as_token = self.bump();
                    let target_token = self.current();
                    let target = match target_token
                        .kind()
                        .keyword()
                        .and_then(ScalarType::from_keyword)
                    {
                        Some(target) => {
                            self.bump();
                            target
                        }
                        None => {
                            self.push_error(
                                INVALID_CAST_TARGET_CODE,
                                "x01.parse.invalid_cast_target",
                                target_token.span(),
                                "as 后只能使用受支持的标量类型".to_string(),
                            );
                            if !is_statement_boundary(target_token.kind()) {
                                self.bump();
                            }
                            break;
                        }
                    };
                    let span =
                        self.source_span(expression.span().start(), target_token.span().end());
                    let _ = as_token;
                    expression = Expression::Cast {
                        expression: Box::new(expression),
                        target,
                        span,
                    };
                }
                TokenKind::LeftBrace => {
                    let brace = self.bump();
                    self.soft_newline_depth += 1;
                    self.skip_soft_newlines();
                    let step = self.parse_expression_bp(0);
                    self.skip_soft_newlines();
                    let close = self.expect_delimiter(TokenKind::RightBrace, brace.span());
                    self.soft_newline_depth = self.soft_newline_depth.saturating_sub(1);
                    let Some(step) = step else {
                        self.push_error(
                            INVALID_SELECTOR_CODE,
                            "x01.parse.missing_step",
                            brace.span(),
                            "步长花括号中缺少表达式".to_string(),
                        );
                        break;
                    };
                    if !matches!(self.current().kind(), TokenKind::LeftBracket) {
                        self.push_error(
                            INVALID_SELECTOR_CODE,
                            "x01.parse.step_without_selector",
                            close.map_or(brace.span(), |token| token.span()),
                            "步长后必须紧跟方括号选择器".to_string(),
                        );
                        break;
                    }
                    expression = self.parse_selector_suffix(expression, Some(step))?;
                }
                TokenKind::LeftBracket => {
                    expression = self.parse_selector_suffix(expression, None)?;
                }
                _ => break,
            }
        }
        Some(expression)
    }

    /// 解析普通调用后缀。
    fn parse_call_suffix(&mut self, callee: Expression) -> Option<Expression> {
        let open = self.bump();
        self.soft_newline_depth += 1;
        self.skip_soft_newlines();
        let mut arguments = Vec::new();
        if !self.at(TokenKind::RightParen) {
            loop {
                let argument = match self.parse_expression_bp(0) {
                    Some(argument) => argument,
                    None => {
                        self.push_error(
                            MISSING_EXPRESSION_CODE,
                            "x01.parse.missing_call_argument",
                            self.current().span(),
                            "调用参数缺少表达式".to_string(),
                        );
                        self.recover_call_arguments();
                        break;
                    }
                };
                arguments.push(argument);
                self.skip_soft_newlines();
                if self.at(TokenKind::Comma) {
                    self.bump();
                    self.skip_soft_newlines();
                    if self.at(TokenKind::RightParen) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        self.skip_soft_newlines();
        let close = self.expect_delimiter(TokenKind::RightParen, open.span());
        self.soft_newline_depth = self.soft_newline_depth.saturating_sub(1);
        let end = close.map_or_else(
            || {
                arguments
                    .last()
                    .map_or(open.span().end(), |arg| arg.span().end())
            },
            |token| token.span().end(),
        );
        Some(Expression::Call {
            callee: Box::new(callee.clone()),
            arguments,
            span: self.source_span(callee.span().start(), end),
        })
    }

    /// 解析一个带可选步长的方括号选择器。
    fn parse_selector_suffix(
        &mut self,
        source: Expression,
        step: Option<Expression>,
    ) -> Option<Expression> {
        let open = self.bump();
        self.soft_newline_depth += 1;
        self.skip_soft_newlines();
        let mut items = Vec::new();
        let mut stopped_at_statement_boundary = false;

        if self.at(TokenKind::RightBracket) {
            self.push_error(
                INVALID_SELECTOR_CODE,
                "x01.parse.empty_selector",
                open.span(),
                "方括号选择器不能为空".to_string(),
            );
        } else {
            loop {
                let item = self.parse_selector_item();
                let item_failed = item.is_none();
                if let Some(item) = item {
                    items.push(item);
                } else {
                    self.recover_selector_item();
                }
                // 选择器内部的换行通常是软换行，但错误项后若下一枚
                // Token 已经落在换行处，优先把它当作顶层同步点，避免
                // 未闭合的 `[` 吞掉下一条语句。
                if item_failed && self.at(TokenKind::Newline) {
                    // 若换行后紧跟逗号或右方括号，仍把它视为选择器
                    // 内部的软换行；否则把换行交还外层语句恢复。
                    if !matches!(
                        self.lookahead_non_newline_kind(),
                        Some(TokenKind::Comma | TokenKind::RightBracket)
                    ) {
                        stopped_at_statement_boundary = true;
                        break;
                    }
                }
                if self.at(TokenKind::Newline)
                    && !matches!(
                        self.lookahead_non_newline_kind(),
                        Some(TokenKind::Comma | TokenKind::RightBracket)
                    )
                {
                    // 已完成的选择项后若没有逗号/右括号，换行很可能
                    // 是下一条顶层语句的边界；不要为了软换行而吞掉它。
                    stopped_at_statement_boundary = true;
                    break;
                }
                self.skip_soft_newlines();
                if self.at(TokenKind::Comma) {
                    let comma = self.bump();
                    self.skip_soft_newlines();
                    if self.at(TokenKind::RightBracket) {
                        self.push_error(
                            INVALID_SELECTOR_CODE,
                            "x01.parse.trailing_selector_comma",
                            comma.span(),
                            "选择器不能以逗号结尾".to_string(),
                        );
                        break;
                    }
                    if self.at(TokenKind::Eof) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }

        if !stopped_at_statement_boundary {
            self.skip_soft_newlines();
        }
        let close = self.expect_delimiter(TokenKind::RightBracket, open.span());
        self.soft_newline_depth = self.soft_newline_depth.saturating_sub(1);
        let end = close.map_or_else(
            || {
                items
                    .last()
                    .map_or(open.span().end(), |item| item.span().end())
            },
            |token| token.span().end(),
        );
        let selector = Selector::new(items, self.source_span(open.span().start(), end));
        let span = self.source_span(source.span().start(), end);
        Some(Expression::Selector {
            source: Box::new(source),
            step: step.map(Box::new),
            selector,
            span,
        })
    }

    /// 将调用参数中的错误输入消费到右括号或 EOF。
    fn recover_call_arguments(&mut self) {
        while !matches!(
            self.current().kind(),
            TokenKind::RightParen | TokenKind::Eof
        ) {
            self.bump();
        }
    }

    /// 解析一个选择项：精确、范围、边界、全选或随机项。
    fn parse_selector_item(&mut self) -> Option<SelectorItem> {
        let first = self.current();
        match first.kind() {
            TokenKind::Equal => {
                self.bump();
                Some(SelectorItem::All { span: first.span() })
            }
            TokenKind::Question | TokenKind::BangQuestion => {
                let mode = if first.kind() == TokenKind::Question {
                    RandomMode::WithoutReplacement
                } else {
                    RandomMode::WithReplacement
                };
                self.bump();
                self.skip_soft_newlines();
                if selector_item_boundary(self.current().kind()) {
                    self.push_error(
                        INVALID_SELECTOR_CODE,
                        "x01.parse.missing_random_count",
                        first.span(),
                        "随机选择前缀后缺少数量表达式".to_string(),
                    );
                    return None;
                }
                let count_diagnostics = self.diagnostics.len();
                let count = match self.parse_expression_bp(0) {
                    Some(count) => count,
                    None => {
                        if self.diagnostics.len() == count_diagnostics {
                            self.push_error(
                                INVALID_SELECTOR_CODE,
                                "x01.parse.invalid_random_count",
                                first.span(),
                                "随机选择数量不是有效表达式".to_string(),
                            );
                        }
                        return None;
                    }
                };
                let span = self.source_span(first.span().start(), count.span().end());
                Some(SelectorItem::Random {
                    mode,
                    count: Box::new(count),
                    span,
                })
            }
            TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual => {
                self.bump();
                let endpoint_diagnostics = self.diagnostics.len();
                let endpoint = match self.parse_index_path() {
                    Some(endpoint) => endpoint,
                    None => {
                        if self.diagnostics.len() == endpoint_diagnostics {
                            self.push_error(
                                INVALID_SELECTOR_CODE,
                                "x01.parse.missing_range_endpoint",
                                first.span(),
                                "单边范围缺少路径端点".to_string(),
                            );
                        }
                        return None;
                    }
                };
                let (start, end, include_start, include_end) = match first.kind() {
                    TokenKind::Less => (None, Some(endpoint), false, false),
                    TokenKind::LessEqual => (None, Some(endpoint), false, true),
                    TokenKind::Greater => (Some(endpoint), None, false, false),
                    TokenKind::GreaterEqual => (Some(endpoint), None, true, false),
                    _ => unreachable!("已由 match 过滤边界运算符"),
                };
                let endpoint_end = start
                    .as_ref()
                    .or(end.as_ref())
                    .expect("单边范围必须有端点")
                    .span()
                    .end();
                Some(SelectorItem::OpenRange {
                    start,
                    end,
                    include_start,
                    include_end,
                    span: self.source_span(first.span().start(), endpoint_end),
                })
            }
            _ => {
                let start = self.parse_index_path()?;
                if self.at(TokenKind::Tilde) {
                    self.bump();
                    let end_diagnostics = self.diagnostics.len();
                    let end = match self.parse_index_path() {
                        Some(end) => end,
                        None => {
                            if self.diagnostics.len() == end_diagnostics {
                                self.push_error(
                                    INVALID_SELECTOR_CODE,
                                    "x01.parse.missing_range_endpoint",
                                    first.span(),
                                    "闭区间缺少右侧路径端点".to_string(),
                                );
                            }
                            return None;
                        }
                    };
                    Some(SelectorItem::Range {
                        span: self.source_span(first.span().start(), end.span().end()),
                        start,
                        end,
                    })
                } else {
                    Some(SelectorItem::Exact {
                        span: self.source_span(first.span().start(), start.span().end()),
                        path: start,
                    })
                }
            }
        }
    }

    /// 解析由 `/` 连接的数字或名称路径。
    fn parse_index_path(&mut self) -> Option<IndexPath> {
        let first = self.current();
        let mut segments = Vec::new();
        loop {
            let segment = self.parse_path_segment()?;
            let end = segment.span().end();
            segments.push(segment);
            if self.at(TokenKind::Slash) {
                self.bump();
                if selector_item_boundary(self.current().kind()) {
                    self.push_error(
                        INVALID_SELECTOR_CODE,
                        "x01.parse.missing_path_segment",
                        self.current().span(),
                        "路径分隔符后缺少路径段".to_string(),
                    );
                    return None;
                }
                continue;
            }
            return Some(IndexPath::new(
                segments,
                self.source_span(first.span().start(), end),
            ));
        }
    }

    /// 解析一个数字或名称路径段。
    fn parse_path_segment(&mut self) -> Option<PathSegment> {
        let token = self.current();
        match token.kind() {
            TokenKind::Integer => {
                self.bump();
                Some(PathSegment::Integer {
                    span: token.span(),
                    negative: false,
                })
            }
            TokenKind::Minus => {
                self.bump();
                let number = self.current();
                if number.kind() != TokenKind::Integer {
                    self.push_error(
                        INVALID_SELECTOR_CODE,
                        "x01.parse.invalid_path_segment",
                        token.span(),
                        "负索引后必须是十进制整数".to_string(),
                    );
                    return None;
                }
                self.bump();
                Some(PathSegment::Integer {
                    span: self.source_span(token.span().start(), number.span().end()),
                    negative: true,
                })
            }
            kind if is_expression_name_token(kind) => {
                self.bump();
                Some(PathSegment::Name(Name {
                    span: token.span(),
                    backticked: token.kind() == TokenKind::BacktickIdentifier,
                }))
            }
            // 这些 Token 表示调用方已经能够准确描述的“缺少路径段”边界。
            // 不在这里重复报告通用的非法路径段，避免 `1~]` 之类输入
            // 同时产生“非法路径段”和“缺少范围端点”两条诊断。
            TokenKind::Comma | TokenKind::RightBracket | TokenKind::Newline | TokenKind::Eof => {
                None
            }
            _ => {
                self.push_error(
                    INVALID_SELECTOR_CODE,
                    "x01.parse.invalid_path_segment",
                    token.span(),
                    "路径段必须是整数、普通名称或反引号名称".to_string(),
                );
                None
            }
        }
    }

    /// 将选择器内部的错误输入消费到逗号、右方括号或 EOF。
    fn recover_selector_item(&mut self) {
        while !matches!(
            self.current().kind(),
            TokenKind::Comma
                | TokenKind::RightBracket
                | TokenKind::Newline
                | TokenKind::Dedent
                | TokenKind::Eof
        ) {
            self.bump();
        }
    }

    /// 返回当前 Token 对应的中缀运算及其绑定强度。
    fn current_infix(&self) -> Option<(BinaryOperator, u8, u8, usize)> {
        let kind = self.current().kind();
        let result = match kind {
            TokenKind::Power => (BinaryOperator::Power, 60, 60, 1),
            TokenKind::Star => (BinaryOperator::Multiply, 50, 51, 1),
            TokenKind::Slash => (BinaryOperator::Divide, 50, 51, 1),
            TokenKind::FloorDiv => (BinaryOperator::FloorDivide, 50, 51, 1),
            TokenKind::Percent => (BinaryOperator::Remainder, 50, 51, 1),
            TokenKind::Plus => (BinaryOperator::Add, 40, 41, 1),
            TokenKind::Minus => (BinaryOperator::Subtract, 40, 41, 1),
            TokenKind::Less => (BinaryOperator::Less, 30, 31, 1),
            TokenKind::LessEqual => (BinaryOperator::LessEqual, 30, 31, 1),
            TokenKind::Greater => (BinaryOperator::Greater, 30, 31, 1),
            TokenKind::GreaterEqual => (BinaryOperator::GreaterEqual, 30, 31, 1),
            TokenKind::EqualEqual => (BinaryOperator::Equal, 30, 31, 1),
            TokenKind::BangEqual => (BinaryOperator::NotEqual, 30, 31, 1),
            TokenKind::Keyword(KeywordKind::In) => (BinaryOperator::In, 30, 31, 1),
            TokenKind::Keyword(KeywordKind::Is) => {
                if self.lookahead_kind(1) == Some(TokenKind::Keyword(KeywordKind::Not)) {
                    (BinaryOperator::IsNot, 30, 31, 2)
                } else {
                    (BinaryOperator::Is, 30, 31, 1)
                }
            }
            TokenKind::Keyword(KeywordKind::Not) => {
                if self.lookahead_kind(1) == Some(TokenKind::Keyword(KeywordKind::In)) {
                    (BinaryOperator::NotIn, 30, 31, 2)
                } else {
                    return None;
                }
            }
            TokenKind::Keyword(KeywordKind::And) => (BinaryOperator::And, 20, 21, 1),
            TokenKind::Keyword(KeywordKind::Or) => (BinaryOperator::Or, 10, 11, 1),
            _ => return None,
        };
        Some(result)
    }

    /// 查看相对当前位置的 Token 类别。
    fn lookahead_kind(&self, distance: usize) -> Option<TokenKind> {
        self.tokens
            .get(self.cursor.saturating_add(distance))
            .map(|token| token.kind())
    }

    /// 返回当前位置之后第一枚非换行 Token，用于错误恢复时判断软换行
    /// 后是否仍有选择器内部的逗号或闭分隔符。
    fn lookahead_non_newline_kind(&self) -> Option<TokenKind> {
        self.tokens
            .iter()
            .skip(self.cursor)
            .find(|token| token.kind() != TokenKind::Newline)
            .map(|token| token.kind())
    }

    /// 跳过括号、路径或步长内部的软换行。
    fn skip_soft_newlines(&mut self) {
        if self.soft_newline_depth == 0 {
            return;
        }
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    /// 期待一个配对分隔符；缺失时只报告错误，不吞掉同步点。
    fn expect_delimiter(&mut self, expected: TokenKind, opener: SourceSpan) -> Option<Token> {
        if self.at(expected) {
            return Some(self.bump());
        }
        let current = self.current();
        let span = if current.span().is_empty() {
            opener
        } else {
            current.span()
        };
        self.push_error(
            MISSING_DELIMITER_CODE,
            "x01.parse.missing_delimiter",
            span,
            format!("缺少配对分隔符 {:?}", expected),
        );
        None
    }

    /// 消费语句结尾的真实换行；`Dedent`/`Eof` 留给外层循环处理。
    fn consume_newline(&mut self) {
        if self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    /// 报告完整表达式后仍未消费的语法尾部。
    fn report_unexpected_tail(&mut self) {
        if self.current().kind() != TokenKind::Invalid {
            self.push_error(
                UNSUPPORTED_EXPRESSION_CODE,
                "x01.parse.unsupported_expression",
                self.current().span(),
                "表达式后存在未预期的语法内容".to_string(),
            );
        }
    }

    /// 将错误输入消费到换行、反缩进或 EOF，以便继续解析后续语句。
    fn synchronize_to_boundary(&mut self) {
        loop {
            match self.current().kind() {
                TokenKind::Newline => {
                    self.bump();
                    break;
                }
                TokenKind::Dedent | TokenKind::Eof => break,
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// 跳过一个 P0 不支持的缩进区域，并保留其后的顶层语句和文档区间。
    fn skip_indented_region(&mut self, orphan_doc_comments: &mut Vec<SourceSpan>) {
        let mut depth = 1usize;
        while depth > 0 && !self.at(TokenKind::Eof) {
            match self.current().kind() {
                TokenKind::DocComment => {
                    orphan_doc_comments.push(self.bump().span());
                }
                TokenKind::Indent => {
                    depth += 1;
                    self.bump();
                }
                TokenKind::Dedent => {
                    depth -= 1;
                    self.bump();
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// 追加一条带源码区间的解析错误。
    fn push_error(
        &mut self,
        code: &'static str,
        message_id: &'static str,
        span: SourceSpan,
        message: String,
    ) {
        self.diagnostics
            .push(Diagnostic::error_at(code, message_id, span, message));
    }

    /// 从两个已经验证的字节边界创建源码区间。
    fn source_span(&self, start: usize, end: usize) -> SourceSpan {
        self.source
            .span(start, end)
            .expect("Token 区间端点必须属于同一份 UTF-8 源码")
    }
}

/// 解析一个已经验证的 Xiao 源文件。
///
/// 这是 [`Parser::new`] 与 [`Parser::parse`] 的便捷入口，适合编译器前端
/// 和规格测试直接调用。
#[must_use]
pub fn parse(source: &SourceFile) -> ParseResult {
    Parser::new(source).parse()
}

/// 判断 Token 是否可以作为表达式中的名称。
///
/// 标量类型关键字在调用位置也作为名称保留，使 `bool(value)`、
/// `int(value)` 等构造式能够先进入统一调用 AST；类型检查阶段再决定
/// 其具体转换或构造语义。
fn is_expression_name_token(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::BacktickIdentifier
            | TokenKind::Keyword(
                KeywordKind::Int
                    | KeywordKind::Sint
                    | KeywordKind::Lint
                    | KeywordKind::Float
                    | KeywordKind::Sfloat
                    | KeywordKind::Lfloat
                    | KeywordKind::Str
                    | KeywordKind::Bool
            )
    )
}

/// 判断 Token 是否为 P2 支持的标量类型关键字。
fn is_scalar_type_token(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Keyword(
            KeywordKind::Int
                | KeywordKind::Sint
                | KeywordKind::Lint
                | KeywordKind::Float
                | KeywordKind::Sfloat
                | KeywordKind::Lfloat
                | KeywordKind::Str
                | KeywordKind::Bool
        )
    )
}

/// 判断 Token 是否可作为声明目标名称。
fn is_declaration_name_token(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Identifier | TokenKind::BacktickIdentifier)
}

/// 将词法赋值 Token 映射为 P1 赋值运算符。
fn assignment_operator(kind: TokenKind) -> Option<AssignmentOperator> {
    Some(match kind {
        TokenKind::Equal => AssignmentOperator::Assign,
        TokenKind::PlusEqual => AssignmentOperator::AddAssign,
        TokenKind::MinusEqual => AssignmentOperator::SubtractAssign,
        TokenKind::StarEqual => AssignmentOperator::MultiplyAssign,
        TokenKind::SlashEqual => AssignmentOperator::DivideAssign,
        TokenKind::FloorDivEqual => AssignmentOperator::FloorDivideAssign,
        TokenKind::PercentEqual => AssignmentOperator::RemainderAssign,
        TokenKind::PowerEqual => AssignmentOperator::PowerAssign,
        _ => return None,
    })
}

/// 判断表达式是否具备语法上的可赋值目标形状。
fn is_assignable_syntax(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Name(_) | Expression::Member { .. } | Expression::Selector { .. }
    )
}

/// 判断选择器当前位置是否已经到达一个项目边界。
fn selector_item_boundary(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Comma
            | TokenKind::RightBracket
            | TokenKind::Newline
            | TokenKind::Dedent
            | TokenKind::Eof
    )
}

/// 判断 Token 是否是 P0 顶层语句边界。
fn is_statement_boundary(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof
    )
}

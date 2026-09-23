//! 动态降低器的表达式、字面量与表字段访问发射。

use xiao_ir::{IrExpression, IrExpressionKind, IrName, IrSpan, IrType};

use super::predicate::name_key;
use super::text::{escape_bytes, format_float, is_identity_cast, parse_i32, parse_i64, unquote};
use super::{BYTES_TYPE, DynamicGenerator, VALUE_TYPE};
use crate::error::{CodegenError, Result};

impl<'a> DynamicGenerator<'a> {
    /// 发射一个表达式并返回 ABI 值 SSA 名称。
    pub(super) fn emit_expression(&mut self, expression: &IrExpression) -> Result<String> {
        match &expression.kind {
            IrExpressionKind::Literal { literal, text } => {
                self.emit_literal(literal, text, &expression.ty, expression.span)
            }
            IrExpressionKind::Name { name } => self.load_slot(name),
            IrExpressionKind::Group { expression } => self.emit_expression(expression),
            IrExpressionKind::Array { elements } => self.emit_sequence(elements, false),
            IrExpressionKind::Tuple { elements } => self.emit_sequence(elements, true),
            IrExpressionKind::Set { elements } => self.emit_set(elements),
            IrExpressionKind::DictTable { entries } => self.emit_dictionary(entries, 0),
            IrExpressionKind::DictColumn { entries } => self.emit_dictionary(entries, 1),
            IrExpressionKind::NewCall { callee, arguments } => {
                self.emit_new_call(callee, arguments, expression.span)
            }
            IrExpressionKind::Cast {
                expression: inner,
                target,
            } => {
                if !is_identity_cast(inner, target, &expression.ty) {
                    return Err(CodegenError::Unsupported {
                        feature: "动态 Cast（仅允许同类型转换）".to_owned(),
                        span: Some(expression.span),
                    });
                }
                self.emit_expression(inner)
            }
            IrExpressionKind::Member { object, member } => {
                self.emit_table_get(object, member, expression.span)
            }
            IrExpressionKind::Binary { .. }
            | IrExpressionKind::Unary { .. }
            | IrExpressionKind::Call { .. }
            | IrExpressionKind::Selector { .. } => Err(CodegenError::Unsupported {
                feature: "动态表达式运算或成员访问".to_owned(),
                span: Some(expression.span),
            }),
        }
    }

    /// 发射一个不拥有输入内存的 ABI 字节视图。
    pub(super) fn emit_bytes_value(&mut self, bytes: &[u8]) -> String {
        let global = format!("@.xiao.bytes{}", self.next_global);
        self.next_global += 1;
        self.globals.push(format!(
            "{global} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
            bytes.len() + 1,
            escape_bytes(bytes)
        ));
        let pointer = self.next_temp();
        self.emit(format!(
            "  {pointer} = getelementptr inbounds [{} x i8], ptr {global}, i64 0, i64 0",
            bytes.len() + 1
        ));
        let value = self.next_temp();
        self.emit(format!(
            "  {value} = insertvalue {BYTES_TYPE} zeroinitializer, ptr {pointer}, 0"
        ));
        let with_length = self.next_temp();
        self.emit(format!(
            "  {with_length} = insertvalue {BYTES_TYPE} {value}, i64 {}, 1",
            bytes.len()
        ));
        with_length
    }

    /// 从动态表值读取一个已类型检查的字段。
    fn emit_table_get(
        &mut self,
        object: &IrExpression,
        member: &IrName,
        span: IrSpan,
    ) -> Result<String> {
        if !matches!(object.ty, IrType::Table { .. }) {
            return Err(CodegenError::Unsupported {
                feature: "动态非表成员访问".to_owned(),
                span: Some(span),
            });
        }
        let object_value = self.emit_expression(object)?;
        let payload = self.next_temp();
        self.emit(format!(
            "  {payload} = extractvalue {VALUE_TYPE} {object_value}, 2"
        ));
        let handle = self.next_temp();
        self.emit(format!("  {handle} = inttoptr i64 {payload} to ptr"));
        let field = self.emit_bytes_value(name_key(member).as_bytes());
        let field_argument = self.emit_bytes_argument(&field);
        let output = self.next_temp();
        self.emit(format!("  {output} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {output}"
        ));
        self.checked_status_call(format!(
            "@xiao_runtime_table_get(ptr {handle}, {field_argument}, ptr {output})"
        ));
        let value = self.next_temp();
        self.emit(format!("  {value} = load {VALUE_TYPE}, ptr {output}"));
        self.release_value(object_value);
        Ok(value)
    }

    /// 向动态表写入一个字段值，并归还表达式临时所有权。
    pub(super) fn emit_table_set(
        &mut self,
        object: &IrExpression,
        member: &IrName,
        value: &IrExpression,
        span: IrSpan,
    ) -> Result<()> {
        if !matches!(object.ty, IrType::Table { .. }) {
            return Err(CodegenError::Unsupported {
                feature: "动态非表字段写入".to_owned(),
                span: Some(span),
            });
        }
        let object_value = self.emit_expression(object)?;
        let object_payload = self.next_temp();
        self.emit(format!(
            "  {object_payload} = extractvalue {VALUE_TYPE} {object_value}, 2"
        ));
        let object_handle = self.next_temp();
        self.emit(format!(
            "  {object_handle} = inttoptr i64 {object_payload} to ptr"
        ));
        let field = self.emit_bytes_value(name_key(member).as_bytes());
        let field_argument = self.emit_bytes_argument(&field);
        let emitted = self.emit_expression(value)?;
        let value_slot = self.next_temp();
        self.emit(format!("  {value_slot} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {emitted}, ptr {value_slot}"));
        self.checked_status_call(format!(
            "@xiao_runtime_table_set(ptr {object_handle}, {field_argument}, ptr {value_slot})"
        ));
        self.release_value(emitted);
        self.release_value(object_value);
        Ok(())
    }

    /// 发射标量或字符串字面量。
    fn emit_literal(
        &mut self,
        literal: &str,
        text: &str,
        ty: &IrType,
        span: IrSpan,
    ) -> Result<String> {
        let scalar_name = match ty {
            IrType::Scalar { name } => Some(name.as_str()),
            _ => None,
        };
        match literal {
            "integer" | "int" | "sint" => {
                if scalar_name == Some("lint") {
                    return self.emit_text_value(
                        text.replace('_', "").as_bytes(),
                        "xiao_runtime_value_lint",
                    );
                }
                if literal == "sint" || scalar_name == Some("sint") {
                    self.constructor_call("xiao_runtime_value_sint", "i32", &parse_i32(text, span)?)
                } else {
                    self.constructor_call("xiao_runtime_value_int", "i64", &parse_i64(text, span)?)
                }
            }
            "float" | "sfloat" => {
                if scalar_name == Some("lfloat") {
                    return self.emit_text_value(
                        text.replace('_', "").as_bytes(),
                        "xiao_runtime_value_lfloat",
                    );
                }
                if literal == "sfloat" || scalar_name == Some("sfloat") {
                    self.constructor_call(
                        "xiao_runtime_value_sfloat",
                        "float",
                        &format_float(text, span)?,
                    )
                } else {
                    self.constructor_call(
                        "xiao_runtime_value_float",
                        "double",
                        &format_float(text, span)?,
                    )
                }
            }
            "bool" => match text {
                "true" => self.constructor_call("xiao_runtime_value_bool", "i8", "1"),
                "false" => self.constructor_call("xiao_runtime_value_bool", "i8", "0"),
                _ => Err(CodegenError::InvalidIr {
                    message: format!("布尔字面量无法编码（{}..{}）", span.start, span.end),
                }),
            },
            "none" => Ok(self.none_value()),
            "str" => self.emit_string_literal(text, span, "xiao_runtime_value_str"),
            "lint" => self.emit_string_literal(text, span, "xiao_runtime_value_lint"),
            "lfloat" => self.emit_string_literal(text, span, "xiao_runtime_value_lfloat"),
            other => Err(CodegenError::Unsupported {
                feature: format!("动态字面量 {other}"),
                span: Some(span),
            }),
        }
    }

    /// 调用一个返回 ABI 值的标量构造器。
    fn constructor_call(&mut self, name: &str, ty: &str, value: &str) -> Result<String> {
        Ok(self.emit_value_call(name, &format!("{ty} {value}")))
    }

    /// 发射字符串全局常量、分配句柄并构造字符串值。
    fn emit_string_literal(
        &mut self,
        text: &str,
        span: IrSpan,
        constructor: &str,
    ) -> Result<String> {
        let parsed = unquote(text).ok_or_else(|| CodegenError::InvalidIr {
            message: format!("字符串字面量无法编码（{}..{}）", span.start, span.end),
        })?;
        self.emit_text_value(parsed.as_bytes(), constructor)
    }

    /// 发射一段已经解码的 UTF-8 文本，并调用指定的字符串类 Runtime 构造器。
    fn emit_text_value(&mut self, bytes: &[u8], constructor: &str) -> Result<String> {
        let global = format!("@.xiao.str{}", self.next_global);
        self.next_global += 1;
        self.globals.push(format!(
            "{global} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
            bytes.len() + 1,
            escape_bytes(bytes)
        ));
        let ptr = self.next_temp();
        self.emit(format!(
            "  {ptr} = getelementptr inbounds [{} x i8], ptr {global}, i64 0, i64 0",
            bytes.len() + 1
        ));
        let descriptor = self.next_temp();
        self.emit(format!(
            "  {descriptor} = insertvalue {BYTES_TYPE} zeroinitializer, ptr {ptr}, 0"
        ));
        let descriptor2 = self.next_temp();
        self.emit(format!(
            "  {descriptor2} = insertvalue {BYTES_TYPE} {descriptor}, i64 {}, 1",
            bytes.len()
        ));
        let input = self.emit_bytes_argument(&descriptor2);
        let handle = self.next_temp();
        self.emit(format!("  {handle} = alloca ptr"));
        self.emit(format!("  store ptr null, ptr {handle}"));
        self.checked_status_call(format!("@xiao_runtime_string_new({input}, ptr {handle})"));
        let raw = self.next_temp();
        self.emit(format!("  {raw} = load ptr, ptr {handle}"));
        let value = self.emit_value_call(constructor, &format!("ptr {raw}"));
        self.emit(format!("  call void @xiao_runtime_release(ptr {raw})"));
        Ok(value)
    }
}

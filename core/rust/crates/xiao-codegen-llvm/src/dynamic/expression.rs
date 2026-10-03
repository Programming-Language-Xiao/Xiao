//! 动态降低器的表达式、字面量与表字段访问发射。

use xiao_diagnostics::{CatchTypeKind, error_kind_of};
use xiao_ir::{IrCallArgument, IrExpression, IrExpressionKind, IrName, IrSpan, IrType};

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
            IrExpressionKind::Call { callee, arguments } => {
                self.emit_call(callee, arguments, expression.span)
            }
            IrExpressionKind::IntrinsicCall { id, arguments } => {
                self.emit_intrinsic(*id, arguments, expression.span)
            }
            IrExpressionKind::Binary { .. }
            | IrExpressionKind::Unary { .. }
            | IrExpressionKind::Selector { .. } => Err(CodegenError::Unsupported {
                feature: "动态表达式运算或成员访问".to_owned(),
                span: Some(expression.span),
            }),
        }
    }

    /// 发射由契约表标识的动态 intrinsic。`print` 走稳定 Runtime ABI；`input`
    /// 暂不在 LLVM 动态入口伪造实现，明确报告工具链边界。
    fn emit_intrinsic(
        &mut self,
        id: u32,
        arguments: &[IrCallArgument],
        span: IrSpan,
    ) -> Result<String> {
        let declaration = xiao_intrinsics::IntrinsicId::new(id)
            .and_then(xiao_intrinsics::active_by_id)
            .ok_or_else(|| CodegenError::InvalidIr {
                message: format!("未知或已废弃的 intrinsic ID {id}"),
            })?;
        match declaration.vm_binding {
            xiao_intrinsics::VmBinding::Print => {
                let mut values = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    if argument.kind != "positional" || argument.name.is_some() {
                        return Err(CodegenError::Unsupported {
                            feature: "print 的关键字或展开实参".to_owned(),
                            span: Some(argument.span),
                        });
                    }
                    values.push(self.emit_expression(&argument.value)?);
                }
                let array = if values.is_empty() {
                    None
                } else {
                    let array = self.next_temp();
                    self.emit(format!(
                        "  {array} = alloca {VALUE_TYPE}, i64 {}",
                        values.len()
                    ));
                    for (index, value) in values.iter().enumerate() {
                        let slot = self.next_temp();
                        self.emit(format!(
                            "  {slot} = getelementptr inbounds {VALUE_TYPE}, ptr {array}, i64 {index}"
                        ));
                        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
                    }
                    Some(array)
                };
                let pointer = array.as_deref().unwrap_or("null");
                self.checked_status_call_at(
                    format!(
                        "@xiao_runtime_print_values(ptr {pointer}, i64 {})",
                        values.len()
                    ),
                    span,
                );
                for value in values {
                    self.release_value(value);
                }
                Ok(self.none_value())
            }
            xiao_intrinsics::VmBinding::Input => {
                if arguments.len() > 1
                    || arguments
                        .iter()
                        .any(|argument| argument.kind != "positional" || argument.name.is_some())
                {
                    return Err(CodegenError::Unsupported {
                        feature: "input 的关键字或展开实参".to_owned(),
                        span: Some(span),
                    });
                }
                let (prompt, prompt_slot) = if let Some(argument) = arguments.first() {
                    let value = self.emit_expression(&argument.value)?;
                    let slot = self.next_temp();
                    self.emit(format!("  {slot} = alloca {VALUE_TYPE}"));
                    self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
                    (format!("ptr {slot}, i8 1"), Some(value))
                } else {
                    ("ptr null, i8 0".to_owned(), None)
                };
                let value = self.emit_value_call("xiao_runtime_input", &prompt);
                if let Some(prompt) = prompt_slot {
                    self.release_value(prompt);
                }
                self.check_pending_error_at(span);
                Ok(value)
            }
            _ => Err(CodegenError::Unsupported {
                feature: format!("动态 intrinsic {}", declaration.public_name),
                span: Some(span),
            }),
        }
    }

    /// 发射动态普通调用；当前只开放前端已识别的错误构造器。
    fn emit_call(
        &mut self,
        callee: &IrExpression,
        arguments: &[IrCallArgument],
        span: IrSpan,
    ) -> Result<String> {
        let IrExpressionKind::Name { name } = &callee.kind else {
            return Err(CodegenError::Unsupported {
                feature: "动态函数调用".to_owned(),
                span: Some(span),
            });
        };
        if error_kind_of(&name.text).is_some() {
            return self.emit_error_new(&name.text, arguments, span);
        }
        Err(CodegenError::Unsupported {
            feature: "动态函数调用".to_owned(),
            span: Some(span),
        })
    }

    /// 发射前端已定义的可恢复错误构造器。
    ///
    /// N0-C 只消费 `code`/`message` 两个字符串参数；不把普通动态调用误当作
    /// 错误构造，也不允许 `FatalError` 经由值 ABI 混入普通错误通道。
    pub(super) fn emit_error_new(
        &mut self,
        type_name: &str,
        arguments: &[IrCallArgument],
        span: IrSpan,
    ) -> Result<String> {
        if matches!(error_kind_of(type_name), Some(CatchTypeKind::Fatal)) {
            return Err(CodegenError::Unsupported {
                feature: "FatalError 不能构造为可恢复错误值".to_owned(),
                span: Some(span),
            });
        }
        if error_kind_of(type_name).is_none() {
            return Err(CodegenError::InvalidIr {
                message: format!("未知错误类型 {type_name}"),
            });
        }
        let mut code = None;
        let mut message = None;
        for (index, argument) in arguments.iter().enumerate() {
            let text = self.emit_error_text_argument(argument, span)?;
            let value = self.emit_text_value(text.as_bytes(), "xiao_runtime_value_str_owned")?;
            let slot = self.next_temp();
            self.emit(format!("  {slot} = alloca {VALUE_TYPE}"));
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
            match argument.name.as_ref().map(|name| name.text.as_str()) {
                Some("code") if code.is_none() => code = Some((slot, value)),
                Some("message") if message.is_none() => message = Some((slot, value)),
                Some(name @ ("code" | "message")) => {
                    return Err(CodegenError::InvalidIr {
                        message: format!("错误构造参数 {name} 重复"),
                    });
                }
                Some(name) => {
                    return Err(CodegenError::Unsupported {
                        feature: format!("错误构造参数 {name}"),
                        span: Some(argument.span),
                    });
                }
                None if index == 0 && code.is_none() => code = Some((slot, value)),
                None if index == 1 && message.is_none() => message = Some((slot, value)),
                None => {
                    return Err(CodegenError::Unsupported {
                        feature: "错误构造参数（只支持 code/message）".to_owned(),
                        span: Some(argument.span),
                    });
                }
            }
        }
        let type_value = self.emit_bytes_value(type_name.as_bytes());
        let type_argument = self.emit_bytes_argument(&type_value);
        let code_argument = code.as_ref().map_or("null", |(slot, _)| slot.as_str());
        let message_argument = message.as_ref().map_or("null", |(slot, _)| slot.as_str());
        let location = self.emit_error_location(span);
        let value = self.emit_value_call(
            "xiao_runtime_error_new_values",
            &format!(
                "{type_argument}, ptr {code_argument}, ptr {message_argument}, ptr {location}"
            ),
        );
        if let Some((_, code)) = code {
            self.release_value(code);
        }
        if let Some((_, message)) = message {
            self.release_value(message);
        }
        self.check_pending_error_at(span);
        Ok(value)
    }

    /// 发射错误构造参数中的稳定文本。
    fn emit_error_text_argument(
        &mut self,
        argument: &IrCallArgument,
        span: IrSpan,
    ) -> Result<String> {
        match &argument.value.kind {
            IrExpressionKind::Literal { literal, text } if literal == "str" => unquote(text)
                .ok_or_else(|| CodegenError::InvalidIr {
                    message: format!("错误构造字符串无法解码（{}..{}）", span.start, span.end),
                }),
            _ => Err(CodegenError::Unsupported {
                feature: "动态错误构造参数（当前只支持字符串字面量）".to_owned(),
                span: Some(argument.value.span),
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
            "  {payload} = extractvalue {VALUE_TYPE} {object_value}, 1"
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
            "  {object_payload} = extractvalue {VALUE_TYPE} {object_value}, 1"
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
                        "xiao_runtime_value_lint_owned",
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
                        "xiao_runtime_value_lfloat_owned",
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
            "str" => self.emit_string_literal(text, span, "xiao_runtime_value_str_owned"),
            "lint" => self.emit_string_literal(text, span, "xiao_runtime_value_lint_owned"),
            "lfloat" => self.emit_string_literal(text, span, "xiao_runtime_value_lfloat_owned"),
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
        Ok(value)
    }
}

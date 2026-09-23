//! N0-B 动态值降低器。
//!
//! 本模块与 N0-A 的静态标量降低器分开维护。它只在前端 IR 明确包含字符串、容器或表
//! 时启用，并通过 `xiao-runtime-abi` 的固定 `%xiao.value` 布局传递值。静态程序仍由
//! [`crate::ir::lower_static_program`] 生成原生 `i64`/`f64`/`i1`，不会因为本模块存在
//! 而链接 Runtime。

use std::collections::{BTreeMap, BTreeSet};

use xiao_ir::{
    IrExpression, IrExpressionKind, IrName, IrProgram, IrSpan, IrStatement, IrStatementKind, IrType,
};
use xiao_runtime_abi::ABI_ENCODED_VERSION;

use crate::CODEGEN_VERSION;
use crate::error::{CodegenError, Result};
use crate::ir::{CodegenOptions, LlvmModule, validate_program};
use crate::text::{escape_llvm, stable_hash};

#[path = "dynamic/entry.rs"]
mod entry;
#[path = "dynamic/predicate.rs"]
mod predicate;
#[path = "dynamic/release.rs"]
mod release;
#[path = "dynamic/runtime_abi.rs"]
mod runtime_abi;
#[path = "dynamic/slot.rs"]
mod slot;
#[path = "dynamic/text.rs"]
mod text;
pub(crate) use self::predicate::program_uses_runtime;
use self::predicate::{abi_field_type, name_key};
use self::text::{escape_bytes, format_float, is_identity_cast, parse_i32, parse_i64, unquote};

/// 动态值的固定 `{ i32, i32, i64 }` LLVM 结构名。
const VALUE_TYPE: &str = "%xiao.value";
/// 不拥有输入内存的 UTF-8 字节视图结构名。
const BYTES_TYPE: &str = "%xiao.bytes";
/// 表字段描述符的 LLVM 结构名。
const TABLE_FIELD_TYPE: &str = "%xiao.table.field";
/// 表描述符的 LLVM 结构名。
const TABLE_DESCRIPTOR_TYPE: &str = "%xiao.table.descriptor";

/// 将一份含动态值的 IR 降低为调用 Runtime ABI 的 LLVM 文本。
pub(crate) fn lower_program(program: &IrProgram, options: &CodegenOptions) -> Result<LlvmModule> {
    validate_program(program)?;
    if !program_uses_runtime(program) {
        return Err(CodegenError::InvalidIr {
            message: "动态降低器收到纯静态程序".to_owned(),
        });
    }
    if let Some(check) = program.runtime_checks.first() {
        return Err(CodegenError::Unsupported {
            feature: format!("动态运行时检查 {}（原生降低尚未接通）", check.kind),
            span: Some(check.span),
        });
    }
    if options.target.pointer_width != 64 {
        return Err(CodegenError::Unsupported {
            feature: "N0-B 当前只接受 64 位 C ABI 目标".to_owned(),
            span: Some(program.span),
        });
    }
    DynamicGenerator::new(program, options).generate()
}

/// 动态值局部槽。
#[derive(Clone, Copy, Debug)]
struct Slot {
    index: usize,
}

/// 当前动态 `while` 的条件与出口块。
#[derive(Clone, Debug)]
struct LoopLabels {
    condition: String,
    end: String,
}

/// 动态 LLVM 文本生成器。
struct DynamicGenerator<'a> {
    program: &'a IrProgram,
    options: &'a CodegenOptions,
    slots: BTreeMap<String, Slot>,
    value_slots: BTreeMap<u32, Slot>,
    declarations: BTreeSet<String>,
    globals: Vec<String>,
    lines: Vec<String>,
    next_temp: usize,
    next_global: usize,
    next_label: usize,
    terminated: bool,
    loop_stack: Vec<LoopLabels>,
    table_initializers: BTreeMap<String, Vec<(String, IrExpression)>>,
    observation_slot: Option<usize>,
    declared_runtime_components: BTreeSet<String>,
}

impl<'a> DynamicGenerator<'a> {
    /// 为一份已验证动态 IR 创建生成器。
    fn new(program: &'a IrProgram, options: &'a CodegenOptions) -> Self {
        Self {
            program,
            options,
            slots: BTreeMap::new(),
            value_slots: BTreeMap::new(),
            declarations: BTreeSet::new(),
            globals: Vec::new(),
            lines: Vec::new(),
            next_temp: 0,
            next_global: 0,
            next_label: 0,
            terminated: false,
            loop_stack: Vec::new(),
            table_initializers: BTreeMap::new(),
            observation_slot: None,
            declared_runtime_components: BTreeSet::new(),
        }
    }

    /// 生成 ABI 类型、声明、入口和释放序列。
    fn generate(mut self) -> Result<LlvmModule> {
        self.collect_table_initializers()?;
        self.collect_slots(self.program.body.as_slice())?;
        self.collect_value_slots();
        self.validate_release_scope_boundary()?;
        if self.options.entry_observation == crate::ir::EntryObservation::ExitCode {
            self.observation_slot = Some(self.slots.len());
        }
        self.declare_runtime();
        if self.options.debug_startup.is_some() {
            self.declarations
                .insert("declare i32 @xiao_native_debug_start()".to_owned());
        }
        self.emit_entry()?;
        let mut text = String::new();
        text.push_str("; Xiao N0-B LLVM dynamic module\n");
        text.push_str(&format!("; target = {}\n", self.options.target.triple));
        text.push_str(&format!(
            "target triple = \"{}\"\n\n",
            escape_llvm(&self.options.target.triple)
        ));
        text.push_str(&format!("{VALUE_TYPE} = type {{ i32, i32, i64 }}\n"));
        text.push_str(&format!("{BYTES_TYPE} = type {{ ptr, i64 }}\n\n"));
        text.push_str(&format!(
            "{TABLE_FIELD_TYPE} = type {{ {BYTES_TYPE}, i32, i8 }}\n"
        ));
        text.push_str(&format!(
            "{TABLE_DESCRIPTOR_TYPE} = type {{ {BYTES_TYPE}, i32, ptr, i64 }}\n\n"
        ));
        for global in &self.globals {
            text.push_str(global);
            text.push('\n');
        }
        if !self.globals.is_empty() {
            text.push('\n');
        }
        for declaration in self.declarations {
            text.push_str(&declaration);
            text.push('\n');
        }
        text.push('\n');
        text.push_str(&self.lines.join("\n"));
        text.push('\n');
        let components = self
            .declared_runtime_components
            .into_iter()
            .collect::<Vec<_>>();
        let fingerprint = format!(
            "xiao-codegen-{CODEGEN_VERSION}-{}",
            stable_hash(
                format!(
                    "{CODEGEN_VERSION};dynamic;abi={ABI_ENCODED_VERSION};{}",
                    self.options.target.fingerprint_fields()
                )
                .as_bytes()
            )
        );
        Ok(LlvmModule {
            text,
            target: self.options.target.clone(),
            entry_symbol: "xiao_entry".to_owned(),
            uses_runtime: true,
            runtime_components: components,
            runtime_abi_version: Some(ABI_ENCODED_VERSION),
            codegen_fingerprint: fingerprint,
        })
    }

    /// 发射一条顶层语句；异常展开留给 N0-C。
    fn emit_statement(&mut self, statement: &IrStatement) -> Result<()> {
        match &statement.kind {
            IrStatementKind::Assignment { target, value }
            | IrStatementKind::ConstDeclaration { target, value, .. } => {
                let value_type = value.ty.clone();
                let emitted = self.emit_expression(value)?;
                self.record_observation(&emitted, &value_type);
                self.store_slot(target, emitted)?;
            }
            IrStatementKind::Declaration { target, value, .. } => {
                let (emitted, value_type) = if let Some(value) = value {
                    let value_type = value.ty.clone();
                    (self.emit_expression(value)?, Some(value_type))
                } else {
                    (self.none_value(), None)
                };
                if let Some(value_type) = value_type {
                    self.record_observation(&emitted, &value_type);
                }
                self.store_slot(target, emitted)?;
            }
            IrStatementKind::Expression { value } => {
                let value_type = value.ty.clone();
                let emitted = self.emit_expression(value)?;
                self.record_observation(&emitted, &value_type);
                self.release_value(emitted);
            }
            IrStatementKind::Table {
                body, table_kind, ..
            } if body.is_empty() && table_kind != "singleton" => {}
            IrStatementKind::Table {
                name, table_kind, ..
            } => {
                if table_kind == "singleton" {
                    let callee = IrExpression {
                        kind: IrExpressionKind::Name { name: name.clone() },
                        ty: IrType::Table {
                            name: name.text.clone(),
                            kind: "constructor".to_owned(),
                        },
                        span: name.span,
                    };
                    let value = self.emit_new_call(&callee, &[], statement.span)?;
                    self.store_slot(name, value)?;
                }
            }
            IrStatementKind::Return { value } => {
                if let Some(value) = value {
                    let value_type = value.ty.clone();
                    let emitted = self.emit_expression(value)?;
                    self.record_observation(&emitted, &value_type);
                    self.release_value(emitted);
                }
                self.release_for_exit("return")?;
                self.emit_observation_return();
                self.terminated = true;
            }
            IrStatementKind::If {
                condition,
                body,
                elif_branches,
                else_body,
            } => self.emit_if(condition, body, elif_branches, else_body.as_deref())?,
            IrStatementKind::While { condition, body } => self.emit_while(condition, body)?,
            IrStatementKind::For { .. }
            | IrStatementKind::Function { .. }
            | IrStatementKind::Import { .. }
            | IrStatementKind::Try { .. }
            | IrStatementKind::Raise { .. } => {
                return Err(CodegenError::Unsupported {
                    feature: "动态模块中的控制流/函数/异常语句".to_owned(),
                    span: Some(statement.span),
                });
            }
            IrStatementKind::ExtendedAssignment {
                target,
                operator,
                value,
            } => {
                if operator != "=" {
                    return Err(CodegenError::Unsupported {
                        feature: "动态表字段复合赋值".to_owned(),
                        span: Some(statement.span),
                    });
                }
                let IrExpressionKind::Member { object, member } = &target.kind else {
                    return Err(CodegenError::Unsupported {
                        feature: "动态扩展赋值".to_owned(),
                        span: Some(statement.span),
                    });
                };
                self.emit_table_set(object, member, value, statement.span)?;
            }
            IrStatementKind::Break => {
                let Some(labels) = self.loop_stack.last().cloned() else {
                    return Err(CodegenError::InvalidIr {
                        message: "动态 break 不在循环中".to_owned(),
                    });
                };
                self.emit(format!("  br label %{}", labels.end));
                self.terminated = true;
            }
            IrStatementKind::Continue => {
                let Some(labels) = self.loop_stack.last().cloned() else {
                    return Err(CodegenError::InvalidIr {
                        message: "动态 continue 不在循环中".to_owned(),
                    });
                };
                self.emit(format!("  br label %{}", labels.condition));
                self.terminated = true;
            }
        }
        Ok(())
    }

    /// 在当前基本块依次发射语句，遇到终止边后停止。
    fn emit_statements(&mut self, statements: &[IrStatement]) -> Result<()> {
        for statement in statements {
            if self.terminated {
                break;
            }
            self.emit_statement(statement)?;
        }
        Ok(())
    }

    /// 发射布尔 `if`/`elif`/`else` 链并在可达分支汇合。
    fn emit_if(
        &mut self,
        condition: &IrExpression,
        body: &[IrStatement],
        elif_branches: &[xiao_ir::IrElifBranch],
        else_body: Option<&[IrStatement]>,
    ) -> Result<()> {
        let condition = self.emit_condition(condition)?;
        let then_label = self.next_label("dynamic.if.then");
        let else_label = self.next_label("dynamic.if.next");
        let merge_label = self.next_label("dynamic.if.merge");
        self.emit(format!(
            "  br i1 {condition}, label %{then_label}, label %{else_label}"
        ));
        self.terminated = true;
        self.emit_label(&then_label);
        self.emit_statements(body)?;
        if !self.terminated {
            self.emit(format!("  br label %{merge_label}"));
            self.terminated = true;
        }
        self.emit_label(&else_label);
        if elif_branches.is_empty() {
            if let Some(else_body) = else_body {
                self.emit_statements(else_body)?;
            }
        } else {
            self.emit_elif_chain(elif_branches, else_body, &merge_label)?;
        }
        if !self.terminated {
            self.emit(format!("  br label %{merge_label}"));
            self.terminated = true;
        }
        self.emit_label(&merge_label);
        Ok(())
    }

    /// 递归发射 `elif` 链。
    fn emit_elif_chain(
        &mut self,
        branches: &[xiao_ir::IrElifBranch],
        else_body: Option<&[IrStatement]>,
        merge_label: &str,
    ) -> Result<()> {
        let branch = &branches[0];
        let condition = self.emit_condition(&branch.condition)?;
        let then_label = self.next_label("dynamic.elif.then");
        let next_label = self.next_label("dynamic.elif.next");
        self.emit(format!(
            "  br i1 {condition}, label %{then_label}, label %{next_label}"
        ));
        self.terminated = true;
        self.emit_label(&then_label);
        self.emit_statements(&branch.body)?;
        if !self.terminated {
            self.emit(format!("  br label %{merge_label}"));
            self.terminated = true;
        }
        self.emit_label(&next_label);
        if branches.len() > 1 {
            self.emit_elif_chain(&branches[1..], else_body, merge_label)?;
        } else if let Some(else_body) = else_body {
            self.emit_statements(else_body)?;
        }
        Ok(())
    }

    /// 发射 `while` 基本块，并为 `break`/`continue` 暴露当前循环目标。
    fn emit_while(&mut self, condition: &IrExpression, body: &[IrStatement]) -> Result<()> {
        let condition_label = self.next_label("dynamic.while.cond");
        let body_label = self.next_label("dynamic.while.body");
        let end_label = self.next_label("dynamic.while.end");
        self.emit(format!("  br label %{condition_label}"));
        self.terminated = true;
        self.emit_label(&condition_label);
        let condition = self.emit_condition(condition)?;
        self.emit(format!(
            "  br i1 {condition}, label %{body_label}, label %{end_label}"
        ));
        self.terminated = true;
        self.emit_label(&body_label);
        self.loop_stack.push(LoopLabels {
            condition: condition_label.clone(),
            end: end_label.clone(),
        });
        let body_result = self.emit_statements(body);
        self.loop_stack.pop();
        body_result?;
        if !self.terminated {
            self.emit(format!("  br label %{condition_label}"));
            self.terminated = true;
        }
        self.emit_label(&end_label);
        Ok(())
    }

    /// 从 ABI 动态值读取已类型检查的布尔载荷。
    fn emit_condition(&mut self, expression: &IrExpression) -> Result<String> {
        if !matches!(&expression.ty, IrType::Scalar { name } if name == "bool") {
            return Err(CodegenError::Unsupported {
                feature: "动态路径中的非 bool 条件".to_owned(),
                span: Some(expression.span),
            });
        }
        match &expression.kind {
            IrExpressionKind::Unary { operator, operand } if operator == "not" => {
                let value = self.emit_condition(operand)?;
                let output = self.next_temp();
                self.emit(format!("  {output} = xor i1 {value}, true"));
                Ok(output)
            }
            IrExpressionKind::Binary {
                operator,
                left,
                right,
            } if matches!(operator.as_str(), "==" | "!=")
                && matches!(&left.ty, IrType::Scalar { name } if name == "bool")
                && matches!(&right.ty, IrType::Scalar { name } if name == "bool") =>
            {
                let left = self.emit_condition(left)?;
                let right = self.emit_condition(right)?;
                let output = self.next_temp();
                let predicate = if operator == "==" { "eq" } else { "ne" };
                self.emit(format!("  {output} = icmp {predicate} i1 {left}, {right}"));
                Ok(output)
            }
            _ => {
                let value = self.emit_expression(expression)?;
                let tag = self.next_temp();
                self.emit(format!("  {tag} = extractvalue {VALUE_TYPE} {value}, 0"));
                let tag_ok = self.next_temp();
                self.emit(format!("  {tag_ok} = icmp eq i32 {tag}, 1"));
                let valid_label = self.next_label("dynamic.bool.ok");
                self.emit(format!(
                    "  br i1 {tag_ok}, label %{valid_label}, label %abi.fail"
                ));
                self.terminated = true;
                self.emit_label(&valid_label);
                let payload = self.next_temp();
                self.emit(format!(
                    "  {payload} = extractvalue {VALUE_TYPE} {value}, 2"
                ));
                let output = self.next_temp();
                self.emit(format!("  {output} = trunc i64 {payload} to i1"));
                // `emit_expression` 返回拥有的 ABI 值；条件只借用其位载荷，
                // 在离开条件块前归还临时值，避免每次判断泄漏句柄。
                self.release_value(value);
                Ok(output)
            }
        }
    }

    /// 发射一个表达式并返回 ABI 值 SSA 名称。
    fn emit_expression(&mut self, expression: &IrExpression) -> Result<String> {
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
    fn emit_bytes_value(&mut self, bytes: &[u8]) -> String {
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
    fn emit_table_set(
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

    /// 发射数组或元组构造。
    fn emit_sequence(&mut self, elements: &[IrExpression], tuple: bool) -> Result<String> {
        let array = self.next_temp();
        self.emit(format!(
            "  {array} = alloca {VALUE_TYPE}, i64 {}",
            elements.len()
        ));
        let mut temporaries = Vec::with_capacity(elements.len());
        for (index, element) in elements.iter().enumerate() {
            let value = self.emit_expression(element)?;
            let ptr = self.next_temp();
            self.emit(format!(
                "  {ptr} = getelementptr {VALUE_TYPE}, ptr {array}, i64 {index}"
            ));
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {ptr}"));
            temporaries.push(value);
        }
        let handle = self.next_temp();
        self.emit(format!("  {handle} = alloca ptr"));
        self.emit(format!("  store ptr null, ptr {handle}"));
        let function = if tuple {
            "xiao_runtime_tuple_new"
        } else {
            "xiao_runtime_array_new"
        };
        self.checked_status_call(format!(
            "@{function}(ptr {array}, i64 {}, ptr {handle})",
            elements.len()
        ));
        for value in temporaries {
            self.release_value(value);
        }
        let raw = self.next_temp();
        self.emit(format!("  {raw} = load ptr, ptr {handle}"));
        let constructor = if tuple {
            "xiao_runtime_value_tuple"
        } else {
            "xiao_runtime_value_array"
        };
        let value = self.emit_value_call(constructor, &format!("ptr {raw}"));
        self.emit(format!("  call void @xiao_runtime_release(ptr {raw})"));
        Ok(value)
    }

    /// 发射字典构造及键字符串描述数组。
    fn emit_dictionary(&mut self, entries: &[xiao_ir::IrDictEntry], kind: u32) -> Result<String> {
        let keys = self.next_temp();
        self.emit(format!(
            "  {keys} = alloca {BYTES_TYPE}, i64 {}",
            entries.len()
        ));
        let values = self.next_temp();
        self.emit(format!(
            "  {values} = alloca {VALUE_TYPE}, i64 {}",
            entries.len()
        ));
        let mut temporaries = Vec::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            let key_global = format!("@.xiao.key{}", self.next_global);
            self.next_global += 1;
            let key = entry.key.as_bytes();
            self.globals.push(format!(
                "{key_global} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
                key.len() + 1,
                escape_bytes(key)
            ));
            let key_ptr = self.next_temp();
            self.emit(format!(
                "  {key_ptr} = getelementptr inbounds [{} x i8], ptr {key_global}, i64 0, i64 0",
                key.len() + 1
            ));
            let key_value = self.next_temp();
            self.emit(format!(
                "  {key_value} = insertvalue {BYTES_TYPE} zeroinitializer, ptr {key_ptr}, 0"
            ));
            let key_value2 = self.next_temp();
            self.emit(format!(
                "  {key_value2} = insertvalue {BYTES_TYPE} {key_value}, i64 {}, 1",
                key.len()
            ));
            let key_slot = self.next_temp();
            self.emit(format!(
                "  {key_slot} = getelementptr {BYTES_TYPE}, ptr {keys}, i64 {index}"
            ));
            self.emit(format!("  store {BYTES_TYPE} {key_value2}, ptr {key_slot}"));
            let value = self.emit_expression(&entry.value)?;
            let value_slot = self.next_temp();
            self.emit(format!(
                "  {value_slot} = getelementptr {VALUE_TYPE}, ptr {values}, i64 {index}"
            ));
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {value_slot}"));
            temporaries.push(value);
        }
        let handle = self.next_temp();
        self.emit(format!("  {handle} = alloca ptr"));
        self.emit(format!("  store ptr null, ptr {handle}"));
        self.checked_status_call(format!(
            "@xiao_runtime_dict_new(i32 {kind}, ptr {keys}, ptr {values}, i64 {}, ptr {handle})",
            entries.len()
        ));
        for value in temporaries {
            self.release_value(value);
        }
        let raw = self.next_temp();
        self.emit(format!("  {raw} = load ptr, ptr {handle}"));
        let value =
            self.emit_value_call("xiao_runtime_value_dict", &format!("ptr {raw}, i32 {kind}"));
        self.emit(format!("  call void @xiao_runtime_release(ptr {raw})"));
        Ok(value)
    }

    /// 发射集合构造。
    fn emit_set(&mut self, elements: &[IrExpression]) -> Result<String> {
        let array = self.next_temp();
        self.emit(format!(
            "  {array} = alloca {VALUE_TYPE}, i64 {}",
            elements.len()
        ));
        let mut temporaries = Vec::with_capacity(elements.len());
        for (index, element) in elements.iter().enumerate() {
            let value = self.emit_expression(element)?;
            let ptr = self.next_temp();
            self.emit(format!(
                "  {ptr} = getelementptr {VALUE_TYPE}, ptr {array}, i64 {index}"
            ));
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {ptr}"));
            temporaries.push(value);
        }
        let handle = self.next_temp();
        self.emit(format!("  {handle} = alloca ptr"));
        self.emit(format!("  store ptr null, ptr {handle}"));
        self.checked_status_call(format!(
            "@xiao_runtime_set_new(ptr {array}, i64 {}, ptr {handle})",
            elements.len()
        ));
        for value in temporaries {
            self.release_value(value);
        }
        let raw = self.next_temp();
        self.emit(format!("  {raw} = load ptr, ptr {handle}"));
        let value = self.emit_value_call("xiao_runtime_value_set", &format!("ptr {raw}"));
        self.emit(format!("  call void @xiao_runtime_release(ptr {raw})"));
        Ok(value)
    }

    /// 发射最小表构造调用；字段描述元数据来自 IR 单一来源。
    fn emit_new_call(
        &mut self,
        callee: &IrExpression,
        arguments: &[xiao_ir::IrCallArgument],
        span: IrSpan,
    ) -> Result<String> {
        let xiao_ir::IrExpressionKind::Name { name } = &callee.kind else {
            return Err(CodegenError::Unsupported {
                feature: "动态表构造目标".to_owned(),
                span: Some(span),
            });
        };
        let signature = self
            .program
            .table_signatures
            .iter()
            .find(|signature| signature.name == name.text)
            .cloned()
            .ok_or_else(|| CodegenError::InvalidIr {
                message: format!("表 {} 没有登记签名", name.text),
            })?;
        if !arguments.is_empty() {
            return Err(CodegenError::Unsupported {
                feature: "动态表构造参数（需运行时 init 调用支持）".to_owned(),
                span: Some(span),
            });
        }
        let descriptor = self.emit_table_descriptor(&signature)?;
        let handle = self.next_temp();
        self.emit(format!("  {handle} = alloca ptr"));
        self.emit(format!("  store ptr null, ptr {handle}"));
        self.checked_status_call(format!(
            "@xiao_runtime_table_new(ptr {descriptor}, ptr {handle})"
        ));
        let raw = self.next_temp();
        self.emit(format!("  {raw} = load ptr, ptr {handle}"));
        let initializers = self
            .table_initializers
            .get(&name.text)
            .cloned()
            .unwrap_or_default();
        for (field, initializer) in initializers {
            let value = self.emit_expression(&initializer)?;
            let field_bytes = self.emit_bytes_value(field.as_bytes());
            let field_argument = self.emit_bytes_argument(&field_bytes);
            let value_slot = self.next_temp();
            self.emit(format!("  {value_slot} = alloca {VALUE_TYPE}"));
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {value_slot}"));
            self.checked_status_call(format!(
                "@xiao_runtime_table_set(ptr {raw}, {field_argument}, ptr {value_slot})"
            ));
            self.release_value(value);
        }
        let value = self.emit_value_call("xiao_runtime_value_table", &format!("ptr {raw}"));
        self.emit(format!("  call void @xiao_runtime_release(ptr {raw})"));
        Ok(value)
    }

    /// 发射静态表描述符及字段数组。
    fn emit_table_descriptor(&mut self, signature: &xiao_ir::IrTableSignature) -> Result<String> {
        // 先还原一次 Runtime 签名，确保 ABI 描述仍由 xiao-ir 的唯一表接口来源校验。
        // 字段描述 ABI 目前不携带方法函数表，因此含方法的表必须显式拒绝，不能把方法
        // 静默伪装成字段。
        if signature.runtime_signature().is_none() {
            return Err(CodegenError::InvalidIr {
                message: format!("表 {} 的 IR 签名无法还原为 Runtime 签名", signature.name),
            });
        }
        if signature.members.iter().any(|member| member.method) {
            return Err(CodegenError::Unsupported {
                feature: "动态表方法（ABI 尚未携带函数表）".to_owned(),
                span: Some(signature.span),
            });
        }

        let field_count = signature.members.len();
        let fields = if field_count == 0 {
            "null".to_owned()
        } else {
            let fields = self.next_temp();
            self.emit(format!(
                "  {fields} = alloca {TABLE_FIELD_TYPE}, i64 {field_count}"
            ));
            fields
        };
        for (index, member) in signature.members.iter().enumerate() {
            let field_type =
                abi_field_type(&member.ty).ok_or_else(|| CodegenError::Unsupported {
                    feature: "动态表字段类型".to_owned(),
                    span: Some(member.span),
                })?;
            let key = member.name.as_bytes();
            let global = format!("@.xiao.field{}", self.next_global);
            self.next_global += 1;
            self.globals.push(format!(
                "{global} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
                key.len() + 1,
                escape_bytes(key)
            ));
            let ptr = self.next_temp();
            self.emit(format!(
                "  {ptr} = getelementptr inbounds [{} x i8], ptr {global}, i64 0, i64 0",
                key.len() + 1
            ));
            let bytes = self.next_temp();
            self.emit(format!(
                "  {bytes} = insertvalue {BYTES_TYPE} zeroinitializer, ptr {ptr}, 0"
            ));
            let bytes_with_len = self.next_temp();
            self.emit(format!(
                "  {bytes_with_len} = insertvalue {BYTES_TYPE} {bytes}, i64 {}, 1",
                key.len()
            ));
            let field_value0 = self.next_temp();
            self.emit(format!(
                "  {field_value0} = insertvalue {TABLE_FIELD_TYPE} zeroinitializer, {BYTES_TYPE} {bytes_with_len}, 0"
            ));
            let field_value1 = self.next_temp();
            self.emit(format!(
                "  {field_value1} = insertvalue {TABLE_FIELD_TYPE} {field_value0}, i32 {field_type}, 1"
            ));
            let field_value2 = self.next_temp();
            self.emit(format!(
                "  {field_value2} = insertvalue {TABLE_FIELD_TYPE} {field_value1}, i8 {}, 2",
                u8::from(member.public)
            ));
            let field = self.next_temp();
            self.emit(format!(
                "  {field} = getelementptr {TABLE_FIELD_TYPE}, ptr {fields}, i64 {index}"
            ));
            self.emit(format!(
                "  store {TABLE_FIELD_TYPE} {field_value2}, ptr {field}"
            ));
        }
        let name = signature.name.as_bytes();
        let name_global = format!("@.xiao.table{}", self.next_global);
        self.next_global += 1;
        self.globals.push(format!(
            "{name_global} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
            name.len() + 1,
            escape_bytes(name)
        ));
        let name_ptr = self.next_temp();
        self.emit(format!(
            "  {name_ptr} = getelementptr inbounds [{} x i8], ptr {name_global}, i64 0, i64 0",
            name.len() + 1
        ));
        let name_bytes0 = self.next_temp();
        self.emit(format!(
            "  {name_bytes0} = insertvalue {BYTES_TYPE} zeroinitializer, ptr {name_ptr}, 0"
        ));
        let name_bytes1 = self.next_temp();
        self.emit(format!(
            "  {name_bytes1} = insertvalue {BYTES_TYPE} {name_bytes0}, i64 {}, 1",
            name.len()
        ));
        let kind = match signature.kind.as_str() {
            "singleton" => 0,
            "instance" => 1,
            _ => {
                return Err(CodegenError::InvalidIr {
                    message: format!("表 {} 的种类无效", signature.name),
                });
            }
        };
        let descriptor = self.next_temp();
        self.emit(format!("  {descriptor} = alloca {TABLE_DESCRIPTOR_TYPE}"));
        let descriptor_value0 = self.next_temp();
        self.emit(format!(
            "  {descriptor_value0} = insertvalue {TABLE_DESCRIPTOR_TYPE} zeroinitializer, {BYTES_TYPE} {name_bytes1}, 0"
        ));
        let descriptor_value1 = self.next_temp();
        self.emit(format!(
            "  {descriptor_value1} = insertvalue {TABLE_DESCRIPTOR_TYPE} {descriptor_value0}, i32 {kind}, 1"
        ));
        let descriptor_value2 = self.next_temp();
        self.emit(format!(
            "  {descriptor_value2} = insertvalue {TABLE_DESCRIPTOR_TYPE} {descriptor_value1}, ptr {fields}, 2"
        ));
        let descriptor_value3 = self.next_temp();
        self.emit(format!(
            "  {descriptor_value3} = insertvalue {TABLE_DESCRIPTOR_TYPE} {descriptor_value2}, i64 {field_count}, 3"
        ));
        self.emit(format!(
            "  store {TABLE_DESCRIPTOR_TYPE} {descriptor_value3}, ptr {descriptor}"
        ));
        Ok(descriptor)
    }

    /// 追加一行 LLVM 文本。
    fn emit(&mut self, line: String) {
        self.lines.push(line);
    }

    /// 追加一个基本块标签，并把后续指令标记为可达。
    fn emit_label(&mut self, label: &str) {
        self.emit(format!("{label}:"));
        self.terminated = false;
    }

    /// 分配一个临时 SSA 名称。
    fn next_temp(&mut self) -> String {
        let temp = format!("%t{}", self.next_temp);
        self.next_temp += 1;
        temp
    }

    /// 分配一个带稳定前缀的基本块标签。
    fn next_label(&mut self, prefix: &str) -> String {
        let label = format!("{prefix}{}", self.next_label);
        self.next_label += 1;
        label
    }

    /// 检查一个 Runtime C ABI 状态码；失败边统一进入不可恢复 trap。
    fn check_status(&mut self, status: &str) {
        let ok = self.next_temp();
        let label = format!("abi.ok{}", self.next_label);
        self.next_label += 1;
        self.emit(format!("  {ok} = icmp eq i32 {status}, 0"));
        self.emit(format!("  br i1 {ok}, label %{label}, label %abi.fail"));
        self.emit(format!("{label}:"));
    }

    /// 发射一个返回状态码的 Runtime 调用并在继续前检查结果。
    fn checked_status_call(&mut self, call: String) -> String {
        let status = self.next_temp();
        self.emit(format!("  {status} = call i32 {call}"));
        self.check_status(&status);
        status
    }
}

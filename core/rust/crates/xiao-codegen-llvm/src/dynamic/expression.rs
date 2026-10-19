//! 动态降低器的表达式、字面量与表字段访问发射。

use xiao_diagnostics::{CatchTypeKind, error_kind_of};
use xiao_ir::{
    IrCallArgument, IrExpression, IrExpressionKind, IrName, IrPathSegmentKind, IrSelectorItem,
    IrSpan, IrType,
};

use super::predicate::name_key;
use super::text::{escape_bytes, format_float, is_identity_cast, parse_i32, parse_i64, unquote};
use super::{BYTES_TYPE, DynamicGenerator, VALUE_TYPE};
use crate::error::{CodegenError, Result};

/// 前端在顶层存在同名用户函数时给静态链接的集合函数使用的内部前缀。
const SET_METHOD_FUNCTION_PREFIX: &str = "__xiao_stdlib_set_";

impl<'a> DynamicGenerator<'a> {
    /// 发射一个表达式并返回 ABI 值 SSA 名称。
    pub(super) fn emit_expression(&mut self, expression: &IrExpression) -> Result<String> {
        let value = match &expression.kind {
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
                let value = self.emit_expression(inner)?;
                if is_identity_cast(inner, target, &expression.ty) {
                    Ok(value)
                } else {
                    let result = self.emit_cast_runtime(target, &value, expression.span);
                    self.release_value(value);
                    Ok(result)
                }
            }
            IrExpressionKind::Member { object, member } => {
                if matches!(expression.ty, IrType::Function { .. }) {
                    return Err(CodegenError::Unsupported {
                        feature: "动态表方法值（A3）".to_owned(),
                        span: Some(expression.span),
                    });
                }
                self.emit_table_get(object, member, expression.span)
            }
            IrExpressionKind::Call { callee, arguments } => {
                self.emit_call(callee, arguments, expression.span)
            }
            IrExpressionKind::IntrinsicCall { id, arguments } => {
                self.emit_intrinsic(*id, arguments, expression.span)
            }
            IrExpressionKind::Binary {
                operator,
                left,
                right,
            } => {
                if matches!(operator.as_str(), "and" | "or") {
                    self.emit_logical_short_circuit(operator, left, right, expression.span)
                } else {
                    let left_value = self.emit_expression(left)?;
                    let right_value = self.emit_expression(right)?;
                    let operation = self.binary_operation_name(operator, left, right)?;
                    let outer_target = self.error_target();
                    let failure_label = self.next_label("dynamic.binary.fail");
                    let continuation_label = self.next_label("dynamic.binary.continue");
                    self.push_error_context(failure_label.clone());
                    let result = self.emit_binary_runtime(
                        operation,
                        &left_value,
                        &right_value,
                        expression.span,
                    );
                    self.pop_error_context();
                    self.release_value(left_value.clone());
                    self.release_value(right_value.clone());
                    self.emit(format!("  br label %{continuation_label}"));
                    self.terminated = true;
                    self.emit_label(&failure_label);
                    self.release_value(left_value);
                    self.release_value(right_value);
                    self.emit(format!("  br label %{outer_target}"));
                    self.terminated = true;
                    self.emit_label(&continuation_label);
                    Ok(result)
                }
            }
            IrExpressionKind::Unary { operator, operand } => {
                let value = self.emit_expression(operand)?;
                let operation = match operator.as_str() {
                    "not" => "not",
                    "+" => "plus",
                    "-" => "minus",
                    other => {
                        return Err(CodegenError::Unsupported {
                            feature: format!("动态一元运算 {other}"),
                            span: Some(expression.span),
                        });
                    }
                };
                let result = self.emit_unary_runtime(operation, &value, expression.span);
                self.release_value(value);
                Ok(result)
            }
            IrExpressionKind::Selector {
                source,
                selector,
                step,
                selection_plan,
            } => self.emit_selector(
                source,
                selector,
                step.as_deref(),
                *selection_plan,
                expression.span,
            ),
        };
        let value = value?;
        self.emit_registered_checks(expression, &value);
        Ok(value)
    }

    /// 发射动态 ABI 值的短路逻辑运算。
    ///
    /// 条件通过统一的 `emit_condition` 读取并校验 bool 标签；结果在每条实际
    /// 执行边构造为 ABI bool 值后写入合流槽。这样右侧表达式及其临时值只会
    /// 出现在被选中的块中，和 VM 的 TAC 分支保持相同的生命周期边界。
    fn emit_logical_short_circuit(
        &mut self,
        operator: &str,
        left: &IrExpression,
        right: &IrExpression,
        span: IrSpan,
    ) -> Result<String> {
        let left = self.emit_condition(left)?;
        let result_slot = self.next_temp();
        self.emit(format!("  {result_slot} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {result_slot}"
        ));
        let right_label = self.next_label("dynamic.logical.right");
        let short_label = self.next_label("dynamic.logical.short");
        let merge_label = self.next_label("dynamic.logical.merge");
        let (if_true, if_false) = if operator == "and" {
            (&right_label, &short_label)
        } else {
            (&short_label, &right_label)
        };
        self.emit(format!(
            "  br i1 {left}, label %{if_true}, label %{if_false}"
        ));
        self.terminated = true;

        self.emit_label(&short_label);
        let short = self.emit_value_call(
            "xiao_runtime_value_bool",
            if operator == "or" { "i8 1" } else { "i8 0" },
        );
        self.emit(format!("  store {VALUE_TYPE} {short}, ptr {result_slot}"));
        self.emit(format!("  br label %{merge_label}"));
        self.terminated = true;

        self.emit_label(&right_label);
        let right = self.emit_condition(right)?;
        // `emit_condition` returns LLVM `i1`, while the frozen Runtime bool ABI uses
        // `i8`.  This is the only current boundary where a computed condition crosses
        // into that ABI, so zero-extend it explicitly before calling the constructor.
        let right_payload = self.next_temp();
        self.emit(format!("  {right_payload} = zext i1 {right} to i8"));
        let right_value =
            self.emit_value_call("xiao_runtime_value_bool", &format!("i8 {right_payload}"));
        self.emit(format!(
            "  store {VALUE_TYPE} {right_value}, ptr {result_slot}"
        ));
        self.emit(format!("  br label %{merge_label}"));
        self.terminated = true;

        self.emit_label(&merge_label);
        let result = self.next_temp();
        self.emit(format!("  {result} = load {VALUE_TYPE}, ptr {result_slot}"));
        let _ = span;
        Ok(result)
    }

    /// 消费不依赖具体操作符的前端检查；算术、集合和选择器检查由其 Runtime
    /// 操作入口直接产生稳定错误，避免把已知错误操作数误判为成功结果。
    fn emit_registered_checks(&mut self, expression: &IrExpression, value: &str) {
        self.emit_registered_checks_at(expression.span, value);
    }

    /// 形参入口和普通表达式消费同一份资源检查计划。
    pub(super) fn emit_registered_checks_at(&mut self, span: IrSpan, value: &str) {
        for check in self
            .program
            .runtime_checks
            .iter()
            .filter(|check| check.span == span)
        {
            if matches!(
                check.kind.as_str(),
                "resource_type" | "resource_array" | "resource_dictionary"
            ) {
                if let Some(IrType::Resource { kind }) = &check.expected {
                    self.emit_dynamic_check(&format!("{}:{kind}", check.kind), value, check.span);
                }
                continue;
            }
            if matches!(
                check.kind.as_str(),
                "string_boolean" | "set_hashability" | "dynamic_conversion"
            ) {
                self.emit_dynamic_check(&check.kind, value, check.span);
            }
        }
    }

    /// 把 IR 二元运算映射为 Runtime 唯一算子表名称。
    pub(super) fn binary_operation_name(
        &self,
        operator: &str,
        left: &IrExpression,
        right: &IrExpression,
    ) -> Result<&'static str> {
        let left_set = matches!(left.ty, IrType::Set { .. });
        let right_set = matches!(right.ty, IrType::Set { .. });
        if left_set || right_set {
            return Ok(match operator {
                "+" => "set_union",
                "&" => "set_intersection",
                "-" => "set_difference",
                "^" => "set_symmetric_difference",
                "==" => "set_eq",
                "!=" => "set_ne",
                "<" => "set_subset",
                "<=" => "set_subset_eq",
                ">" => "set_superset",
                ">=" => "set_superset_eq",
                "in" => "set_member",
                "not in" => "set_not_member",
                other => {
                    return Err(CodegenError::Unsupported {
                        feature: format!("动态集合运算 {other}"),
                        span: Some(left.span),
                    });
                }
            });
        }
        Ok(match operator {
            "+" => "add",
            "-" => "sub",
            "*" => "mul",
            "/" => "div",
            "//" => "floor_div",
            "%" => "rem",
            "**" => "pow",
            "==" => "eq",
            "!=" => "ne",
            "<" => "lt",
            "<=" => "le",
            ">" => "gt",
            ">=" => "ge",
            "in" => "set_member",
            "not in" => "set_not_member",
            other => {
                return Err(CodegenError::Unsupported {
                    feature: format!("动态二元运算 {other}"),
                    span: Some(left.span),
                });
            }
        })
    }

    /// 先覆盖单项精确索引；高级选择计划在后续逻辑中复用同一 Runtime 访问入口。
    fn emit_selector(
        &mut self,
        source: &IrExpression,
        selector: &xiao_ir::IrSelector,
        _step: Option<&IrExpression>,
        selection_plan: Option<u32>,
        span: IrSpan,
    ) -> Result<String> {
        if selector.items.len() != 1 {
            if _step.is_some()
                || selector
                    .items
                    .iter()
                    .any(|item| !matches!(item, IrSelectorItem::Exact { .. }))
            {
                return Err(CodegenError::Unsupported {
                    feature: "动态混合多项选择器".to_owned(),
                    span: Some(span),
                });
            }
            let plan = selection_plan
                .and_then(|id| self.program.selection_plans.get(id as usize))
                .or_else(|| {
                    self.program
                        .selection_plans
                        .iter()
                        .find(|plan| plan.span == span)
                })
                .ok_or_else(|| CodegenError::InvalidIr {
                    message: "多项选择器缺少规范选择计划".to_owned(),
                })?;
            let paths = plan.selected_paths.clone();
            let value = self.emit_expression(source)?;
            let selected = self.emit_selection_paths(&value, &paths, span)?;
            self.release_value(value);
            return Ok(selected);
        }
        let mut value = self.emit_expression(source)?;
        if let Some(step) = _step {
            let step_value = self.emit_expression(step)?;
            self.emit_dynamic_check("selector_step", &step_value, step.span);
            self.release_value(step_value);
        }
        let IrSelectorItem::Exact { path, .. } = &selector.items[0] else {
            match &selector.items[0] {
                IrSelectorItem::Random { count, mode, .. } => {
                    let count_value = self.emit_expression(count)?;
                    self.emit_dynamic_check("random_count", &count_value, count.span);
                    let count_payload = self.next_temp();
                    self.emit(format!(
                        "  {count_payload} = extractvalue {VALUE_TYPE} {count_value}, 1"
                    ));
                    let selected = self.emit_random_select(&value, &count_payload, mode, span);
                    self.release_value(count_value);
                    self.release_value(value);
                    return Ok(selected);
                }
                IrSelectorItem::Range { .. } | IrSelectorItem::OpenRange { .. } => {
                    self.emit_dynamic_check("selector_bounds", &value, span);
                }
                IrSelectorItem::All { .. } => {}
                _ => {}
            }
            let plan = selection_plan
                .and_then(|id| self.program.selection_plans.get(id as usize))
                .or_else(|| {
                    self.program
                        .selection_plans
                        .iter()
                        .find(|plan| plan.span == span)
                });
            let Some(plan) = plan else {
                self.release_value(value);
                return Err(CodegenError::Unsupported {
                    feature: "动态选择器缺少规范选择计划".to_owned(),
                    span: Some(span),
                });
            };
            let selected = self.emit_selection_paths(&value, &plan.selected_paths, span)?;
            self.release_value(value);
            return Ok(selected);
        };
        for segment in &path.segments {
            let next = match &segment.kind {
                IrPathSegmentKind::Name { name } => {
                    self.emit_dict_key_get(&value, &name.text, segment.span)
                }
                IrPathSegmentKind::Index { text, negative } => {
                    let raw = text.parse::<i64>().map_err(|_| CodegenError::InvalidIr {
                        message: format!("选择器索引无法解析：{text}"),
                    })?;
                    let index = if *negative { -raw } else { raw };
                    self.emit_iter_get(&value, &index.to_string(), span)
                }
            };
            self.release_value(value);
            value = next;
        }
        Ok(value)
    }

    /// 按类型阶段生成的静态路径镜像构造选择结果数组。
    fn emit_selection_paths(
        &mut self,
        source: &str,
        paths: &[xiao_ir::IrSelectionPath],
        span: IrSpan,
    ) -> Result<String> {
        let array = self.next_temp();
        self.emit(format!(
            "  {array} = alloca {VALUE_TYPE}, i64 {}",
            paths.len()
        ));
        let mut temporaries = Vec::with_capacity(paths.len());
        for (index, path) in paths.iter().enumerate() {
            let mut value = source.to_owned();
            for segment in path {
                let next = match segment {
                    xiao_ir::IrSelectionPathSegment::Key(key) => {
                        self.emit_dict_key_get(&value, key, span)
                    }
                    xiao_ir::IrSelectionPathSegment::Index { resolved, raw } => {
                        let index = resolved.map_or(*raw, |value| value as i128) as i64;
                        self.emit_iter_get(&value, &index.to_string(), span)
                    }
                };
                if value != source {
                    self.release_value(value);
                }
                value = next;
            }
            let ptr = self.next_temp();
            self.emit(format!(
                "  {ptr} = getelementptr {VALUE_TYPE}, ptr {array}, i64 {index}"
            ));
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {ptr}"));
            if value != source {
                temporaries.push(value);
            }
        }
        let handle = self.next_temp();
        self.emit(format!("  {handle} = alloca ptr"));
        self.emit(format!("  store ptr null, ptr {handle}"));
        self.checked_status_call(format!(
            "@xiao_runtime_array_new(ptr {array}, i64 {}, ptr {handle})",
            paths.len()
        ));
        for value in temporaries {
            self.release_value(value);
        }
        let raw = self.next_temp();
        self.emit(format!("  {raw} = load ptr, ptr {handle}"));
        Ok(self.emit_value_call("xiao_runtime_value_array_owned", &format!("ptr {raw}")))
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
            xiao_intrinsics::VmBinding::ScalarCast => {
                if arguments.len() != 1
                    || arguments[0].kind != "positional"
                    || arguments[0].name.is_some()
                {
                    return Err(CodegenError::Unsupported {
                        feature: format!("{} 的转换参数形态", declaration.public_name),
                        span: Some(span),
                    });
                }
                let value = self.emit_expression(&arguments[0].value)?;
                let result = self.emit_cast_runtime(declaration.public_name, &value, span);
                self.release_value(value);
                Ok(result)
            }
            xiao_intrinsics::VmBinding::CharConstructor => {
                if arguments.len() != 1
                    || arguments[0].kind != "positional"
                    || arguments[0].name.is_some()
                {
                    return Err(CodegenError::Unsupported {
                        feature: format!("{} 的构造参数形态", declaration.public_name),
                        span: Some(span),
                    });
                }
                let value = self.emit_expression(&arguments[0].value)?;
                let result = self.emit_char_runtime(&value, span);
                self.release_value(value);
                Ok(result)
            }
            xiao_intrinsics::VmBinding::Print => {
                let mut values = Vec::with_capacity(arguments.len());
                let mut deferred = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    if argument.kind != "positional" || argument.name.is_some() {
                        return Err(CodegenError::Unsupported {
                            feature: "print 的关键字或展开实参".to_owned(),
                            span: Some(argument.span),
                        });
                    }
                    let value = self.emit_expression(&argument.value)?;
                    let keep = !self.statement_temporaries.is_empty()
                        && super::predicate::is_heap_temporary(&argument.value);
                    if keep {
                        self.own_statement_temporary(&argument.value, &value);
                    }
                    deferred.push(keep);
                    values.push(value);
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
                        "@xiao_runtime_print_values_with_capabilities(ptr {pointer}, i64 {}, i64 {})",
                        values.len(),
                        self.options.capabilities
                    ),
                    span,
                );
                for (value, deferred) in values.into_iter().zip(deferred) {
                    if !deferred {
                        self.release_value(value);
                    }
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
                let value = self.emit_value_call(
                    "xiao_runtime_input_with_capabilities",
                    &format!("{prompt}, i64 {}", self.options.capabilities),
                );
                if let Some(prompt) = prompt_slot {
                    self.release_value(prompt);
                }
                self.check_pending_error_at(span);
                Ok(value)
            }
            xiao_intrinsics::VmBinding::ResourceOpenFile
            | xiao_intrinsics::VmBinding::ResourceOpenDirectory
            | xiao_intrinsics::VmBinding::ProcessSpawn => {
                if arguments.len() != 1
                    || arguments[0].kind != "positional"
                    || arguments[0].name.is_some()
                {
                    return Err(CodegenError::Unsupported {
                        feature: format!("{} 的参数形态", declaration.public_name),
                        span: Some(span),
                    });
                }
                let path = self.emit_expression(&arguments[0].value)?;
                let path_slot = self.next_temp();
                self.emit(format!("  {path_slot} = alloca {VALUE_TYPE}"));
                self.emit(format!("  store {VALUE_TYPE} {path}, ptr {path_slot}"));
                let symbol = match declaration.vm_binding {
                    xiao_intrinsics::VmBinding::ResourceOpenFile => {
                        "xiao_runtime_resource_open_file_with_capabilities"
                    }
                    xiao_intrinsics::VmBinding::ResourceOpenDirectory => {
                        "xiao_runtime_resource_open_directory_with_capabilities"
                    }
                    xiao_intrinsics::VmBinding::ProcessSpawn => {
                        "xiao_runtime_process_spawn_with_capabilities"
                    }
                    _ => unreachable!(),
                };
                let result = self.emit_value_call(
                    symbol,
                    &format!("ptr {path_slot}, i64 {}", self.options.capabilities),
                );
                self.release_value(path);
                self.check_pending_error_at(span);
                Ok(result)
            }
            xiao_intrinsics::VmBinding::ResourceClose
            | xiao_intrinsics::VmBinding::ProcessClose => {
                if arguments.len() != 1
                    || arguments[0].kind != "positional"
                    || arguments[0].name.is_some()
                {
                    return Err(CodegenError::Unsupported {
                        feature: format!("{} 的参数形态", declaration.public_name),
                        span: Some(span),
                    });
                }
                let value = self.emit_expression(&arguments[0].value)?;
                let value_slot = self.next_temp();
                self.emit(format!("  {value_slot} = alloca {VALUE_TYPE}"));
                self.emit(format!("  store {VALUE_TYPE} {value}, ptr {value_slot}"));
                let symbol = if declaration.vm_binding == xiao_intrinsics::VmBinding::ProcessClose {
                    "xiao_runtime_process_close_with_capabilities"
                } else {
                    "xiao_runtime_resource_close_with_capabilities"
                };
                self.checked_status_call_at(
                    format!(
                        "@{symbol}(ptr {value_slot}, i64 {})",
                        self.options.capabilities
                    ),
                    span,
                );
                self.release_value(value);
                Ok(self.none_value())
            }
            xiao_intrinsics::VmBinding::ResourceReadText
            | xiao_intrinsics::VmBinding::ProcessWait => {
                if arguments.len() != 1
                    || arguments[0].kind != "positional"
                    || arguments[0].name.is_some()
                {
                    return Err(CodegenError::Unsupported {
                        feature: format!("{} 的参数形态", declaration.public_name),
                        span: Some(span),
                    });
                }
                let value = self.emit_expression(&arguments[0].value)?;
                let value_slot = self.next_temp();
                self.emit(format!("  {value_slot} = alloca {VALUE_TYPE}"));
                self.emit(format!("  store {VALUE_TYPE} {value}, ptr {value_slot}"));
                let result = self.emit_value_call(
                    if declaration.vm_binding == xiao_intrinsics::VmBinding::ProcessWait {
                        "xiao_runtime_process_wait_with_capabilities"
                    } else {
                        "xiao_runtime_resource_read_text_with_capabilities"
                    },
                    &format!("ptr {value_slot}, i64 {}", self.options.capabilities),
                );
                self.release_value(value);
                self.check_pending_error_at(span);
                Ok(result)
            }
            xiao_intrinsics::VmBinding::MakeError => {
                self.emit_error_new(declaration.public_name, arguments, span)
            }
            xiao_intrinsics::VmBinding::SetNew => {
                if !arguments.is_empty() {
                    return Err(CodegenError::Unsupported {
                        feature: "set 构造器参数".to_owned(),
                        span: Some(span),
                    });
                }
                self.emit_set(&[])
            }
            xiao_intrinsics::VmBinding::SetAdd | xiao_intrinsics::VmBinding::SetRemove => {
                if arguments.len() != 2
                    || arguments
                        .iter()
                        .any(|argument| argument.kind != "positional" || argument.name.is_some())
                {
                    return Err(CodegenError::Unsupported {
                        feature: format!("{} 的参数形态", declaration.public_name),
                        span: Some(span),
                    });
                }
                let set = self.emit_expression(&arguments[0].value)?;
                let element = self.emit_expression(&arguments[1].value)?;
                let set_slot = self.next_temp();
                let element_slot = self.next_temp();
                self.emit(format!("  {set_slot} = alloca {VALUE_TYPE}"));
                self.emit(format!("  {element_slot} = alloca {VALUE_TYPE}"));
                self.emit(format!("  store {VALUE_TYPE} {set}, ptr {set_slot}"));
                self.emit(format!(
                    "  store {VALUE_TYPE} {element}, ptr {element_slot}"
                ));
                let result = if declaration.vm_binding == xiao_intrinsics::VmBinding::SetAdd {
                    self.checked_status_call_at(
                        format!("@xiao_runtime_set_add(ptr {set_slot}, ptr {element_slot})"),
                        span,
                    );
                    self.none_value()
                } else {
                    let out = self.next_temp();
                    self.emit(format!("  {out} = alloca i8"));
                    self.emit(format!("  store i8 0, ptr {out}"));
                    self.checked_status_call_at(
                        format!(
                            "@xiao_runtime_set_remove(ptr {set_slot}, ptr {element_slot}, ptr {out})"
                        ),
                        span,
                    );
                    let raw = self.next_temp();
                    self.emit(format!("  {raw} = load i8, ptr {out}"));
                    self.emit_value_call("xiao_runtime_value_bool", &format!("i8 {raw}"))
                };
                self.release_value(set);
                self.release_value(element);
                Ok(result)
            }
            _ => Err(CodegenError::Unsupported {
                feature: format!("动态 intrinsic {}", declaration.public_name),
                span: Some(span),
            }),
        }
    }

    /// 发射已静态解析的顶层函数、表方法与错误构造调用。
    fn emit_call(
        &mut self,
        callee: &IrExpression,
        arguments: &[IrCallArgument],
        span: IrSpan,
    ) -> Result<String> {
        if let IrExpressionKind::Member { object, member } = &callee.kind
            && matches!(&object.kind, IrExpressionKind::Name { name } if name.text == "random")
            && member.text == "seed"
        {
            let Some(argument) = arguments.first() else {
                return Err(CodegenError::Unsupported {
                    feature: "random.seed 缺少参数".to_owned(),
                    span: Some(span),
                });
            };
            let value = self.emit_expression(&argument.value)?;
            let slot = self.next_temp();
            self.emit(format!("  {slot} = alloca {VALUE_TYPE}"));
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
            self.checked_status_call_at(
                format!("@xiao_runtime_random_seed(ptr {slot})"),
                argument.value.span,
            );
            self.release_value(value);
            return Ok(self.none_value());
        }
        if let IrExpressionKind::Member { object, member } = &callee.kind
            && matches!(object.ty, IrType::Set { .. })
            && !member.backticked
            && matches!(
                member.text.as_str(),
                "size"
                    | "is_empty"
                    | "add"
                    | "remove"
                    | "contains"
                    | "union"
                    | "intersection"
                    | "difference"
            )
        {
            let internal_name = format!("{SET_METHOD_FUNCTION_PREFIX}{}", member.text);
            let function_name = if self.function_definitions.contains_key(&internal_name) {
                internal_name
            } else if self.function_definitions.contains_key(&member.text) {
                member.text.clone()
            } else {
                return Err(CodegenError::Unsupported {
                    feature: format!("集合方法 {} 未静态链接", member.text),
                    span: Some(span),
                });
            };
            let mut bound = Vec::with_capacity(arguments.len() + 1);
            bound.push(IrCallArgument {
                kind: "positional".to_owned(),
                name: None,
                value: (**object).clone(),
                span: object.span,
            });
            bound.extend(arguments.iter().cloned());
            return self.emit_function_call(&function_name, &bound, span);
        }
        if let IrExpressionKind::Member { object, member } = &callee.kind
            && matches!(object.ty, IrType::Table { .. })
        {
            return self.emit_table_method_call(object, member, arguments, span);
        }
        let IrExpressionKind::Name { name } = &callee.kind else {
            return Err(CodegenError::Unsupported {
                feature: "动态函数调用".to_owned(),
                span: Some(span),
            });
        };
        if self.function_definitions.contains_key(&name.text) {
            return self.emit_function_call(&name.text, arguments, span);
        }
        if error_kind_of(&name.text).is_some() {
            return self.emit_error_new(&name.text, arguments, span);
        }
        Err(CodegenError::Unsupported {
            feature: "动态函数调用".to_owned(),
            span: Some(span),
        })
    }

    /// 调用一个顶层动态函数；参数和返回值统一经 `%xiao.value` 指针槽传递。
    fn emit_function_call(
        &mut self,
        name: &str,
        arguments: &[IrCallArgument],
        span: IrSpan,
    ) -> Result<String> {
        let Some(statement) = self.function_definitions.get(name) else {
            return Err(CodegenError::InvalidIr {
                message: format!("函数 {name} 未登记"),
            });
        };
        let xiao_ir::IrStatementKind::Function { parameters, .. } = &statement.kind else {
            return Err(CodegenError::InvalidIr {
                message: format!("符号 {name} 不是函数"),
            });
        };
        if arguments.len() != parameters.len()
            || arguments
                .iter()
                .any(|argument| argument.kind != "positional" || argument.name.is_some())
        {
            return Err(CodegenError::Unsupported {
                feature: format!("动态函数 {name} 的参数形态"),
                span: Some(span),
            });
        }
        let mut argument_slots = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let value = self.emit_expression(&argument.value)?;
            let slot = self.next_temp();
            self.emit(format!("  {slot} = alloca {VALUE_TYPE}"));
            self.emit(format!("  store {VALUE_TYPE} zeroinitializer, ptr {slot}"));
            self.store_call_argument(&argument.value, &argument.value, &value, &slot);
            argument_slots.push(slot);
        }
        let output = self.next_temp();
        self.emit(format!("  {output} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {output}"
        ));
        let symbol = self.function_symbol(name);
        let mut call = format!("  call void @{symbol}(ptr {output}");
        for slot in &argument_slots {
            call.push_str(&format!(", ptr {slot}"));
        }
        call.push(')');
        self.emit(call);
        let release = self.value_release_symbol();
        for slot in &argument_slots {
            self.emit(format!("  call void @{release}(ptr {slot})"));
        }
        let class = self.next_temp();
        self.emit(format!("  {class} = call i32 @xiao_runtime_error_class()"));
        let ok = self.next_temp();
        self.emit(format!("  {ok} = icmp eq i32 {class}, 0"));
        let ok_label = self.next_label("dynamic.call.ok");
        let fail_label = self.next_label("dynamic.call.fail");
        self.emit(format!(
            "  br i1 {ok}, label %{ok_label}, label %{fail_label}"
        ));
        self.terminated = true;
        self.emit_label(&fail_label);
        self.emit(format!("  br label %{}", self.error_target()));
        self.terminated = true;
        self.emit_label(&ok_label);
        let result = self.next_temp();
        self.emit(format!("  {result} = load {VALUE_TYPE}, ptr {output}"));
        Ok(result)
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
            // Runtime 的 `new_values` 负责检查参数是否为文本，这样变量、函数返回值和
            // 动态值都能与 VM 共享同一类型错误身份。
            // Runtime 入口只读参数槽并自行取得文本；已经存在的名称槽可以直接借用，
            // 避免先复制一次再由 Runtime 再复制读取。复杂表达式仍放入拥有的临时槽。
            let (slot, owned_value) =
                if let Some(slot) = self.borrowed_error_argument_slot(&argument.value) {
                    (slot, None)
                } else {
                    let value = self.emit_expression(&argument.value)?;
                    let slot = self.next_temp();
                    self.emit(format!("  {slot} = alloca {VALUE_TYPE}"));
                    self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
                    (slot, Some(value))
                };
            match argument.name.as_ref().map(|name| name.text.as_str()) {
                Some("code") if code.is_none() => code = Some((slot, owned_value)),
                Some("message") if message.is_none() => message = Some((slot, owned_value)),
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
                None if index == 0 && code.is_none() => code = Some((slot, owned_value)),
                None if index == 1 && message.is_none() => message = Some((slot, owned_value)),
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
        if let Some((_, Some(code))) = code {
            self.release_value(code);
        }
        if let Some((_, Some(message))) = message {
            self.release_value(message);
        }
        self.check_pending_error_at(span);
        Ok(value)
    }

    /// 返回名称或分组名称对应的现有 ABI 槽，供只读 Runtime 参数直接借用。
    fn borrowed_error_argument_slot(&self, expression: &IrExpression) -> Option<String> {
        match &expression.kind {
            IrExpressionKind::Group { expression } => self.borrowed_error_argument_slot(expression),
            IrExpressionKind::Name { name } => self
                .slots
                .get(&name_key(name))
                .or_else(|| self.slots.get(&name.text))
                .map(|slot| format!("%slot{}", slot.index)),
            _ => None,
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
        if matches!(object.ty, IrType::Dynamic | IrType::DictTable { .. }) {
            let object_value = self.emit_expression(object)?;
            let object_slot = self.next_temp();
            self.emit(format!("  {object_slot} = alloca {VALUE_TYPE}"));
            self.emit(format!(
                "  store {VALUE_TYPE} {object_value}, ptr {object_slot}"
            ));
            let field = self.emit_bytes_value(name_key(member).as_bytes());
            let field_argument = self.emit_bytes_argument(&field);
            let output = self.next_temp();
            self.emit(format!("  {output} = alloca {VALUE_TYPE}"));
            self.emit(format!(
                "  store {VALUE_TYPE} zeroinitializer, ptr {output}"
            ));
            self.checked_status_call_at(
                format!(
                    "@xiao_runtime_dynamic_member_get(ptr {object_slot}, {field_argument}, ptr {output})"
                ),
                span,
            );
            let value = self.next_temp();
            self.emit(format!("  {value} = load {VALUE_TYPE}, ptr {output}"));
            self.release_value(object_value);
            return Ok(value);
        }
        if !matches!(object.ty, IrType::Table { .. }) {
            return Err(CodegenError::Unsupported {
                feature: "动态非表成员访问".to_owned(),
                span: Some(span),
            });
        }
        self.emit_callback_field_get(object, member, span)
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
        self.emit_callback_field_set(object, member, value, span)
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
                        xiao_types::normalize_decimal_zero(text).as_bytes(),
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

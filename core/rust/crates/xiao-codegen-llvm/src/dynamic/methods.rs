//! 表方法回调、字段初始化辅助函数与固定签名调用；类型事实只消费 IR。

use xiao_ir::{
    IrCallArgument, IrExpression, IrExpressionKind, IrName, IrParameter, IrSpan, IrStatement,
    IrStatementKind, IrType,
};
use xiao_runtime_abi::{TABLE_METHOD_DYNAMIC_TYPE, XiaoValueTag, table_method_signature_id};

use super::predicate::name_key;
use super::{
    BYTES_TYPE, DynamicGenerator, TABLE_DESCRIPTOR_TYPE, TABLE_DESCRIPTOR_V2_TYPE,
    TABLE_METHOD_TYPE, VALUE_TYPE,
};
use crate::error::{CodegenError, Result};

/// 同一函数发射器的 C 回调配置，不另建方法执行/清理语义。
#[derive(Clone)]
pub(super) struct CallbackContext {
    /// 稳定的表内函数符号。
    pub(super) symbol: String,
    /// 字段辅助函数没有源码函数作用域，按合成帧持有的槽归还。
    pub(super) fields: bool,
}

/// 从静态 IR 类型映射既有 ABI 值标签，拒绝未知类型而非另做推断。
pub(super) fn method_type(ty: &IrType) -> Result<u32> {
    let tag = match ty {
        IrType::None => XiaoValueTag::None,
        IrType::Dynamic | IrType::Variable { .. } => return Ok(TABLE_METHOD_DYNAMIC_TYPE),
        IrType::Scalar { name } => match name.as_str() {
            "bool" => XiaoValueTag::Bool,
            "int" => XiaoValueTag::Int,
            "sint" => XiaoValueTag::Sint,
            "lint" => XiaoValueTag::Lint,
            "float" => XiaoValueTag::Float,
            "sfloat" => XiaoValueTag::Sfloat,
            "lfloat" => XiaoValueTag::Lfloat,
            "str" => XiaoValueTag::Str,
            _ => {
                return Err(CodegenError::InvalidIr {
                    message: format!("未知方法标量类型 {name}"),
                });
            }
        },
        IrType::Table { .. } => XiaoValueTag::Table,
        IrType::Array { .. } => XiaoValueTag::Array,
        IrType::Tuple { .. } => XiaoValueTag::Tuple,
        IrType::DictTable { .. } => XiaoValueTag::DictTable,
        IrType::DictColumn { .. } => XiaoValueTag::DictColumn,
        IrType::Set { .. } => XiaoValueTag::Set,
        IrType::Function { .. } => {
            return Err(CodegenError::Unsupported {
                feature: "动态函数值 ABI（A3）".to_owned(),
                span: None,
            });
        }
    };
    Ok(tag.raw())
}

impl<'a> DynamicGenerator<'a> {
    /// 把已检查成员签名与预登记回调写入独立 V2 描述符；Runtime 会复制所有元数据。
    pub(super) fn emit_table_descriptor_v2(
        &mut self,
        signature: &xiao_ir::IrTableSignature,
        fields: &str,
    ) -> Result<String> {
        let methods = signature
            .members
            .iter()
            .filter(|member| member.method)
            .collect::<Vec<_>>();
        let method_array = if methods.is_empty() {
            "null".to_owned()
        } else {
            let array = self.next_temp();
            self.emit(format!(
                "  {array} = alloca {TABLE_METHOD_TYPE}, i64 {}",
                methods.len()
            ));
            array
        };
        let mut init = "null".to_owned();
        let mut drop = "null".to_owned();
        for (index, member) in methods.iter().enumerate() {
            let statement = self
                .method_definitions
                .get(&(signature.name.clone(), member.name.clone()))
                .copied()
                .ok_or_else(|| CodegenError::InvalidIr {
                    message: format!("表 {} 方法 {} 缺少函数体", signature.name, member.name),
                })?;
            let IrStatementKind::Function {
                parameters,
                return_type,
                ..
            } = &statement.kind
            else {
                unreachable!()
            };
            if parameters.is_empty() {
                return Err(CodegenError::InvalidIr {
                    message: "表方法缺少 receiver 参数".to_owned(),
                });
            }
            if member.name == "ascii:init" && parameters.len() != 1 {
                return Err(CodegenError::Unsupported {
                    feature: "动态表构造参数（A2）".to_owned(),
                    span: Some(statement.span),
                });
            }
            let parameter_types = parameters[1..]
                .iter()
                .map(|parameter| method_type(&parameter.ty))
                .collect::<Result<Vec<_>>>()?;
            let result_type = method_type(return_type)?;
            let id = table_method_signature_id(&parameter_types, result_type);
            let types = if parameter_types.is_empty() {
                "null".to_owned()
            } else {
                let types = self.next_temp();
                self.emit(format!(
                    "  {types} = alloca i32, i64 {}",
                    parameter_types.len()
                ));
                for (index, ty) in parameter_types.iter().enumerate() {
                    let ptr = self.next_temp();
                    self.emit(format!(
                        "  {ptr} = getelementptr i32, ptr {types}, i64 {index}"
                    ));
                    self.emit(format!("  store i32 {ty}, ptr {ptr}"));
                }
                types
            };
            let callback = format!(
                "@{}",
                self.table_callback_symbol(&signature.name, Some(&member.name))
            );
            if member.name == "ascii:init" {
                init = callback.clone();
            }
            if member.name == "ascii:drop" {
                drop = callback.clone();
            }
            let name = self.emit_bytes_value(member.name.as_bytes());
            let method = self.emit_metadata_value(
                TABLE_METHOD_TYPE,
                &[
                    format!("{BYTES_TYPE} {name}"),
                    format!("i64 {id}"),
                    format!("ptr {types}"),
                    format!("i64 {}", parameter_types.len()),
                    format!("i32 {result_type}"),
                    format!("i8 {}", u8::from(member.public)),
                    format!("ptr {callback}"),
                ],
            );
            let ptr = self.next_temp();
            self.emit(format!(
                "  {ptr} = getelementptr {TABLE_METHOD_TYPE}, ptr {method_array}, i64 {index}"
            ));
            self.emit(format!("  store {TABLE_METHOD_TYPE} {method}, ptr {ptr}"));
        }
        let fields_value = self.next_temp();
        self.emit(format!(
            "  {fields_value} = load {TABLE_DESCRIPTOR_TYPE}, ptr {fields}"
        ));
        let fields_callback = if self.table_initializers.contains_key(&signature.name) {
            format!("@{}", self.table_callback_symbol(&signature.name, None))
        } else {
            "null".to_owned()
        };
        let value = self.emit_metadata_value(
            TABLE_DESCRIPTOR_V2_TYPE,
            &[
                format!(
                    "i32 {}",
                    std::mem::size_of::<xiao_runtime_abi::XiaoTableDescriptorV2>()
                ),
                format!("i32 {}", xiao_runtime_abi::TABLE_DESCRIPTOR_VERSION),
                format!("{TABLE_DESCRIPTOR_TYPE} {fields_value}"),
                format!("ptr {method_array}"),
                format!("i64 {}", methods.len()),
                format!("ptr {fields_callback}"),
                format!("ptr {init}"),
                format!("ptr {drop}"),
            ],
        );
        let descriptor = self.next_temp();
        self.emit(format!(
            "  {descriptor} = alloca {TABLE_DESCRIPTOR_V2_TYPE}"
        ));
        self.emit(format!(
            "  store {TABLE_DESCRIPTOR_V2_TYPE} {value}, ptr {descriptor}"
        ));
        Ok(descriptor)
    }

    /// 按 ABI 字段顺序组装纯元数据聚合值，避免把 Rust 内部对象布局写入 LLVM。
    fn emit_metadata_value(&mut self, ty: &str, fields: &[String]) -> String {
        let mut value = "zeroinitializer".to_owned();
        for (index, field) in fields.iter().enumerate() {
            let next = self.next_temp();
            self.emit(format!(
                "  {next} = insertvalue {ty} {value}, {field}, {index}"
            ));
            value = next;
        }
        value
    }

    /// 当前局部槽不可被被调函数重新绑定；无额外前端检查的名称可直接借给 ABI。
    pub(super) fn borrow_named_slot(&self, expression: &IrExpression) -> Option<String> {
        if self.program.runtime_checks.iter().any(|check| {
            check.span == expression.span
                && matches!(
                    check.kind.as_str(),
                    "string_boolean" | "set_hashability" | "dynamic_conversion"
                )
        }) {
            return None;
        }
        if let IrExpressionKind::Group { expression } = &expression.kind {
            return self.borrow_named_slot(expression);
        }
        let IrExpressionKind::Name { name } = &expression.kind else {
            return None;
        };
        self.slots
            .get(&name_key(name))
            .or_else(|| self.slots.get(&name.text))
            .map(|slot| format!("%slot{}", slot.index))
    }

    /// 物化一个调用期间有效的 receiver 指针，并标记是否拥有临时引用。
    fn field_receiver(&mut self, object: &IrExpression) -> Result<(String, bool)> {
        if let Some(slot) = self.borrow_named_slot(object) {
            return Ok((slot, false));
        }
        let value = self.emit_expression(object)?;
        if super::predicate::is_heap_temporary(object) {
            let slot = self.own_statement_temporary(object, &value);
            return Ok((slot, false));
        }
        let slot = self.empty_value_slot();
        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
        Ok((slot, true))
    }

    /// 预先建立空槽，使任意求值失败边都能安全清理已求值的部分。
    fn empty_value_slot(&mut self) -> String {
        let slot = self.next_temp();
        self.emit(format!("  {slot} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} zeroinitializer, ptr {slot}"));
        slot
    }

    /// 弱析构接收者和普通接收者共用值指针字段入口。
    pub(super) fn emit_callback_field_get(
        &mut self,
        object: &IrExpression,
        member: &IrName,
        span: IrSpan,
    ) -> Result<String> {
        self.emit_callback_field(object, member, None, span)
            .map(|value| value.expect("字段读取结果"))
    }

    /// 字段赋值在求值失败和 ABI 失败两条边都归还临时拥有值。
    pub(super) fn emit_callback_field_set(
        &mut self,
        object: &IrExpression,
        member: &IrName,
        value: &IrExpression,
        span: IrSpan,
    ) -> Result<()> {
        self.emit_callback_field(object, member, Some(value), span)
            .map(|_| ())
    }

    /// 名称槽直接借用，表达式临时槽显式拥有；不把弱视图提取成强句柄。
    fn emit_callback_field(
        &mut self,
        object: &IrExpression,
        member: &IrName,
        input: Option<&IrExpression>,
        span: IrSpan,
    ) -> Result<Option<String>> {
        let (receiver, receiver_owned) = self.field_receiver(object)?;
        let borrowed_input = input.and_then(|value| self.borrow_named_slot(value));
        let input_owned = borrowed_input.is_none();
        let slot = borrowed_input.unwrap_or_else(|| self.empty_value_slot());
        let outer = self.error_target();
        let failure = self.next_label("table.field.fail");
        let continuation = self.next_label("table.field.continue");
        self.push_error_context(failure.clone());
        if let Some(input) = input
            && input_owned
        {
            let value = self.emit_expression(input)?;
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {slot}"));
        }
        let field = self.emit_bytes_value(name_key(member).as_bytes());
        let field = self.emit_bytes_argument(&field);
        let function = if input.is_some() {
            "xiao_runtime_table_set_value"
        } else {
            "xiao_runtime_table_get_value"
        };
        self.checked_status_call_at(
            format!("@{function}(ptr {receiver}, {field}, ptr {slot})"),
            span,
        );
        self.pop_error_context();
        if input.is_some() && input_owned {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_any(ptr {slot})"
            ));
        }
        if receiver_owned {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_any(ptr {receiver})"
            ));
        }
        self.emit(format!("  br label %{continuation}"));
        self.emit_label(&failure);
        if input_owned {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_any(ptr {slot})"
            ));
        }
        if receiver_owned {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_any(ptr {receiver})"
            ));
        }
        self.emit(format!("  br label %{outer}"));
        self.emit_label(&continuation);
        if input.is_some() {
            return Ok(None);
        }
        let result = self.next_temp();
        self.emit(format!("  {result} = load {VALUE_TYPE}, ptr {slot}"));
        Ok(Some(result))
    }

    /// 复合字段赋值只求值一次 receiver，按读取、右值、运算、写回的顺序执行。
    pub(super) fn emit_table_compound_set(
        &mut self,
        target: &IrExpression,
        operator: &str,
        value: &IrExpression,
        span: IrSpan,
    ) -> Result<()> {
        let IrExpressionKind::Member { object, member } = &target.kind else {
            unreachable!()
        };
        let operation =
            self.binary_operation_name(operator.trim_end_matches('='), target, value)?;
        let (receiver, receiver_owned) = self.field_receiver(object)?;
        let left_slot = self.empty_value_slot();
        let result_slot = self.empty_value_slot();
        let borrowed_right = self.borrow_named_slot(value);
        let right_owned = borrowed_right.is_none();
        let right_slot = borrowed_right.unwrap_or_else(|| self.empty_value_slot());
        let outer = self.error_target();
        let failure = self.next_label("table.update.fail");
        let continuation = self.next_label("table.update.continue");
        self.push_error_context(failure.clone());
        let field = self.emit_bytes_value(name_key(member).as_bytes());
        let field = self.emit_bytes_argument(&field);
        self.checked_status_call_at(
            format!("@xiao_runtime_table_get_value(ptr {receiver}, {field}, ptr {left_slot})"),
            span,
        );
        if right_owned {
            let right = self.emit_expression(value)?;
            self.emit(format!("  store {VALUE_TYPE} {right}, ptr {right_slot}"));
        }
        let left = self.next_temp();
        let right = self.next_temp();
        self.emit(format!("  {left} = load {VALUE_TYPE}, ptr {left_slot}"));
        self.emit(format!("  {right} = load {VALUE_TYPE}, ptr {right_slot}"));
        let result = self.emit_binary_runtime(operation, &left, &right, span);
        self.emit(format!("  store {VALUE_TYPE} {result}, ptr {result_slot}"));
        self.checked_status_call_at(
            format!("@xiao_runtime_table_set_value(ptr {receiver}, {field}, ptr {result_slot})"),
            span,
        );
        self.pop_error_context();
        let mut owned = vec![left_slot, result_slot];
        if right_owned {
            owned.push(right_slot);
        }
        if receiver_owned {
            owned.push(receiver);
        }
        for slot in &owned {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_any(ptr {slot})"
            ));
        }
        self.emit(format!("  br label %{continuation}"));
        self.emit_label(&failure);
        for slot in &owned {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_any(ptr {slot})"
            ));
        }
        self.emit(format!("  br label %{outer}"));
        self.emit_label(&continuation);
        Ok(())
    }

    /// 在生成任何方法体前预登记完整方法表，支持同名隔离与方法互调。
    pub(super) fn collect_table_methods(&mut self) {
        for statement in &self.program.body {
            if let IrStatementKind::Table { name, body, .. } = &statement.kind {
                for method in body {
                    if let IrStatementKind::Function { name: member, .. } = &method.kind {
                        self.method_definitions
                            .insert((name.text.clone(), name_key(member)), method);
                    }
                }
            }
        }
    }

    /// 方法符号使用表索引和成员索引，不依赖同名函数或源码路径。
    pub(super) fn table_callback_symbol(&self, table: &str, member: Option<&str>) -> String {
        let table_index = self
            .program
            .table_signatures
            .iter()
            .position(|item| item.name == table)
            .expect("已验证表身份");
        if let Some(member) = member {
            let method_index = self
                .method_definitions
                .keys()
                .filter(|(name, _)| name == table)
                .position(|(_, name)| name == member)
                .expect("已预登记方法");
            format!("xiao.table.{table_index}.method.{method_index}")
        } else {
            format!("xiao.table.{table_index}.fields")
        }
    }

    /// 为方法和字段初始化生成相同的指针式 i32 回调约定。
    pub(super) fn emit_table_callbacks(&mut self) -> Result<()> {
        let methods = self
            .method_definitions
            .iter()
            .map(|(key, statement)| (key.clone(), *statement))
            .collect::<Vec<_>>();
        for ((table, member), statement) in methods {
            let callback = CallbackContext {
                symbol: self.table_callback_symbol(&table, Some(&member)),
                fields: false,
            };
            self.emit_function_with_callback(statement, Some(callback))?;
        }
        let tables = self
            .program
            .body
            .iter()
            .filter_map(|statement| {
                if let IrStatementKind::Table { name, .. } = &statement.kind {
                    Some((name.text.clone(), statement))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for (table, statement) in tables {
            let function = field_function(statement);
            let callback = CallbackContext {
                symbol: self.table_callback_symbol(&table, None),
                fields: true,
            };
            self.emit_function_with_callback(&function, Some(callback))?;
        }
        Ok(())
    }

    /// 调用一次求值的接收者，按源码顺序求值显式实参后绑定参数顺序与缺省值。
    pub(super) fn emit_table_method_call(
        &mut self,
        object: &IrExpression,
        member: &IrName,
        arguments: &[IrCallArgument],
        span: IrSpan,
    ) -> Result<String> {
        let IrType::Table { name: table, .. } = &object.ty else {
            return Err(CodegenError::Unsupported {
                feature: "动态表方法接收者".to_owned(),
                span: Some(span),
            });
        };
        let key = name_key(member);
        let statement = self
            .method_definitions
            .get(&(table.clone(), key.clone()))
            .copied()
            .ok_or_else(|| CodegenError::InvalidIr {
                message: format!("表 {table} 的方法 {key} 未登记"),
            })?;
        let IrStatementKind::Function {
            parameters,
            return_type,
            ..
        } = &statement.kind
        else {
            unreachable!()
        };
        let parameters = &parameters[1..];
        let types = parameters
            .iter()
            .map(|parameter| method_type(&parameter.ty))
            .collect::<Result<Vec<_>>>()?;
        let signature = table_method_signature_id(&types, method_type(return_type)?);
        let receiver = self.emit_expression(object)?;
        let receiver_slot = self.empty_value_slot();
        self.store_call_argument(object, object, &receiver, &receiver_slot);
        let argument_array = self.next_temp();
        self.emit(format!(
            "  {argument_array} = alloca {VALUE_TYPE}, i64 {}",
            parameters.len()
        ));
        let mut assigned = vec![false; parameters.len()];
        let mut evaluated = Vec::new();
        // 失败边必须归还已经求值的接收者和实参；槽先清零，清理不触碰未求值内容。
        let mut argument_slots = Vec::new();
        for index in 0..parameters.len() {
            let slot = self.next_temp();
            self.emit(format!(
                "  {slot} = getelementptr {VALUE_TYPE}, ptr {argument_array}, i64 {index}"
            ));
            self.emit(format!("  store {VALUE_TYPE} zeroinitializer, ptr {slot}"));
            argument_slots.push(slot);
        }
        let outer = self.error_target();
        let failure = self.next_label("table.call.fail");
        let continuation = self.next_label("table.call.continue");
        self.push_error_context(failure.clone());
        for argument in arguments {
            let index = if let Some(name) = &argument.name {
                parameters
                    .iter()
                    .position(|parameter| name_key(&parameter.name) == name_key(name))
            } else if argument.kind == "positional" {
                assigned.iter().position(|assigned| !assigned)
            } else {
                None
            }
            .ok_or_else(|| CodegenError::Unsupported {
                feature: "表方法参数形态".to_owned(),
                span: Some(span),
            })?;
            if assigned[index] {
                return Err(CodegenError::InvalidIr {
                    message: "重复的方法参数".to_owned(),
                });
            }
            let value = self.emit_expression(&argument.value)?;
            self.store_call_argument(&argument.value, object, &value, &argument_slots[index]);
            assigned[index] = true;
            evaluated.push(index);
        }
        for (index, parameter) in parameters.iter().enumerate() {
            if !assigned[index] {
                let default =
                    parameter
                        .default
                        .as_ref()
                        .ok_or_else(|| CodegenError::InvalidIr {
                            message: format!("缺少方法参数 {}", parameter.name.text),
                        })?;
                let value = self.emit_expression(default)?;
                self.store_call_argument(default, object, &value, &argument_slots[index]);
                evaluated.push(index);
            }
        }
        let output = self.next_temp();
        self.emit(format!("  {output} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {output}"
        ));
        let name = self.emit_bytes_value(key.as_bytes());
        let name = self.emit_bytes_argument(&name);
        self.checked_status_call_at(format!("@xiao_runtime_table_call(ptr {receiver_slot}, {name}, i64 {signature}, ptr {argument_array}, i64 {}, ptr {output})", parameters.len()), span);
        self.pop_error_context();
        self.emit(format!(
            "  call void @xiao_runtime_value_release_any(ptr {receiver_slot})"
        ));
        for index in evaluated {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_any(ptr {})",
                argument_slots[index]
            ));
        }
        self.emit(format!("  br label %{continuation}"));
        self.emit_label(&failure);
        self.emit(format!(
            "  call void @xiao_runtime_value_release_any(ptr {receiver_slot})"
        ));
        for slot in &argument_slots {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_any(ptr {slot})"
            ));
        }
        self.emit(format!("  br label %{outer}"));
        self.emit_label(&continuation);
        let result = self.next_temp();
        self.emit(format!("  {result} = load {VALUE_TYPE}, ptr {output}"));
        Ok(result)
    }
}

/// 仅把已检查的字段语句扩展成局部赋值和 receiver 写入，不推断类型或改写 VM。
fn field_function(statement: &IrStatement) -> IrStatement {
    let IrStatementKind::Table {
        name,
        table_kind,
        body,
    } = &statement.kind
    else {
        unreachable!()
    };
    let receiver = IrName {
        text: "#table_receiver".to_owned(),
        backticked: false,
        span: statement.span,
    };
    let receiver_type = IrType::Table {
        name: name.text.clone(),
        kind: if table_kind == "singleton" {
            "singleton"
        } else {
            "instance"
        }
        .to_owned(),
    };
    let mut fields = Vec::new();
    for field in body {
        let target = match &field.kind {
            IrStatementKind::Assignment { target, .. }
            | IrStatementKind::ConstDeclaration { target, .. }
            | IrStatementKind::Declaration {
                target,
                value: Some(_),
                ..
            } => target,
            _ => continue,
        };
        fields.push(field.clone());
        let object = IrExpression {
            kind: IrExpressionKind::Name {
                name: receiver.clone(),
            },
            ty: receiver_type.clone(),
            span: statement.span,
        };
        fields.push(IrStatement {
            kind: IrStatementKind::ExtendedAssignment {
                target: IrExpression {
                    kind: IrExpressionKind::Member {
                        object: Box::new(object),
                        member: target.clone(),
                    },
                    ty: IrType::Dynamic,
                    span: target.span,
                },
                operator: "=".to_owned(),
                value: IrExpression {
                    kind: IrExpressionKind::Name {
                        name: target.clone(),
                    },
                    ty: IrType::Dynamic,
                    span: target.span,
                },
            },
            span: field.span,
            leading_docs: Vec::new(),
        });
    }
    IrStatement {
        kind: IrStatementKind::Function {
            name: receiver.clone(),
            parameters: vec![IrParameter {
                name: receiver,
                kind: "positional_or_keyword".to_owned(),
                ty: receiver_type,
                default: None,
                span: statement.span,
            }],
            return_type: IrType::None,
            body: fields,
        },
        span: statement.span,
        leading_docs: Vec::new(),
    }
}

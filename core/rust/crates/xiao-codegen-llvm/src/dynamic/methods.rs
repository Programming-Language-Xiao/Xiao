//! 表方法回调、字段初始化辅助函数与固定签名调用；类型事实只消费 IR。

use xiao_ir::{
    IrCallArgument, IrExpression, IrExpressionKind, IrName, IrParameter, IrSpan, IrStatement,
    IrStatementKind, IrType,
};
use xiao_runtime_abi::{TABLE_METHOD_DYNAMIC_TYPE, XiaoValueTag, table_method_signature_id};

use super::predicate::name_key;
use super::{DynamicGenerator, VALUE_TYPE};
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
    /// 弱析构接收者和普通接收者共用值指针字段入口。
    pub(super) fn emit_callback_field_get(
        &mut self,
        receiver: String,
        member: &IrName,
        span: IrSpan,
    ) -> Result<String> {
        self.emit_callback_field(receiver, member, None, span)
    }

    /// 字段赋值在求值失败和 ABI 失败两条边都归还临时拥有值。
    pub(super) fn emit_callback_field_set(
        &mut self,
        receiver: String,
        member: &IrName,
        value: &IrExpression,
        span: IrSpan,
    ) -> Result<()> {
        self.emit_callback_field(receiver, member, Some(value), span)
            .map(|_| ())
    }

    /// 发射一次字段 ABI 调用及其共享清理边，不从弱视图中提取强句柄。
    fn emit_callback_field(
        &mut self,
        receiver: String,
        member: &IrName,
        input: Option<&IrExpression>,
        span: IrSpan,
    ) -> Result<String> {
        let receiver_slot = self.next_temp();
        let value_slot = self.next_temp();
        self.emit(format!("  {receiver_slot} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} {receiver}, ptr {receiver_slot}"
        ));
        self.emit(format!("  {value_slot} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {value_slot}"
        ));
        let outer = self.error_target();
        let failure = self.next_label("table.field.fail");
        let continuation = self.next_label("table.field.continue");
        self.push_error_context(failure.clone());
        if let Some(input) = input {
            let value = self.emit_expression(input)?;
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {value_slot}"));
        }
        let field = self.emit_bytes_value(name_key(member).as_bytes());
        let field = self.emit_bytes_argument(&field);
        let function = if input.is_some() {
            "xiao_runtime_table_set_value"
        } else {
            "xiao_runtime_table_get_value"
        };
        self.checked_status_call_at(
            format!("@{function}(ptr {receiver_slot}, {field}, ptr {value_slot})"),
            span,
        );
        self.pop_error_context();
        if input.is_some() {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_any(ptr {value_slot})"
            ));
        }
        self.emit(format!(
            "  call void @xiao_runtime_value_release_any(ptr {receiver_slot})"
        ));
        self.emit(format!("  br label %{continuation}"));
        self.emit_label(&failure);
        self.emit(format!(
            "  call void @xiao_runtime_value_release_any(ptr {value_slot})"
        ));
        self.emit(format!(
            "  call void @xiao_runtime_value_release_any(ptr {receiver_slot})"
        ));
        self.emit(format!("  br label %{outer}"));
        self.emit_label(&continuation);
        let result = self.next_temp();
        self.emit(format!("  {result} = load {VALUE_TYPE}, ptr {value_slot}"));
        Ok(result)
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
        let receiver_slot = self.next_temp();
        self.emit(format!("  {receiver_slot} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} {receiver}, ptr {receiver_slot}"
        ));
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
            self.emit(format!(
                "  store {VALUE_TYPE} {value}, ptr {}",
                argument_slots[index]
            ));
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
                self.emit(format!(
                    "  store {VALUE_TYPE} {value}, ptr {}",
                    argument_slots[index]
                ));
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
        for index in evaluated {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_any(ptr {})",
                argument_slots[index]
            ));
        }
        self.emit(format!(
            "  call void @xiao_runtime_value_release_any(ptr {receiver_slot})"
        ));
        self.emit(format!("  br label %{continuation}"));
        self.emit_label(&failure);
        for slot in &argument_slots {
            self.emit(format!(
                "  call void @xiao_runtime_value_release_any(ptr {slot})"
            ));
        }
        self.emit(format!(
            "  call void @xiao_runtime_value_release_any(ptr {receiver_slot})"
        ));
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

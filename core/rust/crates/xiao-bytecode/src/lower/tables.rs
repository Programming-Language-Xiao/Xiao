//! 表声明与普通函数表的适配；不重新推断类型或释放计划。

use super::{Lowerer, name_key};
use crate::tac::{
    FuncId, RegisterClass, SigId, TacArgument, TacInstr, TacOp, TacTableDefinition, VReg,
};
use std::collections::BTreeMap;
use xiao_ir::{
    IrCallArgument, IrExpression, IrExpressionKind, IrName, IrParameter, IrSpan, IrStatement,
    IrStatementKind, IrType,
};

/// 合成字段函数使用的接收者名称；源码不能声明此名称。
const RECEIVER: &str = "#table_receiver";

impl Lowerer<'_> {
    /// 保持顶层编号后，按源码顺序预登记全部表辅助函数和方法。
    pub(super) fn register_tables(&mut self, mut next: u32) -> Vec<(String, IrStatement, SigId)> {
        let mut functions = Vec::new();
        for statement in &self.program.body {
            let IrStatementKind::Table {
                name,
                table_kind,
                body,
            } = &statement.kind
            else {
                continue;
            };
            let Some(signature) = self
                .program
                .table_signatures
                .iter()
                .find(|item| item.name == name.text)
                .cloned()
            else {
                self.record_unsupported(format!("表 {} 缺少前端签名", name.text));
                continue;
            };
            let fields = field_function(name, table_kind, body, statement.span);
            let IrStatementKind::Function {
                parameters,
                return_type,
                ..
            } = &fields.kind
            else {
                unreachable!()
            };
            let call_sig = self.signature_for(parameters, return_type);
            let fields_id = FuncId::new(next);
            self.indexed_signatures.insert(fields_id, call_sig);
            next += 1;
            functions.push((format!("{}::<fields>", name.text), fields, call_sig));
            let mut methods = BTreeMap::new();
            for method in body {
                let IrStatementKind::Function {
                    name: member,
                    parameters,
                    return_type,
                    ..
                } = &method.kind
                else {
                    continue;
                };
                let id = FuncId::new(next);
                next += 1;
                let call_sig = self.signature_for(parameters, return_type);
                self.indexed_signatures.insert(id, call_sig);
                methods.insert(name_key(&member.text, member.backticked), id);
                functions.push((
                    format!(
                        "{}::{}",
                        name.text,
                        name_key(&member.text, member.backticked)
                    ),
                    method.clone(),
                    call_sig,
                ));
            }
            self.table_definitions.push(TacTableDefinition {
                signature,
                fields: fields_id,
                methods,
            });
        }
        functions
    }

    /// 按前端已解析的表身份定位定义。
    pub(super) fn table_index(&self, ty: &IrType) -> Option<u32> {
        let IrType::Table { name, .. } = ty else {
            return None;
        };
        self.table_definitions
            .iter()
            .position(|table| table.signature.name == *name)
            .map(|index| index as u32)
    }

    /// 降低声明：构造器只占定义表，单例在语句位置创建并绑定。
    pub(super) fn lower_table_declaration(&mut self, name: &IrName, kind: &str, span: IrSpan) {
        if kind != "singleton" {
            return;
        }
        let ty = IrType::Table {
            name: name.text.clone(),
            kind: kind.to_owned(),
        };
        let Some(table) = self.table_index(&ty) else {
            return;
        };
        let register = self.new_register(RegisterClass::ObjHandle, span);
        let arguments = self.table_arguments(table, "ascii:init", &[]);
        self.emit(TacInstr::with_dst(
            TacOp::LoadTable {
                table,
                construct: true,
                arguments,
            },
            register,
            span,
        ));
        super::stmt::store_into(self, &name.text, name.backticked, name.span, register);
    }

    /// 构造实参先求值，字段默认值与 init 由 Runtime 构造边界执行。
    pub(super) fn lower_new(
        &mut self,
        callee: &IrExpression,
        arguments: &[IrCallArgument],
        span: IrSpan,
    ) -> VReg {
        let Some(table) = self.table_index(&callee.ty) else {
            self.record_unsupported("new 目标缺少静态表身份".to_owned());
            return self.new_register(RegisterClass::Poly, span);
        };
        let arguments = self.table_arguments(table, "ascii:init", arguments);
        let register = self.new_register(RegisterClass::ObjHandle, span);
        self.emit(TacInstr::with_dst(
            TacOp::LoadTable {
                table,
                construct: true,
                arguments,
            },
            register,
            span,
        ));
        register
    }

    /// 表方法直接调用现有 Call，并显式传递一次求值的接收者。
    pub(super) fn lower_method_call(
        &mut self,
        object: &IrExpression,
        member: &IrName,
        arguments: &[IrCallArgument],
        expression: &IrExpression,
    ) -> Option<VReg> {
        let table = self.table_index(&object.ty)?;
        let key = name_key(&member.text, member.backticked);
        let callee = *self.table_definitions[table as usize].methods.get(&key)?;
        let receiver = self.lower_expression(object);
        let mut bound = vec![TacArgument::positional(receiver)];
        bound.extend(self.table_arguments(table, &key, arguments));
        let signature = self.indexed_signatures[&callee];
        let register = self.new_register(Self::class_of_type(&expression.ty), expression.span);
        self.emit(TacInstr::with_dst(
            TacOp::Call {
                callee,
                signature,
                arguments: bound,
            },
            register,
            expression.span,
        ));
        Some(register)
    }

    /// 填入静态方法的缺省实参；显式实参保持源码求值顺序。
    fn table_arguments(
        &mut self,
        table: u32,
        method: &str,
        arguments: &[IrCallArgument],
    ) -> Vec<TacArgument> {
        let mut result = Vec::new();
        for argument in arguments {
            if !matches!(argument.kind.as_str(), "positional" | "keyword") {
                self.record_unsupported("表方法的展开实参尚未降低".to_owned());
            }
            let value = self.lower_expression(&argument.value);
            result.push(match &argument.name {
                Some(name) => TacArgument::keyword(name.text.clone(), value),
                None => TacArgument::positional(value),
            });
        }
        let name = &self.table_definitions[table as usize].signature.name;
        let defaults = self
            .program
            .body
            .iter()
            .find_map(|statement| {
                let IrStatementKind::Table {
                    name: candidate,
                    body,
                    ..
                } = &statement.kind
                else {
                    return None;
                };
                if candidate.text != *name {
                    return None;
                }
                body.iter().find_map(|statement| {
                    let IrStatementKind::Function {
                        name, parameters, ..
                    } = &statement.kind
                    else {
                        return None;
                    };
                    (name_key(&name.text, name.backticked) == method).then(|| parameters.clone())
                })
            })
            .unwrap_or_default();
        let positional = arguments
            .iter()
            .filter(|argument| argument.name.is_none())
            .count();
        for (index, parameter) in defaults.iter().skip(1).enumerate() {
            if index < positional
                || arguments.iter().any(|argument| {
                    argument
                        .name
                        .as_ref()
                        .is_some_and(|name| name.text == parameter.name.text)
                })
            {
                continue;
            }
            if let Some(default) = &parameter.default {
                let value = self.lower_expression(default);
                result.push(TacArgument::keyword(parameter.name.text.clone(), value));
            }
        }
        result
    }

    /// 为跨函数的单例名称发出弱注册表读取，避免伪造局部寄存器。
    pub(super) fn singleton_reference(&mut self, name: &str, span: IrSpan) -> Option<VReg> {
        let table =
            self.table_definitions.iter().position(|table| {
                table.signature.name == name && table.signature.kind == "singleton"
            })? as u32;
        let register = self.new_register(RegisterClass::ObjHandle, span);
        self.emit(TacInstr::with_dst(
            TacOp::LoadTable {
                table,
                construct: false,
                arguments: Vec::new(),
            },
            register,
            span,
        ));
        Some(register)
    }

    /// 只内联前端已接受的顶层常量表达式；不为普通全局变量伪造捕获。
    pub(super) fn global_constant(&mut self, name: &str, backticked: bool) -> Option<VReg> {
        let expression = self
            .program
            .body
            .iter()
            .find_map(|statement| match &statement.kind {
                IrStatementKind::ConstDeclaration { target, value, .. }
                    if target.text == name && target.backticked == backticked =>
                {
                    Some(value.clone())
                }
                _ => None,
            })?;
        Some(self.lower_expression(&expression))
    }
}

/// 生成只含原字段绑定及写入的辅助函数；字段作用域与释放动作继续取自 IR。
fn field_function(name: &IrName, kind: &str, body: &[IrStatement], span: IrSpan) -> IrStatement {
    let receiver = IrName {
        text: RECEIVER.to_owned(),
        backticked: false,
        span,
    };
    let receiver_type = IrType::Table {
        name: name.text.clone(),
        kind: if kind == "singleton" {
            "singleton"
        } else {
            "instance"
        }
        .to_owned(),
    };
    let mut fields = Vec::new();
    for statement in body {
        let target = match &statement.kind {
            IrStatementKind::Assignment { target, .. }
            | IrStatementKind::ConstDeclaration { target, .. }
            | IrStatementKind::Declaration {
                target,
                value: Some(_),
                ..
            } => target,
            _ => continue,
        };
        fields.push(statement.clone());
        let object = IrExpression {
            kind: IrExpressionKind::Name {
                name: receiver.clone(),
            },
            ty: receiver_type.clone(),
            span,
        };
        let target_expr = IrExpression {
            kind: IrExpressionKind::Member {
                object: Box::new(object),
                member: target.clone(),
            },
            ty: IrType::Dynamic,
            span: target.span,
        };
        let value = IrExpression {
            kind: IrExpressionKind::Name {
                name: target.clone(),
            },
            ty: IrType::Dynamic,
            span: target.span,
        };
        fields.push(IrStatement {
            kind: IrStatementKind::ExtendedAssignment {
                target: target_expr,
                operator: "=".to_owned(),
                value,
            },
            span: statement.span,
            leading_docs: Vec::new(),
        });
    }
    IrStatement {
        span,
        leading_docs: Vec::new(),
        kind: IrStatementKind::Function {
            name: receiver.clone(),
            parameters: vec![IrParameter {
                name: receiver,
                kind: "positional_or_keyword".to_owned(),
                ty: receiver_type,
                default: None,
                span,
            }],
            return_type: IrType::None,
            body: fields,
        },
    }
}

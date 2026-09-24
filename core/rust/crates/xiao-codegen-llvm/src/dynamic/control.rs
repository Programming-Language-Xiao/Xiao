//! 动态降低器的语句、条件与控制流发射。

use xiao_ir::{IrExpression, IrExpressionKind, IrStatement, IrStatementKind, IrType};

use super::{DynamicGenerator, LoopLabels, VALUE_TYPE};
use crate::error::{CodegenError, Result};

impl<'a> DynamicGenerator<'a> {
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
    pub(super) fn emit_statements(&mut self, statements: &[IrStatement]) -> Result<()> {
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
                    "  {payload} = extractvalue {VALUE_TYPE} {value}, 1"
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
}

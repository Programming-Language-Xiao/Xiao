//! 动态降低器的语句、条件与控制流发射。

use xiao_diagnostics::{CatchTypeKind, error_kind_of};
use xiao_ir::{
    IrCatchClause, IrExpression, IrExpressionKind, IrSpan, IrStatement, IrStatementKind, IrType,
};

use super::{CleanupContext, DynamicGenerator, LoopLabels, VALUE_TYPE};
use crate::error::{CodegenError, Result};

/// 一个控制转移的最终目的地。
#[derive(Clone, Debug)]
enum ControlExitTarget {
    /// 返回动态入口的观察值或成功退出。
    Return,
    /// 跳转到已生成的循环或合流块。
    Branch(String),
}

/// 一条受保护区域的清理发射参数。
struct TryCleanup<'a> {
    /// 要执行的 `finally` 主体。
    region: CleanupContext<'a>,
    /// 当前退出边名称。
    protected_exit: &'static str,
    /// 清理成功后的目标块。
    success_target: String,
    /// 清理失败后的外层错误派发块。
    failure_target: String,
    /// 生成标签的稳定前缀。
    label_prefix: String,
}

/// 清理失败路径上的作用域和退出类别。
struct CleanupFailure {
    /// 第一个待释放作用域。
    first_scope: Option<u32>,
    /// 第二个待释放作用域。
    second_scope: Option<u32>,
    /// 可恢复错误的释放计划类别。
    recoverable_exit: &'static str,
    /// 清理完成后的错误派发目标。
    failure_target: String,
}

impl<'a> DynamicGenerator<'a> {
    /// 发射一条顶层语句；异常展开留给 N0-C。
    fn emit_statement(&mut self, statement: &'a IrStatement) -> Result<()> {
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
                    let emitted = self.emit_expression(value)?;
                    if let (true, Some(slot), Some(label)) = (
                        self.function_mode,
                        self.function_return_slot.clone(),
                        self.function_return_label.clone(),
                    ) {
                        self.emit(format!("  store {VALUE_TYPE} {emitted}, ptr {slot}"));
                        self.emit(format!("  br label %{label}"));
                        self.terminated = true;
                    } else {
                        let value_type = value.ty.clone();
                        self.record_observation(&emitted, &value_type);
                        self.release_value(emitted);
                        self.emit_nonlocal_exit_from_depth("return", ControlExitTarget::Return, 0)?;
                        self.terminated = true;
                    }
                } else if self.function_mode {
                    if let (Some(slot), Some(label)) = (
                        self.function_return_slot.clone(),
                        self.function_return_label.clone(),
                    ) {
                        let none = self.none_value();
                        self.emit(format!("  store {VALUE_TYPE} {none}, ptr {slot}"));
                        self.emit(format!("  br label %{label}"));
                        self.terminated = true;
                    }
                } else {
                    self.emit_nonlocal_exit_from_depth("return", ControlExitTarget::Return, 0)?;
                    self.terminated = true;
                }
            }
            IrStatementKind::If {
                condition,
                body,
                elif_branches,
                else_body,
            } => self.emit_if(condition, body, elif_branches, else_body.as_deref())?,
            IrStatementKind::While { condition, body } => self.emit_while(condition, body)?,
            IrStatementKind::For {
                target,
                iterable,
                body,
            } => self.emit_for(target, iterable, body)?,
            IrStatementKind::Function { .. } => {}
            IrStatementKind::Import { items, .. } => {
                for item in items {
                    if self.initialized_modules.insert(item.module.clone()) {
                        let Some(module) = self
                            .program
                            .modules
                            .iter()
                            .find(|module| module.name == item.module)
                        else {
                            return Err(CodegenError::Unsupported {
                                feature: format!("原生模块 {} 不在前端模块图中", item.module),
                                span: Some(statement.span),
                            });
                        };
                        self.emit_statements(&module.body)?;
                        if item.selected.is_none() {
                            let namespace = self.emit_module_namespace(module)?;
                            self.store_slot(&item.binding, namespace)?;
                            continue;
                        }
                    }
                    if let Some(selected) = &item.selected {
                        if selected.text != item.binding.text
                            || selected.backticked != item.binding.backticked
                        {
                            let value = self.load_slot(selected)?;
                            self.store_slot(&item.binding, value)?;
                        }
                        continue;
                    }
                }
            }
            IrStatementKind::Try {
                body,
                catches,
                finally_body,
            } => self.emit_try(body, catches, finally_body.as_deref(), statement.span)?,
            IrStatementKind::Raise { value } => self.emit_raise(value, statement.span)?,
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
                self.emit_nonlocal_exit_from_depth(
                    "break",
                    ControlExitTarget::Branch(labels.end),
                    labels.cleanup_depth,
                )?;
                self.terminated = true;
            }
            IrStatementKind::Continue => {
                let Some(labels) = self.loop_stack.last().cloned() else {
                    return Err(CodegenError::InvalidIr {
                        message: "动态 continue 不在循环中".to_owned(),
                    });
                };
                self.emit_nonlocal_exit_from_depth(
                    "continue",
                    ControlExitTarget::Branch(labels.condition),
                    labels.cleanup_depth,
                )?;
                self.terminated = true;
            }
        }
        Ok(())
    }

    /// 在当前基本块依次发射语句，遇到终止边后停止。
    pub(super) fn emit_statements(&mut self, statements: &'a [IrStatement]) -> Result<()> {
        for statement in statements {
            if self.terminated {
                break;
            }
            self.emit_statement(statement)?;
        }
        Ok(())
    }

    /// 发射主动 `raise`，先把错误值交给 Runtime，再跳入当前最内层错误派发块。
    fn emit_raise(&mut self, expression: &IrExpression, span: IrSpan) -> Result<()> {
        let value = self.emit_expression(expression)?;
        let input = self.next_temp();
        self.emit(format!("  {input} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {input}"));
        let location = self.emit_error_location(span);
        let status = self.next_temp();
        self.emit(format!(
            "  {status} = call i32 @xiao_runtime_error_raise_value(ptr {input}, ptr {location})"
        ));
        self.emit(format!(
            "  call void @xiao_runtime_value_release_strong(ptr {input})"
        ));
        self.check_status_at(&status, span);
        self.emit(format!("  br label %{}", self.error_target()));
        self.terminated = true;
        Ok(())
    }

    /// 发射原生 `try`/`catch`/`finally` 展开。
    ///
    /// 每条进入处理器的路径都保持 `finally -> drop -> catch/继续传播`；Fatal
    /// 在派发入口直接进入统一终点，不执行普通处理器或释放计划。
    fn emit_try(
        &mut self,
        body: &'a [IrStatement],
        catches: &'a [IrCatchClause],
        finally_body: Option<&'a [IrStatement]>,
        span: IrSpan,
    ) -> Result<()> {
        for catch in catches {
            if !matches!(
                error_kind_of(&catch.error_type.text),
                Some(CatchTypeKind::AnyRecoverable | CatchTypeKind::Recoverable(_))
            ) {
                return Err(CodegenError::Unsupported {
                    feature: format!("原生 catch 类型 {}", catch.error_type.text),
                    span: Some(catch.error_type.span),
                });
            }
        }

        let outer_target = self.error_target();
        let dispatch = self.next_label("dynamic.try.dispatch");
        let normal_exit = self.next_label("dynamic.try.normal");
        let continuation = self.next_label("dynamic.try.continue");
        let try_scope = self.region_scope(span, "try");
        let finally_scope = self.region_scope(span, "finally");

        let cleanup_snapshot = self.cleanup_stack.clone();
        self.cleanup_stack.push(CleanupContext {
            finally_body,
            finally_scope,
            protected_scope: try_scope,
            failure_target: outer_target.clone(),
        });
        self.push_error_context(dispatch.clone());
        self.emit_statements(body)?;
        self.pop_error_context();
        self.cleanup_stack = cleanup_snapshot;
        if !self.terminated {
            self.emit(format!("  br label %{normal_exit}"));
            self.terminated = true;
        }

        self.terminated = true;
        self.emit_label(&normal_exit);
        self.emit_try_cleanup(TryCleanup {
            region: CleanupContext {
                finally_body,
                finally_scope,
                protected_scope: try_scope,
                failure_target: outer_target.clone(),
            },
            protected_exit: "normal",
            success_target: continuation.clone(),
            failure_target: outer_target.clone(),
            label_prefix: "dynamic.try.normal.finally".to_owned(),
        })?;

        if !self.terminated {
            self.emit(format!("  br label %{continuation}"));
            self.terminated = true;
        }
        self.emit_label(&dispatch);
        let class = self.next_temp();
        self.emit(format!("  {class} = call i32 @xiao_runtime_error_class()"));
        let fatal = self.next_temp();
        let recoverable = self.next_label("dynamic.try.recoverable");
        self.emit(format!("  {fatal} = icmp eq i32 {class}, 2"));
        self.emit(format!(
            "  br i1 {fatal}, label %{}, label %{recoverable}",
            self.error_terminal_label
        ));
        self.emit_label(&recoverable);

        let unmatched = self.next_label("dynamic.try.unmatched");
        let catch_labels = catches
            .iter()
            .enumerate()
            .map(|(index, _)| self.next_label(&format!("dynamic.try.catch{index}")))
            .collect::<Vec<_>>();
        let mut next_match = recoverable;
        for (index, catch) in catches.iter().enumerate() {
            if index > 0 {
                self.emit_label(&next_match);
            }
            let error_type = self.emit_bytes_value(catch.error_type.text.as_bytes());
            let error_type = self.emit_bytes_argument(&error_type);
            let matched = self.next_temp();
            self.emit(format!(
                "  {matched} = call i32 @xiao_runtime_error_matches({error_type})"
            ));
            let is_match = self.next_temp();
            let next = if index + 1 == catches.len() {
                unmatched.clone()
            } else {
                self.next_label("dynamic.try.next-catch")
            };
            self.emit(format!("  {is_match} = icmp eq i32 {matched}, 1"));
            self.emit(format!(
                "  br i1 {is_match}, label %{}, label %{next}",
                catch_labels[index]
            ));
            next_match = next;
        }
        if catches.is_empty() {
            self.emit(format!("  br label %{unmatched}"));
            self.terminated = true;
        }

        self.emit_label(&unmatched);
        self.emit_try_cleanup(TryCleanup {
            region: CleanupContext {
                finally_body,
                finally_scope,
                protected_scope: try_scope,
                failure_target: outer_target.clone(),
            },
            protected_exit: "unmatched_error",
            success_target: outer_target.clone(),
            failure_target: outer_target.clone(),
            label_prefix: "dynamic.try.unmatched.finally".to_owned(),
        })?;

        if !self.terminated {
            self.emit(format!("  br label %{outer_target}"));
            self.terminated = true;
        }

        for (index, catch) in catches.iter().enumerate() {
            self.emit_label(&catch_labels[index]);
            let catch_body_label = self.next_label(&format!("dynamic.try.catch{index}.body"));
            self.emit_try_cleanup(TryCleanup {
                region: CleanupContext {
                    finally_body,
                    finally_scope,
                    protected_scope: try_scope,
                    failure_target: outer_target.clone(),
                },
                protected_exit: "catch",
                success_target: catch_body_label.clone(),
                failure_target: outer_target.clone(),
                label_prefix: format!("dynamic.try.catch{index}.finally"),
            })?;
            self.emit_label(&catch_body_label);

            let error_slot = self.next_temp();
            self.emit(format!("  {error_slot} = alloca {VALUE_TYPE}"));
            self.emit(format!(
                "  store {VALUE_TYPE} zeroinitializer, ptr {error_slot}"
            ));
            let take_status = self.next_temp();
            self.emit(format!(
                "  {take_status} = call i32 @xiao_runtime_error_take(ptr {error_slot})"
            ));
            self.check_status_at(&take_status, catch.span);
            let error_value = self.next_temp();
            self.emit(format!(
                "  {error_value} = load {VALUE_TYPE}, ptr {error_slot}"
            ));
            self.store_slot(&catch.binding, error_value)?;

            let catch_failure = self.next_label(&format!("dynamic.try.catch{index}.fail"));
            let catch_scope = self.region_scope(catch.span, "catch");
            let cleanup_snapshot = self.cleanup_stack.clone();
            self.cleanup_stack.push(CleanupContext {
                finally_body: None,
                finally_scope: None,
                protected_scope: catch_scope,
                failure_target: outer_target.clone(),
            });
            self.push_error_context(catch_failure.clone());
            self.emit_statements(&catch.body)?;
            self.pop_error_context();
            self.cleanup_stack = cleanup_snapshot;
            if !self.terminated {
                self.release_for_scope(catch_scope, "normal")?;
                self.emit(format!("  br label %{continuation}"));
                self.terminated = true;
            }
            self.emit_label(&catch_failure);
            self.emit_error_cleanup_failure(CleanupFailure {
                first_scope: catch_scope,
                second_scope: None,
                recoverable_exit: "unmatched_error",
                failure_target: outer_target.clone(),
            })?;
            self.terminated = true;
        }

        self.emit_label(&continuation);
        Ok(())
    }

    /// 发射一条 `finally` 后接作用域释放的路径。
    fn emit_try_cleanup(&mut self, cleanup: TryCleanup<'a>) -> Result<()> {
        let TryCleanup {
            region,
            protected_exit,
            success_target,
            failure_target,
            label_prefix,
        } = cleanup;
        if let Some(finally_body) = region.finally_body {
            let failure = self.next_label(&format!("{label_prefix}.fail"));
            let success = self.next_label(&format!("{label_prefix}.success"));
            let stack_save = self.begin_finally_stack_frame();
            let cleanup_snapshot = self.cleanup_stack.clone();
            self.cleanup_stack.push(CleanupContext {
                finally_body: None,
                finally_scope: region.finally_scope,
                protected_scope: region.protected_scope,
                failure_target: region.failure_target.clone(),
            });
            self.push_error_context(failure.clone());
            self.emit_statements(finally_body)?;
            self.pop_error_context();
            self.cleanup_stack = cleanup_snapshot;
            let continues = !self.terminated;
            if continues {
                self.restore_finally_stack_frame();
                self.release_for_scope_with_target(
                    region.finally_scope,
                    "normal",
                    &region.failure_target,
                )?;
                self.release_for_scope_with_target(
                    region.protected_scope,
                    protected_exit,
                    &region.failure_target,
                )?;
                self.emit(format!("  br label %{success}"));
                self.terminated = true;
            }
            self.emit_label(&failure);
            self.restore_finally_stack_frame();
            self.emit_error_cleanup_failure(CleanupFailure {
                first_scope: region.finally_scope,
                second_scope: region.protected_scope,
                recoverable_exit: "unmatched_error",
                failure_target,
            })?;
            if continues {
                self.emit_label(&success);
                self.emit(format!("  br label %{success_target}"));
                self.terminated = true;
            }
            self.finish_finally_stack_frame(&stack_save);
        } else {
            self.release_for_scope_with_target(
                region.finally_scope,
                protected_exit,
                &region.failure_target,
            )?;
            self.release_for_scope_with_target(
                region.protected_scope,
                protected_exit,
                &region.failure_target,
            )?;
            if !self.terminated {
                self.emit(format!("  br label %{success_target}"));
                self.terminated = true;
            }
        }
        Ok(())
    }

    /// 发射一条非局部退出边，并依次完成当前受保护作用域的清理。
    fn emit_nonlocal_exit_from_depth(
        &mut self,
        exit: &str,
        target: ControlExitTarget,
        cleanup_depth: usize,
    ) -> Result<()> {
        if cleanup_depth > self.cleanup_stack.len() {
            return Err(CodegenError::InvalidIr {
                message: format!("动态 {exit} 退出的清理深度无效"),
            });
        }
        while self.cleanup_stack.len() > cleanup_depth {
            let region = self
                .cleanup_stack
                .pop()
                .ok_or_else(|| CodegenError::InvalidIr {
                    message: format!("动态 {exit} 退出缺少作用域清理区域"),
                })?;
            self.emit_cleanup_context(region, exit, &target)?;
            if self.terminated {
                return Ok(());
            }
        }

        match target {
            ControlExitTarget::Return => {
                if self.function_mode {
                    if let Some(label) = self.function_return_label.clone() {
                        self.emit(format!("  br label %{label}"));
                        return Ok(());
                    }
                }
                self.with_error_target(self.error_terminal_label.clone(), |generator| {
                    generator.release_for_exit("return")
                })?;
                self.emit_observation_return();
            }
            ControlExitTarget::Branch(label) => {
                self.emit(format!("  br label %{label}"));
            }
        }
        Ok(())
    }

    /// 发射一个已弹出区域的 `finally -> drop` 清理链。
    fn emit_cleanup_context(
        &mut self,
        region: CleanupContext<'a>,
        protected_exit: &str,
        _target: &ControlExitTarget,
    ) -> Result<()> {
        if let Some(finally_body) = region.finally_body {
            let failure = self.next_label("dynamic.nonlocal.finally.fail");
            let success = self.next_label("dynamic.nonlocal.finally.success");
            let stack_save = self.begin_finally_stack_frame();
            let cleanup_snapshot = self.cleanup_stack.clone();
            self.cleanup_stack.push(CleanupContext {
                finally_body: None,
                finally_scope: region.finally_scope,
                protected_scope: region.protected_scope,
                failure_target: region.failure_target.clone(),
            });
            self.push_error_context(failure.clone());
            self.emit_statements(finally_body)?;
            self.pop_error_context();
            self.cleanup_stack = cleanup_snapshot;
            let continues = !self.terminated;
            if continues {
                self.restore_finally_stack_frame();
                self.release_for_scope_with_target(
                    region.finally_scope,
                    "normal",
                    &region.failure_target,
                )?;
                self.release_for_scope_with_target(
                    region.protected_scope,
                    protected_exit,
                    &region.failure_target,
                )?;
                self.emit(format!("  br label %{success}"));
                self.terminated = true;
            }
            self.emit_label(&failure);
            self.restore_finally_stack_frame();
            self.emit_error_cleanup_failure(CleanupFailure {
                first_scope: region.finally_scope,
                second_scope: region.protected_scope,
                recoverable_exit: "unmatched_error",
                failure_target: region.failure_target,
            })?;
            if continues {
                self.emit_label(&success);
            }
            self.finish_finally_stack_frame(&stack_save);
        } else {
            if region.finally_scope.is_some() {
                self.restore_finally_stack_frame();
            }
            self.release_for_scope_with_target(
                region.finally_scope,
                protected_exit,
                &region.failure_target,
            )?;
            self.release_for_scope_with_target(
                region.protected_scope,
                protected_exit,
                &region.failure_target,
            )?;
        }
        Ok(())
    }

    /// 在指定错误目标下发射释放计划，避免重新进入已经离开的 `try`。
    fn release_for_scope_with_target(
        &mut self,
        scope: Option<u32>,
        exit: &str,
        error_target: &str,
    ) -> Result<()> {
        self.with_error_target(error_target.to_owned(), |generator| {
            generator.release_for_scope(scope, exit)
        })
    }

    /// 临时切换当前 ABI 失败边的目标块。
    fn with_error_target<T>(
        &mut self,
        target: String,
        operation: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        self.push_error_context(target);
        let result = operation(self);
        self.pop_error_context();
        result
    }

    /// 清理错误路径上的两个嵌套作用域；Fatal 仍直接终止。
    fn emit_error_cleanup_failure(&mut self, cleanup: CleanupFailure) -> Result<()> {
        let class = self.next_temp();
        self.emit(format!("  {class} = call i32 @xiao_runtime_error_class()"));
        let fatal = self.next_temp();
        let recoverable = self.next_label("dynamic.error-cleanup.recoverable");
        self.emit(format!("  {fatal} = icmp eq i32 {class}, 2"));
        self.emit(format!(
            "  br i1 {fatal}, label %{}, label %{recoverable}",
            self.error_terminal_label
        ));
        self.emit_label(&recoverable);
        self.release_for_scope_with_target(
            cleanup.first_scope,
            cleanup.recoverable_exit,
            &cleanup.failure_target,
        )?;
        self.release_for_scope_with_target(
            cleanup.second_scope,
            cleanup.recoverable_exit,
            &cleanup.failure_target,
        )?;
        self.emit(format!("  br label %{}", cleanup.failure_target));
        self.terminated = true;
        Ok(())
    }

    /// 查找由前端生命周期分析登记的区域作用域。
    fn region_scope(&self, span: IrSpan, kind: &str) -> Option<u32> {
        self.program
            .ownership
            .scopes
            .iter()
            .find(|scope| scope.kind == kind && scope.span == span)
            .map(|scope| scope.id)
    }

    /// 发射布尔 `if`/`elif`/`else` 链并在可达分支汇合。
    fn emit_if(
        &mut self,
        condition: &IrExpression,
        body: &'a [IrStatement],
        elif_branches: &'a [xiao_ir::IrElifBranch],
        else_body: Option<&'a [IrStatement]>,
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
        let cleanup_snapshot = self.cleanup_stack.clone();
        self.emit_statements(body)?;
        self.cleanup_stack = cleanup_snapshot;
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
        branches: &'a [xiao_ir::IrElifBranch],
        else_body: Option<&'a [IrStatement]>,
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
        let cleanup_snapshot = self.cleanup_stack.clone();
        self.emit_statements(&branch.body)?;
        self.cleanup_stack = cleanup_snapshot;
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
    fn emit_while(&mut self, condition: &IrExpression, body: &'a [IrStatement]) -> Result<()> {
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
        let cleanup_snapshot = self.cleanup_stack.clone();
        self.loop_stack.push(LoopLabels {
            condition: condition_label.clone(),
            end: end_label.clone(),
            cleanup_depth: self.cleanup_stack.len(),
        });
        let body_result = self.emit_statements(body);
        self.cleanup_stack = cleanup_snapshot;
        self.loop_stack.pop();
        body_result?;
        if !self.terminated {
            self.emit(format!("  br label %{condition_label}"));
            self.terminated = true;
        }
        self.emit_label(&end_label);
        Ok(())
    }

    /// 发射 `for target in iterable`；来源只求值一次，索引和长度走 Runtime ABI。
    fn emit_for(
        &mut self,
        target: &xiao_ir::IrName,
        iterable: &IrExpression,
        body: &'a [IrStatement],
    ) -> Result<()> {
        let outer_target = self.error_target();
        let source = self.emit_expression(iterable)?;
        let failure_label = self.next_label("dynamic.for.fail");
        let continuation_label = self.next_label("dynamic.for.continue");
        self.push_error_context(failure_label.clone());
        self.emit_dynamic_check("iterable", &source, iterable.span);
        let length = self.emit_iter_len(&source, iterable.span);
        let index_slot = self.next_temp();
        self.emit(format!("  {index_slot} = alloca i64"));
        self.emit(format!("  store i64 0, ptr {index_slot}"));
        let condition_label = self.next_label("dynamic.for.cond");
        let body_label = self.next_label("dynamic.for.body");
        let advance_label = self.next_label("dynamic.for.advance");
        let end_label = self.next_label("dynamic.for.end");
        self.emit(format!("  br label %{condition_label}"));
        self.terminated = true;
        self.emit_label(&condition_label);
        let index = self.next_temp();
        self.emit(format!("  {index} = load i64, ptr {index_slot}"));
        let condition = self.next_temp();
        self.emit(format!("  {condition} = icmp ult i64 {index}, {length}"));
        self.emit(format!(
            "  br i1 {condition}, label %{body_label}, label %{end_label}"
        ));
        self.terminated = true;
        self.emit_label(&body_label);
        let element = self.emit_iter_get(&source, &index, target.span);
        self.store_slot(target, element)?;
        self.loop_stack.push(LoopLabels {
            condition: advance_label.clone(),
            end: end_label.clone(),
            cleanup_depth: self.cleanup_stack.len(),
        });
        let cleanup_snapshot = self.cleanup_stack.clone();
        let body_result = self.emit_statements(body);
        self.cleanup_stack = cleanup_snapshot;
        self.loop_stack.pop();
        body_result?;
        if !self.terminated {
            self.emit(format!("  br label %{advance_label}"));
            self.terminated = true;
        }
        self.emit_label(&advance_label);
        let next = self.next_temp();
        self.emit(format!("  {next} = add i64 {index}, 1"));
        self.emit(format!("  store i64 {next}, ptr {index_slot}"));
        self.emit(format!("  br label %{condition_label}"));
        self.terminated = true;
        self.emit_label(&end_label);
        self.release_value(source.clone());
        self.emit(format!("  br label %{continuation_label}"));
        self.terminated = true;
        self.pop_error_context();
        self.emit_label(&failure_label);
        self.release_value(source);
        self.emit(format!("  br label %{outer_target}"));
        self.terminated = true;
        self.emit_label(&continuation_label);
        Ok(())
    }

    /// 从 ABI 动态值读取已类型检查的布尔载荷。
    fn emit_condition(&mut self, expression: &IrExpression) -> Result<String> {
        if !matches!(&expression.ty, IrType::Scalar { name } if name == "bool") {
            let value = self.emit_expression(expression)?;
            self.emit_dynamic_check("boolean_condition", &value, expression.span);
            let payload = self.next_temp();
            self.emit(format!(
                "  {payload} = extractvalue {VALUE_TYPE} {value}, 1"
            ));
            let output = self.next_temp();
            self.emit(format!("  {output} = trunc i64 {payload} to i1"));
            self.release_value(value);
            return Ok(output);
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
                let failure_target = self.error_target();
                self.emit(format!(
                    "  br i1 {tag_ok}, label %{valid_label}, label %{failure_target}"
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

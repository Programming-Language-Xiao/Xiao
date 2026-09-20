//! 表的 Runtime 接线；构造和析构都复用相同载体与 TAC 语义核。

use super::*;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use xiao_runtime::{TableDefinition, TableInstance, WeakHandle};
use xiao_types::SeededRandom;

/// 同一次执行及其重入析构帧共享的状态；不持有表的强引用。
pub(super) struct TableContext {
    /// 致命故障之后禁止执行用户析构代码。
    pub(super) fatal: Cell<bool>,
    /// 包含重入析构帧的全局调用深度。
    pub(super) depth: Cell<usize>,
    /// 回调和普通代码消费同一随机源。
    pub(super) random: RefCell<SeededRandom>,
    singletons: RefCell<BTreeMap<u32, WeakHandle>>,
    program: RefCell<Option<Rc<TacProgram>>>,
    pending: RefCell<Vec<Fault>>,
    events: RefCell<Vec<VmEvent>>,
    metrics: Cell<VmMetrics>,
}

impl Default for TableContext {
    /// 首次构造表时才复制可执行程序，普通标量运行不承担该开销。
    fn default() -> Self {
        Self {
            fatal: Cell::new(false),
            depth: Cell::new(0),
            random: RefCell::new(SeededRandom::new(0)),
            singletons: RefCell::new(BTreeMap::new()),
            program: RefCell::new(None),
            pending: RefCell::new(Vec::new()),
            events: RefCell::new(Vec::new()),
            metrics: Cell::new(VmMetrics::default()),
        }
    }
}

/// 析构帧的事件缓冲；外层执行器在生命周期边界顺序回放。
struct HookSink(Rc<TableContext>);

impl VmEventSink for HookSink {
    /// 记录事件而不借用外层正在执行的 VM。
    fn record(&mut self, event: VmEvent) {
        self.0.events.borrow_mut().push(event);
    }
}

impl<C: Carrier, S: VmEventSink> Vm<'_, C, S> {
    /// 构造表或升级已声明单例的弱引用。
    pub(super) fn load_table(
        &mut self,
        table: u32,
        construct: bool,
        arguments: &[BoundArgument],
    ) -> Result<RuntimeValue, Fault> {
        let definition = self
            .program
            .table_definitions
            .get(table as usize)
            .cloned()
            .ok_or_else(|| Fault::Error(XiaoError::invalid_value("表定义索引不存在")))?;
        if !construct {
            let singletons = self.tables.singletons.borrow();
            let weak = singletons
                .get(&table)
                .ok_or_else(|| Fault::Error(XiaoError::invalid_value("单例声明尚未执行")))?;
            return TableInstance::from_weak(weak)
                .map(RuntimeValue::Table)
                .map_err(Fault::Error);
        }
        if definition.signature.kind == "singleton"
            && self.tables.singletons.borrow().contains_key(&table)
        {
            return Err(Fault::Error(XiaoError::invalid_value("单例声明被重复执行")));
        }
        let signature = definition
            .signature
            .runtime_signature()
            .ok_or_else(|| Fault::Error(XiaoError::invalid_value("表静态接口无效")))?;
        let mut runtime = TableDefinition::new(signature);
        if let Some(method) = definition.methods.get("ascii:drop").copied() {
            let program = self
                .tables
                .program
                .borrow_mut()
                .get_or_insert_with(|| Rc::new(self.program.clone()))
                .clone();
            let context = Rc::clone(&self.tables);
            let options = self.options;
            runtime = runtime.with_drop_executor(move |object| {
                if context.fatal.get() {
                    return Ok(());
                }
                let mut vm =
                    Vm::<C, HookSink>::new(&program, options, HookSink(Rc::clone(&context)));
                vm.tables = Rc::clone(&context);
                vm.pending_base = context.pending.borrow().len();
                let result = vm.execute(
                    method,
                    &[BoundArgument {
                        keyword: None,
                        value: RuntimeValue::TableDropView(object.drop_view()),
                    }],
                    None,
                );
                context
                    .metrics
                    .set(add_metrics(context.metrics.get(), vm.metrics));
                match result {
                    Ok(_) => Ok(()),
                    Err(Fault::Error(error)) => {
                        context.pending.borrow_mut().push(Fault::Error(
                            XiaoError::table_drop("表 drop 钩子失败").with_cause(error.clone()),
                        ));
                        Err(error)
                    }
                    Err(Fault::Fatal(fatal)) => {
                        context.fatal.set(true);
                        context.pending.borrow_mut().push(Fault::Fatal(fatal));
                        Ok(())
                    }
                }
            });
        }
        self.note_map_point(MapPoint::CallSite);
        if let Some(frame) = self.frames.last_mut() {
            frame.carrier.begin_call();
        }
        let instance = TableInstance::with_initializer(runtime, |instance| {
            let receiver = BoundArgument {
                keyword: None,
                value: RuntimeValue::Table(instance.clone()),
            };
            let result = self
                .execute(definition.fields, std::slice::from_ref(&receiver), None)
                .and_then(|_| {
                    if let Some(init) = definition.methods.get("ascii:init") {
                        let mut bound = vec![receiver];
                        bound.extend_from_slice(arguments);
                        self.execute(*init, &bound, None).map(|_| ())
                    } else if arguments.is_empty() {
                        Ok(())
                    } else {
                        Err(Fault::Error(XiaoError::invalid_value(
                            "没有 init 的表不能接受构造参数",
                        )))
                    }
                });
            match result {
                Ok(()) => Ok(()),
                Err(Fault::Error(error)) => Err(error),
                Err(Fault::Fatal(fatal)) => {
                    self.tables.fatal.set(true);
                    self.tables.pending.borrow_mut().push(Fault::Fatal(fatal));
                    Err(XiaoError::invalid_value("构造被致命故障中断"))
                }
            }
        });
        if let Some(frame) = self.frames.last_mut() {
            frame.carrier.end_call();
        }
        let instance = self.finish_table_effects(instance.map_err(Fault::Error))?;
        if definition.signature.kind == "singleton" {
            self.tables
                .singletons
                .borrow_mut()
                .insert(table, instance.downgrade());
        }
        Ok(RuntimeValue::Table(instance))
    }

    /// 回放已经同步完成的钩子事件和指标，保持外层释放动作的原顺序。
    pub(super) fn sync_table_events(&mut self) {
        let events = std::mem::take(&mut *self.tables.events.borrow_mut());
        for event in events {
            self.sink.record(event);
        }
        self.metrics = add_metrics(
            self.metrics,
            self.tables.metrics.replace(VmMetrics::default()),
        );
    }

    /// 将隐式 Rust 释放产生的错误送回当前 Xiao 执行边界。
    pub(super) fn finish_table_effects<T>(
        &mut self,
        mut result: Result<T, Fault>,
    ) -> Result<T, Fault> {
        self.sync_table_events();
        let pending = self
            .tables
            .pending
            .borrow_mut()
            .split_off(self.pending_base);
        for fault in pending {
            result = match (result, fault) {
                (Err(Fault::Fatal(primary)), _) => Err(Fault::Fatal(primary)),
                (_, Fault::Fatal(fatal)) => Err(Fault::Fatal(fatal)),
                (Err(Fault::Error(mut primary)), Fault::Error(secondary)) => {
                    if !contains_error(&primary, &secondary) {
                        primary.push_suppressed(secondary);
                    }
                    Err(Fault::Error(primary))
                }
                (Ok(_), fault) => Err(fault),
            };
        }
        if matches!(result, Err(Fault::Fatal(_))) {
            self.tables.fatal.set(true);
        }
        result
    }
}

/// Runtime 构造回滚可能已经收录同一个析构原因，避免再次加入 suppressed。
fn contains_error(primary: &XiaoError, secondary: &XiaoError) -> bool {
    primary.error_id() == secondary.error_id()
        || secondary.cause().is_some_and(|cause| {
            primary
                .cause()
                .is_some_and(|existing| existing.error_id() == cause.error_id())
        })
        || primary
            .suppressed()
            .iter()
            .any(|error| contains_error(error, secondary))
}

/// 合并独立帧指标；累计项求和，峰值项取最大。
fn add_metrics(left: VmMetrics, right: VmMetrics) -> VmMetrics {
    VmMetrics {
        instructions: left.instructions.saturating_add(right.instructions),
        max_call_depth: left.max_call_depth.max(right.max_call_depth),
        max_stack_depth: left.max_stack_depth.max(right.max_stack_depth),
        releases: left.releases.saturating_add(right.releases),
        spill_count: left.spill_count.saturating_add(right.spill_count),
        stack_map_entries: left
            .stack_map_entries
            .saturating_add(right.stack_map_entries),
        call_save_count: left.call_save_count.saturating_add(right.call_save_count),
    }
}

/// 字段读取共用 Runtime 签名和状态检查。
pub(super) fn member_get(object: &RuntimeValue, member: &str) -> Result<RuntimeValue, Fault> {
    let value = match object {
        RuntimeValue::Table(instance) => instance.get_compiled_field(member),
        RuntimeValue::TableDropView(view) => view.get_compiled_field(member),
        other => {
            return Err(Fault::Error(XiaoError::type_mismatch(
                "table",
                other.type_name(),
            )));
        }
    }
    .map_err(Fault::Error)?;
    value.ok_or_else(|| {
        Fault::Error(XiaoError::invalid_value(format!(
            "表字段 {member} 尚未初始化"
        )))
    })
}

/// 析构弱视图无写入入口，不能恢复为拥有表对象。
pub(super) fn member_set(
    object: &RuntimeValue,
    member: &str,
    value: RuntimeValue,
) -> Result<(), Fault> {
    match object {
        RuntimeValue::Table(instance) => instance
            .set_compiled_field(member, value)
            .map_err(Fault::Error),
        RuntimeValue::TableDropView(_) => {
            Err(Fault::Error(XiaoError::table_state("writable", "dropping")))
        }
        other => Err(Fault::Error(XiaoError::type_mismatch(
            "table",
            other.type_name(),
        ))),
    }
}

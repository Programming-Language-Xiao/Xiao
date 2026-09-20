//! 调试事件接收器。
//!
//! 事件只在**粗粒度边界**产生：模块加载、函数进出、作用域进出、值释放、错误
//! 和调用栈。刻意不做逐指令回调——逐指令观测会污染后续批次的性能基准，也会让
//! 语义核依赖接收器是否存在。本批次只建立接收器，不实现诊断窗口。

use xiao_bytecode::VReg;

use crate::run::VmMetrics;

/// 生产事件接收器的默认容量。
pub const DEFAULT_EVENT_CAPACITY: usize = 256;
/// 生产事件接收器允许的最大容量。
pub const MAX_EVENT_CAPACITY: usize = 1_000_000;

/// 一条结构化调试事件。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VmEvent {
    /// 模块开始执行。
    ModuleLoaded {
        /// 运行请求提供的逻辑模块身份。
        module: String,
    },
    /// 进入一个函数。
    FunctionEntered {
        /// 函数名。
        function: String,
        /// 进入后的调用深度。
        depth: usize,
    },
    /// 离开一个函数。
    FunctionReturned {
        /// 函数名。
        function: String,
        /// 离开前的调用深度。
        depth: usize,
    },
    /// 进入一个静态作用域。
    ScopeEntered {
        /// 作用域编号。
        scope: u32,
    },
    /// 离开一个静态作用域。
    ScopeExited {
        /// 作用域编号。
        scope: u32,
        /// 退出边稳定名称。
        exit: String,
    },
    /// 开始尝试当前帧的异常处理器路由。
    HandlerEntered {
        /// 处理器所属作用域。
        scope: u32,
        /// 处理器入口块。
        handler: u32,
    },
    /// 错误命中了一个具体处理器。
    HandlerMatched {
        /// 处理器所属作用域。
        scope: u32,
        /// 处理器入口块编号。
        handler: u32,
        /// 匹配的错误类型名称。
        catch_type: Option<String>,
    },
    /// 当前帧没有匹配的处理器。
    HandlerUnmatched {
        /// 发生未匹配的作用域。
        scope: u32,
    },
    /// 按冻结计划释放了一个值。
    ///
    /// 事件带上 `(作用域, 退出边)`，语义向量才能锁定完整释放序列，而不只是
    /// 统计释放个数。
    ValueReleased {
        /// 触发释放的作用域。
        scope: u32,
        /// 退出边稳定名称。
        exit: String,
        /// `IrValue.id`。
        value: u32,
        /// 强释放或弱释放。
        kind: String,
    },
    /// 产生一个可恢复运行时错误。
    ErrorRaised {
        /// 稳定错误码。
        code: String,
        /// 稳定消息编号。
        message_id: String,
    },
    /// 产生一个不可恢复故障。
    FatalRaised {
        /// 稳定故障码。
        code: String,
    },
    /// 一条调用栈记录。
    StackFrame {
        /// 函数名。
        function: String,
        /// 该帧的深度。
        depth: usize,
        /// 该帧的返回值寄存器（若有）。
        return_to: Option<VReg>,
    },
    /// 物理 pc 映射缺失时保留的结构化诊断事件。
    BackendLocationMissing {
        /// 当前函数。
        function: String,
        /// 当前基本块。
        block: u32,
        /// 当前指令在块内的序号。
        instruction: usize,
    },
    /// 一次运行完成时的聚合指标。
    Metrics {
        /// 执行的指令条数。
        instructions: u64,
        /// 达到过的最大调用深度。
        max_call_depth: usize,
        /// 载体占用的历史峰值。
        max_stack_depth: usize,
        /// 按冻结计划释放的值的数量。
        releases: usize,
        /// 写入独立帧槽的次数。
        spill_count: u64,
        /// 需要建立栈映射的程序点数量。
        stack_map_entries: usize,
        /// 跨调用保存值的次数。
        call_save_count: u64,
        /// 因容量上限丢弃的事件数量。
        dropped_events: usize,
    },
}

/// 接收调试事件的目标。
pub trait VmEventSink {
    /// 记录一条事件。
    fn record(&mut self, event: VmEvent);
}

/// 丢弃全部事件的接收器，用于不关心观测的运行。
#[derive(Clone, Copy, Debug, Default)]
pub struct NullSink;

impl VmEventSink for NullSink {
    /// 丢弃事件。
    fn record(&mut self, _event: VmEvent) {}
}

/// 把事件收进向量的接收器，供规格测试与后续诊断窗口消费。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecordingSink {
    events: Vec<VmEvent>,
}

impl RecordingSink {
    /// 创建空接收器。
    #[must_use]
    pub const fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// 返回已记录事件。
    #[must_use]
    pub fn events(&self) -> &[VmEvent] {
        &self.events
    }

    /// 取出已记录事件。
    #[must_use]
    pub fn into_events(self) -> Vec<VmEvent> {
        self.events
    }

    /// 统计某一类事件的数量。
    #[must_use]
    pub fn count_of(&self, predicate: impl Fn(&VmEvent) -> bool) -> usize {
        self.events.iter().filter(|event| predicate(event)).count()
    }
}

impl VmEventSink for RecordingSink {
    /// 追加事件。
    fn record(&mut self, event: VmEvent) {
        self.events.push(event);
    }
}

/// 有界的生产事件接收器。
///
/// 接收器达到容量后丢弃新来的普通事件并递增 [`Self::dropped_events`]，不阻塞
/// 解释器，也不改变执行顺序或释放计划。终止错误事件会优先挤出一条普通事件；
/// 完成指标也会尽量保留。研究测试应继续使用 [`RecordingSink`]，因为它不丢弃事件。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedSink {
    capacity: usize,
    events: Vec<VmEvent>,
    dropped_events: usize,
}

impl BoundedSink {
    /// 创建一个指定容量的有界接收器。
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            events: Vec::with_capacity(capacity.min(DEFAULT_EVENT_CAPACITY)),
            dropped_events: 0,
        }
    }

    /// 创建使用默认容量的生产接收器。
    #[must_use]
    pub fn default_capacity() -> Self {
        Self::new(DEFAULT_EVENT_CAPACITY)
    }

    /// 返回容量上限。
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// 返回已丢弃的普通事件数量。
    #[must_use]
    pub const fn dropped_events(&self) -> usize {
        self.dropped_events
    }

    /// 返回当前保留的事件。
    #[must_use]
    pub fn events(&self) -> &[VmEvent] {
        &self.events
    }

    /// 取出当前保留的事件。
    #[must_use]
    pub fn into_events(self) -> Vec<VmEvent> {
        self.events
    }

    /// 追加一条完成指标，并为它保留容量。
    pub fn record_metrics(&mut self, metrics: VmMetrics) {
        if self.capacity == 0 {
            self.dropped_events = self.dropped_events.saturating_add(1);
            return;
        }
        if self.events.len() >= self.capacity {
            if let Some(index) = self
                .events
                .iter()
                .position(|event| !is_priority_event(event))
            {
                self.events.remove(index);
                self.dropped_events = self.dropped_events.saturating_add(1);
            } else {
                self.dropped_events = self.dropped_events.saturating_add(1);
                return;
            }
        }
        self.events.push(VmEvent::Metrics {
            instructions: metrics.instructions,
            max_call_depth: metrics.max_call_depth,
            max_stack_depth: metrics.max_stack_depth,
            releases: metrics.releases,
            spill_count: metrics.spill_count,
            stack_map_entries: metrics.stack_map_entries,
            call_save_count: metrics.call_save_count,
            dropped_events: self.dropped_events,
        });
    }
}

impl VmEventSink for BoundedSink {
    /// 在容量内保留事件，超限时按丢弃新事件策略记账。
    fn record(&mut self, event: VmEvent) {
        if self.events.len() < self.capacity {
            self.events.push(event);
        } else if is_priority_event(&event) {
            if let Some(index) = self
                .events
                .iter()
                .position(|existing| !is_priority_event(existing))
            {
                self.events.remove(index);
                self.events.push(event);
                self.dropped_events = self.dropped_events.saturating_add(1);
            } else {
                self.dropped_events = self.dropped_events.saturating_add(1);
            }
        } else {
            self.dropped_events = self.dropped_events.saturating_add(1);
        }
    }
}

/// 判断一条事件是否应在容量压力下优先保留。
fn is_priority_event(event: &VmEvent) -> bool {
    matches!(
        event,
        VmEvent::ErrorRaised { .. } | VmEvent::FatalRaised { .. }
    )
}

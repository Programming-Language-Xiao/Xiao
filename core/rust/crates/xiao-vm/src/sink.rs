//! 调试事件接收器。
//!
//! 事件只在**粗粒度边界**产生：模块加载、函数进出、作用域进出、值释放、错误
//! 和调用栈。刻意不做逐指令回调——逐指令观测会污染后续批次的性能基准，也会让
//! 语义核依赖接收器是否存在。本批次只建立接收器，不实现诊断窗口。

use xiao_bytecode::VReg;

/// 一条结构化调试事件。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VmEvent {
    /// 模块开始执行。
    ModuleLoaded {
        /// 模块标签；研究阶段使用目标平台描述。
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

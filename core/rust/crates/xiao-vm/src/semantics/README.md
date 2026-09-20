# `xiao-vm/src/semantics`

工程期 09；这里实现机型无关的 TAC 解释循环和运行时控制流。语义核只依赖 `Carrier` 窄
接口，不认识具体栈、寄存器或混合窗口；旧 `research::semantics` 路径只是兼容重导出。

职责：执行冻结指令、调用与返回、异常/`finally`/释放计划、容器和表生命周期，并通过
`VmEventSink` 报告结构化事件。值运算统一经 `src/ops` 和 `xiao-runtime`。

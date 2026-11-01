# `xiao-vm/src/semantics`

工程期 09；这里实现机型无关的 TAC 解释循环和运行时控制流。语义核只依赖 `Carrier` 窄
接口，不认识具体栈、寄存器或混合窗口；旧 `research::semantics` 路径只是兼容重导出。

职责：执行冻结指令、调用与返回、异常/`finally`/释放计划、容器和表生命周期，并通过
`VmEventSink` 报告结构化事件。值运算统一经 `src/ops` 和 `xiao-runtime`。

20O-B1-FIX-B 的 intrinsic 32 在 `exec.rs` 直接借读指定 RuntimeValue::Error 的 code，
经既有字符串入口复制为拥有值；不读取或改写挂起错误，不消费源错误。类型失败带调用 span，
结果分配失败保留 ResourceError / 011；三种 Carrier 的结构化身份回归由 driver 执行。

B1-IMP 开工前修复 `std.` 既有别名接线：`module_exec.rs` 与归档调用图共用标准库元数据
crate 的规范身份函数，将 `std.text` 等路径统一为 `stdlib.text`，只初始化一次。
并发虚拟模块先按原有特例分派，不新增指令、ABI 或普通包身份。

22C-FIX.1 的 `concurrent.rs` 按 TAC 计划执行原生并发绑定，子线程独立构造 VM，
只传递只读程序、XCON 信封、显式共享句柄和取消/事件通道。`module_exec.rs` 管理模块
初始化及程序上下文切换，不把 `VmSession` 的 `Rc` 状态发送给子线程。

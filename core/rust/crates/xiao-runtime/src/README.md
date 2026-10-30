# `xiao-runtime/src`

放置 Rust Runtime 的值、表、内存、引用计数、错误展开和测试驱动器。06-B 提供对象头与
生命周期基础，10B 已在此接入固定 C ABI 实现；后续 03 容器语义、07 错误报告、09 VM、
10/15 LLVM 和 11C 调试/国际化按子目录继续接入。

## 子目录

- `value/`：标量、`str`、统一值和 `ExternalResource`（`file`/`directory`/`process` 承载，进程退出码缓存）。
- `resources.rs`：VM 与原生 ABI 共用的文件/目录和进程操作，以及 filesystem/process 能力检查。
- `abi_resource.rs`：资源的带能力 ABI 包装，负责参数解码、结果拥有权与错误槽。
- `abi_version.rs`：ABI 版本查询及启动兼容比较，委托 ABI crate 的参数化规则。
- `memory/`：不透明对象头、强/弱句柄和计数策略。
- `tables/`：静态表签名与构造状态机。
- `containers/`：数组、元组、字典表、字典列和集合（工程期 09R2）。
- `abi.rs`：10B 固定 C ABI 的核心符号与句柄实现（含 `XiaoValueTag::ExternalResource = 17`）；数组、元组、字典、集合和旧表入口在 `abi_collections.rs`；动态检查、迭代/字符串入口在旁置的
  `abi_dynamic.rs`，原生诊断握手与事件入口在 `abi_diagnostics.rs`；异常对象与展开仍留给 N0-C。
- `errors/`：结构化 Runtime 错误和展开累加器。
- `testing/`：只用于规格测试的释放计划驱动器。
- `vm_concurrency.rs`：22C-FIX.1 的单线程 VM 句柄、可发送 XCON 信封和结构化子执行结果；
  `vm_concurrency_tests.rs` 验证复制字节、锁守卫寿命、消费身份和限制。

- `abi_tables.rs`：10T 注册式表方法、元数据复制、弱只读析构视图与错误边界；
  `abi_table_tests.rs` 覆盖初始化回滚、别名、视图失效和非法元数据。

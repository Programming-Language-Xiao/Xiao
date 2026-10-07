# `xiao-runtime/src`

放置 Rust Runtime 的值、表、内存、引用计数、错误展开和测试驱动器。06-B 提供对象头与
生命周期基础，10B 已在此接入固定 C ABI 实现；后续 03 容器语义、07 错误报告、09 VM、
10/15 LLVM 和 11C 调试/国际化按子目录继续接入。

## 子目录

- `value/`：标量、`str` 和统一值。
- `memory/`：不透明对象头、强/弱句柄和计数策略。
- `tables/`：静态表签名与构造状态机。
- `containers/`：数组、元组、字典表、字典列和集合（工程期 09R2）。
- `abi.rs`：10B 固定 C ABI 的核心符号与句柄实现；动态检查、迭代/字符串入口在旁置的
  `abi_dynamic.rs`，原生诊断握手与事件入口在 `abi_diagnostics.rs`；异常对象与展开仍留给 N0-C。
- `errors/`：结构化 Runtime 错误和展开累加器。
- `testing/`：只用于规格测试的释放计划驱动器。

- `abi_tables.rs`：10T 注册式表方法、元数据复制、弱只读析构视图与错误边界；
  `abi_table_tests.rs` 覆盖初始化回滚、别名、视图失效和非法元数据。

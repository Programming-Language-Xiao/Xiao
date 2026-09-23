# `xiao-vm`

## 目录职责

Rust 字节码唯一执行实现：解释循环、调用栈、局部槽、模块加载、错误回溯、确定性随机源和诊断事件。

## 工程期

09 最小运行闭环；14 接入优化 `.xiaoc` 加载和只读验证；后续可在独立阶段加入 JIT，不原地修改公开字节码。

## 模块放置

`src/` 下按 `interpreter`、`stack`、`loader`、`debug` 和 `random` 分模块；REPL 缓冲区放在 TypeScript CLI。

## 09R 沿革与兼容边界

09R2 的实质实现已经迁入 `src/`，按「机型无关语义核 + 可替换载体」两层组织：
`semantics/` 只认识三地址指令与 `carrier.rs` 的窄接口，不认识「栈」这个词；
`machine/stack.rs`、`machine/register.rs`、`machine/hybrid.rs` 分别实现三种载体；`ops.rs`
统一提供算术、精确索引、高级选择、随机和事务性广播，`sink.rs` 记录结构化事件，`run.rs`
提供结果与指标。`run_request`/`run_production` 是生产入口：它携带 IR、TAC、模块和源码身份，
先做 `verify_for_execution`，再固定使用栈式载体；`RunOutcome.value` 在脚本与 `[main]` 中都表示
入口显式返回值。`run_with_values` 仍只供研究夹具注入动态边界。
三种载体复用同一选择器、集合、迭代和表声明语义及 79 条共享向量，选择器结果值由
`tests/r2_selector_values.rs` 的 7 条手工 TAC 夹具断言，集合指令由
`tests/r2_set_values.rs` 的手工 TAC 夹具和 `sets.json` 入口向量共同断言，迭代指令由
`tests/r2_iteration_values.rs` 与 `iteration.json` 共同断言；表生命周期由
`tests/r2_table_values.rs` 与 `tables.json` 验证结果、错误链、析构次数和完整释放序列。

`src/research/` 现在是兼容重导出层，保留旧路径供 09R 共享向量和基准设施使用；生产入口
固定使用冻结的栈式载体，寄存器式与混合式载体仍保留用于复现和对比。别名层的移除条件是
B0-C 交付且生产驱动器成为唯一消费方，不得提前删除。

生产事件默认进入固定容量的 `BoundedSink`；超过容量的普通事件按丢弃新事件策略记账，
完成指标仍通过 `Metrics` 事件和 `RunOutcome.metrics` 提供。`RecordingSink` 继续用于规格测试，
因此事件接收策略不会改变语义或释放计划。

## 09-B0-E 取消检查点

`CancellationToken` 和 `CancellationSource` 是 VM 侧的可注入控制源，支持跨线程取消和绝对
截止时间。`VmOptions.checkpoints_enabled` 控制运行期开关，`checkpoint_interval` 控制轮询
频率；检查点计数跨普通调用帧和析构帧共享，且不改变 `metrics.instructions`。
取消使用独立的 `Cancelled` 通道，绕过用户 `catch`，但复用既有 `finally` 与释放计划机制；
`Fatal` 和普通 `Error` 的既有不对称语义保持不变。性能开关对照见
`docs/DevDocs/09b0e-checkpoint-performance.json`，不属于 09R3 冻结数字。

冻结结论与施工顺序见 [09R. 字节码寄存器机型特别研究](../../../../docs/DevDocs/09r-bytecode-machine-research.md)。

## 边界

不处理 TypeScript REPL 状态、不解析 CLI 参数、不依赖人类可读日志判断控制流；用户可见的
`xiao run` 仍归 X0。

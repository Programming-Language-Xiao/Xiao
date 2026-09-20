# `xiao-vm`

## 目录职责

Rust 字节码唯一执行实现：解释循环、调用栈、局部槽、模块加载、错误回溯、确定性随机源和 `-debug` 诊断事件。

## 工程期

09 最小运行闭环；14 接入优化 `.xiaoc` 加载和只读验证；后续可在独立阶段加入 JIT，不原地修改公开字节码。

## 模块放置

`src/` 下按 `interpreter`、`stack`、`loader`、`debug` 和 `random` 分模块；REPL 缓冲区放在 TypeScript CLI。

## 09R 研究边界

`src/research/` 已落地 09R2 首版，按「机型无关语义核 + 可替换载体」两层组织：
`semantics/` 只认识三地址指令与 `carrier.rs` 的窄接口，不认识「栈」这个词；
`machine/stack.rs`、`machine/register.rs`、`machine/hybrid.rs` 分别实现三种载体；`ops.rs`
统一提供算术、精确索引、高级选择、随机和事务性广播，`sink.rs` 记录结构化事件，`run.rs`
提供结果与指标；研究入口的 `RunOutcome.value` 只用于观察显式返回值，不是生产 API。
三种载体复用同一选择器、集合、迭代和表声明语义及 79 条共享向量，选择器结果值由
`tests/r2_selector_values.rs` 的 7 条手工 TAC 夹具断言，集合指令由
`tests/r2_set_values.rs` 的手工 TAC 夹具和 `sets.json` 入口向量共同断言，迭代指令由
`tests/r2_iteration_values.rs` 与 `iteration.json` 共同断言；表生命周期由
`tests/r2_table_values.rs` 与 `tables.json` 验证结果、错误链、析构次数和完整释放序列。

该子模块属于 09R 特别研究工程，**不是稳定执行接口**：三种机型在 09R3 冻结前都不得被当作
生产 VM 暴露，也不得让 TypeScript 层依赖其中任何一型的内部结构。

冻结结论与施工顺序见 [09R. 字节码寄存器机型特别研究](../../../../docs/DevDocs/09r-bytecode-machine-research.md)。

## 边界

不处理 TypeScript REPL 状态、不解析 CLI 参数、不依赖人类可读日志判断控制流。

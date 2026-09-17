# 08A. U0 统一前端实现交接记录

> 本记录对应 08 阶段的首个可实施闭环。它把已经完成的词法、语法、模块、类型和生命周期分析串成一次前端请求，并输出经过验证的类型化 IR。接手 Agent 必须先阅读 [08. 前端与统一中间表示](08-frontend-pipeline.md)、[05A–05D 模块/表/配置交接记录](05a-import-syntax.md)、[06A 生命周期静态闭环](06a-lifetime-static-closure.md) 和 [07-B 错误控制流交接](07-concurrency-and-errors.md#07-b-已完成错误控制流与统一展开消费)。

## 一级工程目标：统一入口

### U0-A 已实现

`xiao-driver` 提供 `FrontendRequest`、`FrontendContext`、`FrontendCompiler` 和 `FrontendArtifact`。请求包含 UTF-8 `SourceFile`、可选源路径、项目根、已校验 `config.xiao` 摘要、可选外部包图摘要、语言版本和目标平台。

固定流水线为：

```text
SourceFile -> Parser -> 本地模块分析 -> TypeChecker -> LifetimeAnalyzer -> xiao-ir 降低 -> IrValidator
```

每个阶段只消费前序公开结果。解析、模块、类型和生命周期诊断会累积；存在错误时停止降低，不返回后端可消费 IR。前端不执行配置值、模块初始化、表钩子或用户代码。

## 一级工程目标：类型化 IR

### U0-B 已实现

`xiao-ir` 使用不可变递归枚举树，不复用 AST，也不暴露宿主地址。`IrProgram` 包含：

- 版本、语言版本、目标平台、配置存在性和外部包身份；
- 脚本/工程入口、递归语句和表达式；
- 标量、动态值、数组形状、元组、字典表/字典列、集合和表类型；
- 精确路径、多选、范围、步长、`?`/`!?` 随机选择；
- 函数、调用参数、条件/循环、`try/catch/finally/raise`；
- 模块符号摘要、生命周期控制流、所有权边和释放计划；
- 源码字节区间及运行时检查标记。

集合仍然无序；选择器 IR 不允许集合索引。错误控制流沿用 07-B 的恢复边界和释放计划，不创建第二套异常语义。

### 后端边界

字节码和 LLVM 后端只能读取已验证 `IrProgram`。它们不得重新解析源码、重新解析模块、重新推断类型或自行计算释放顺序。U0 不实现 VM、LLVM、优化 Pass、Runtime 容器或 CLI。

## 一级工程目标：验证与快照

### U0-C 已实现

`IrValidator` 检查版本、源码区间、递归类型、名称、选择器、集合无索引约束、`try` 处理器、控制流后继、作用域/值编号、所有权边和释放计划。验证失败返回 `X08-IR-001`/`X08-IR-002` 结构化错误，禁止后端继续消费。

`to_json`/`from_json` 使用显式版本字段和固定结构顺序。降低器对映射、基本块、作用域、所有权边和释放计划进行确定性排序；快照不包含宿主地址或哈希表迭代顺序。当前快照版本为 1。

## 二级实现任务与接手顺序

1. **U0-A.1 请求与阶段边界**：先阅读 `xiao-driver/src/frontend.rs`，保持诊断累积和错误即停降低规则。
2. **U0-B.1 IR 扩展**：在 `xiao-ir/src/model.rs` 增加节点时同步更新降低器、验证器、JSON 快照和测试；不得把 CLI/Runtime 类型引入 IR。
3. **U0-C.1 验证不变量**：新增后端要求时先增加验证错误和非法结构测试，再允许后端消费。
4. **U0-C.2 快照兼容**：修改字段必须提升 IR/快照版本或提供显式迁移，不能静默改变既有快照含义。
5. 每次模块代码变更必须同提交更新 crate README、分层 UseDocs、测试 README 和 `docs/module-registry.json`。

## 验收与后置债项

- `core/rust/crates/xiao-ir/tests/u0_ir.rs` 覆盖完整静态语义降低、类型携带、快照往返和验证失败。
- `core/rust/crates/xiao-driver/tests/u0_frontend.rs` 覆盖成功、诊断累积和上下文元数据。
- workspace 测试、Clippy、Rustdoc、仓库检查、文档覆盖率和 `git diff --check` 必须通过。
- 当前模块只登记静态前端结果；字节码/LLVM 消费、跨后端差分和真实执行债项分别带入 09/10 阶段。

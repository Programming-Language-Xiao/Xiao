# 06-B / R0-B：Runtime 对象与表生命周期执行闭环

> 本文是 06-B 的实现交接记录。它把 06-A 的静态身份和释放计划降低为 Rust Runtime
> 的可执行句柄与表状态机，但不提前实现字节码 VM、LLVM、CLI、并发模型或容器全集。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：Runtime 使用 Rust、无追踪式 GC、错误字段和禁止高度耦合规则。
2. [00A. 工程框架与目录布局](00a-project-layout.md)：crate 边界、目录 README 和文档门禁。
3. [05C. 表语法与静态生命周期闭环](05c-table-static-closure.md)：`TableSignature`、成员可见性和 `init/drop` 静态契约。
4. [06A. 生命周期静态闭环](06a-lifetime-static-closure.md)：`ValueId`、所有权边方向和八类 `ReleasePlan`。
5. [06. 内存与运行时语义](06-memory-and-runtime.md)：整个 06 阶段的释放顺序与后置容器边界。
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)：本阶段测试、Rustdoc 和 UseDocs 退出门槛。

### 已冻结输入与输出

| 项目 | 约定 |
| --- | --- |
| 静态输入 | `xiao-types::TableSignature`、`Type`、`xiao-lifetime::ReleasePlan` |
| Runtime 语言 | Rust；TypeScript、VM、LLVM 不在本阶段调用内部对象 |
| 引用计数 | 策略可插拔；首版单线程非原子，句柄不可跨线程传递 |
| 对象头 | 私有不透明布局：类型标签、强/弱计数、布局摘要、释放钩子和销毁标记 |
| 表形态 | `[Table]` 单例、`[[Table]]` 实例；只接受 05-C 已验证的成员签名 |
| 错误 | 小型 `RuntimeError` 句柄，详细数据堆分配；保留 `code`、`message_id`、参数、原因链和 `suppressed` |

### 明确不负责

- 不解析 Xiao 源码，不执行 `def`、`try/catch/finally/raise` 语法，不生成字节码或 LLVM IR。
- 不实现数组、元组、集合、字典、选择器、I/O、调试窗口、包管理和并发调度。
- 不在 Runtime 重新推断表签名、可见性或生命周期图；这些事实必须来自前置 crate。
- 不把中文错误预览当作机器接口；完整语言目录由 11C 接入。

## 一级工程目标：统一 Runtime 值边界

### 06B-1：不透明对象头与安全句柄

`memory` 模块用 `#[repr(C)]` 私有对象头保存 `RuntimeTypeTag`、强/弱计数、
`ObjectLayout`、计数策略、销毁状态和 `ObjectPayload`。`StrongHandle` 负责拥有对象，
`WeakHandle` 只保留对象头；上层只能查询标签/计数、克隆、降级或升级，不能取得裸指针。

固定宽度标量直接放在 `RuntimeValue` 中；`str` 和表通过强句柄放在堆上。`lint`/`lfloat`
首版以规范文本占位，复杂高精度算术不得伪装成固定宽度运算。

### 06B-2：引用计数策略

`RefCountStrategy` 提供 checked 增减和稳定名称；`CounterStrategyKind::NonAtomic` 是
当前唯一实现。最后一个强句柄归零时先执行载荷钩子，再释放隐式弱引用；最后一个弱句柄
归零时才释放对象头。计数下溢、溢出、重复销毁和已释放访问必须产生稳定 Runtime 错误。

`Rc` 标记保留在句柄类型中，以便编译器拒绝首版跨线程传递。未来原子策略只能替换计数
存储和同步实现，不得改变 `Weak` 不拥有目标、最后强引用立即释放等语义。

## 一级工程目标：执行表生命周期

### 06B-3：表定义与状态机

`TableDefinition` 持有一份 `TableSignature` 和可选的 Rust 测试/后端钩子。表实例状态严格
按以下路径迁移：

```text
Allocated -> FieldsInitializing -> InitCompleted -> Usable
                                  \-> Dropping -> Released
Usable ---------------------------------------> Dropping -> Released
```

`TableInstance::new` 只接受 `TableKind::Instance`；`TableInstance::singleton` 只接受
`TableKind::Singleton`。字段写入必须命中签名中的字段并通过 `Type` 对应的 Runtime 值检查；
方法不能当作字段，私有成员不能从表外读写，运行时不能新增或改名成员。

`init` 失败进入 `ConstructFailure` 清理路径，已建立的字段和句柄仍须释放；`drop` 钩子
只执行一次，执行期间可以只读观察已初始化字段，成功或失败后状态都变为 `Released`。
清理失败包装为 `X06-RUNTIME-008`，
原始错误保留在 `cause`。

### 06B-4：值与最小运算

`value` 模块提供 `str` 的 UTF-8 堆存储、Unicode 标量长度、只读访问和字符串拼接。
布尔运算严格遵守语言规则：仅当左值是 `bool`、右值是整数时，按整数绝对值奇偶翻转或
保持，结果仍为 `bool`；布尔不能与浮点或另一个布尔做加减。固定宽度整数和有限浮点
加减使用 checked/finite 检查，溢出返回 `X06-RUNTIME-009`。

## 一级工程目标：释放计划与错误展开

### 06B-5：计划驱动器

`testing` 模块的 `RuntimeDriver` 将 06-A 的 `ReleaseAction` 映射到强/弱句柄，记录
稳定事件序列，并提供 `unwind` 测试接口。展开顺序固定为：

1. 执行预先确定的 `finally` 钩子；
2. 按 `ReleasePlan` 顺序释放资源；
3. 把主错误交给后续 `catch` 或程序边界；
4. 清理阶段产生的错误追加到主错误的 `suppressed`，不覆盖主错误。

驱动器在首个清理失败后仍继续处理剩余动作，避免错误路径泄漏绑定。缺少绑定或强/弱
动作不匹配属于 Runtime 不变量错误，事件仍记录，供测试报告完整展开结果。

### 错误字段和国际化边界

`RuntimeError` 本身是小型堆句柄，详细数据不可变地保存稳定 `code`、`message_id`、
`DiagnosticParams`、`SourceSpan`、`cause`、`suppressed` 和 `error_id`。本阶段只提供中文
预览；后续渲染器可以根据同一消息键重新生成文本，但不得改变机器字段、退出类别或控制流。

## 二级实施 SOP

### SOP-06B-1：对象头和计数

1. 先阅读 `memory/README.md`，确认所有裸指针只留在模块内部。
2. 增加对象分配、强/弱克隆、升级、归零销毁和计数不变量测试。
3. 用 `cargo miri` 或等价工具审查销毁顺序；不得在弱句柄仍存在时释放对象头。

### SOP-06B-2：表和错误

1. 由 05-C 测试构造 `TableSignature`，禁止手写第二份成员类型规则。
2. 覆盖单例/实例区分、状态迁移、字段类型、私有访问、`init` 失败和 `drop` 失败。
3. 验证主错误、原因链和 `suppressed` 不被清理错误覆盖。

### SOP-06B-3：跨阶段接线准备

1. 以 `xiao-lifetime::ReleasePlan` 作为唯一释放顺序输入，保留 `ScopeId`、`ValueId` 和 `ExitKind`。
2. 为 08 IR 记录 Runtime 句柄边界和动态检查位置，不在本阶段建立 IR。
3. 09/10 接入时复用句柄与测试断言，禁止 VM/LLVM 另写表生命周期实现。

## 测试与验收

### 当前测试入口

- `cargo test --manifest-path core/rust/Cargo.toml -p xiao-runtime`
- `cargo test --manifest-path core/rust/Cargo.toml -p xiao-runtime --test b06_runtime`
- `cargo clippy --manifest-path core/rust/Cargo.toml -p xiao-runtime --all-targets -- -D warnings`
- `cargo doc --manifest-path core/rust/Cargo.toml -p xiao-runtime --no-deps`

### 退出条件

1. 句柄、Weak、对象头和非原子计数测试全部通过，且没有追踪式 GC 或跨线程自动实现。
2. 表实例严格遵守状态机，`init`/`drop` 失败都能确定性清理并保留结构化错误。
3. 释放计划驱动器覆盖正常、返回、错误和构造失败代表性路径，清理错误不覆盖主错误。
4. Rust 工作区、Clippy、Rustdoc、目录完整性、文档覆盖率和 UseDocs 门禁全部通过。
5. 本文、crate 分层 README、UseDocs、模块登记与实现测试在同一变更集中提交。

## 后续交接

09R2H 已完成研究 VM 接线：IR 保留静态表签名镜像，后端通过初始化闭包与可捕获上下文的
析构回调复用本阶段状态机。编译后字段访问保留类型/状态检查，析构接收者使用弱只读视图；
原有函数指针钩子继续有效。新增 Runtime 边界回归、三载体执行测试与完整释放向量见
[09R2H 交接记录](09r2h-table-declarations.md)，正式生产入口仍未开放。

06-B 完成后，08 前端应把 `TableSignature` 和 `LifetimeResult` 原样保留到统一 IR；09
字节码 VM 先消费 `RuntimeValue`/句柄与错误展开接口；10 LLVM 后端只链接按需 Runtime。
数组、元组、集合、字典和选择器的真实存储由独立 Runtime 子阶段补充，不能把本阶段的
标量/表实现扩张成高度耦合的单文件运行库。

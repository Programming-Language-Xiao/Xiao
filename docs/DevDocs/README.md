# Xiao 开发文档

> 本目录记录 Xiao 从规格冻结到完整工具链的开发主线。README 只负责说明阅读顺序、阶段状态和当前里程碑；具体语言规则以对应阶段文档为准。

## 阅读与维护方式

### 贯穿文档

以下文档不属于首尾相接的实施阶段，而是在整个开发周期持续维护：

| 文档 | 作用 | 状态 |
| --- | --- | --- |
| [00. 决策基线](00-decisions.md) | 汇总已经确认的语言边界、跨阶段约束和待定决策；每次设计确认后首先更新 | 进行中 |
| [00A. 工程框架与目录布局](00a-project-layout.md) | Rust 核心、TypeScript CLI、平台、测试、工具和资源的目录分配与文档质量门槛 | 进行中 |
| [12. 测试与开发里程碑](12-tests-and-milestones.md) | 为每个实施阶段规定测试分层、子里程碑和退出条件；从第一阶段起同步执行，不是最后才实施的测试阶段 | 进行中 |
| [A0. 工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md) | Rust/Bun workspace 清单、目录完整性检查器、文档覆盖率检查器和 UseDocs 同步门禁的可执行契约 | 已完成 |
| [00C. 文档 lint 接线实现交接](00c-doc-lint-wiring.md) | 统一 20 个 Rust crate 的 `missing_docs` 接线，补齐 rustc 与 repo-check 两套口径的文档缺口；实现提交 `6d296c3` | 已完成（方案 B） |
| [00D. Rust AST 适配器协议 v2 修正交接](00d-doc-adapter-protocol-v2-fixes.md) | 修正 `0e9a42b` 的 `end_line` 死字段、大纲行数口径与协议契约文档，并给请求体加大纲开关；是单文件行数门禁的前置 | 已完成 |
| [00E. 单文件行数门禁交接](00e-file-size-gate.md) | `A0-SIZE-001` 的判定与树形大纲渲染、TS 侧结构大纲、旁置 md 豁免机制；超标文件按批次拆分 | 已完成（门禁已启用，当前无尺寸债务） |
| [00F. 解析器第二批模块解耦交接](00f-parser-decoupling.md) | 将 `parser.rs` 的语句/控制流实现拆到 `parser/statements.rs`，保持解析 API 与诊断兼容 | 已完成 |

实现边界速览：字节码 VM 与执行 Runtime 的 Rust 决策见 [00. 决策基线](00-decisions.md) 和 [09. 字节码运行模式](09-bytecode-runtime.md)；字节码机型、调用约定与编码的研究冻结过程见 [09R. 字节码寄存器机型特别研究](09r-bytecode-machine-research.md)；TypeScript CLI/REPL 边界见 [11. CLI、项目配置与平台](11-cli-config-and-platform.md)；Java 对照性能目标与验收口径见 [19. 优化、兼容性与发布验收](19-optimization-release.md)。

面向自然人的使用文档从 [`docs/UseDocs/README.md`](../UseDocs/README.md) 开始。UseDocs 与本目录分离：本目录写设计、实现和交接，UseDocs 写已经验证的安装、操作和排错路径。

### 状态说明

- **未开始**：尚未进入实现。
- **进行中**：正在设计、实现或验证，尚未满足退出条件。
- **已完成**：对应阶段当前可执行的实现任务与退出条件均已通过；依赖后置消费者的集成契约会继续由后续阶段追踪。
- **受阻**：存在必须先解决的规格、依赖或工程问题。

每个实施阶段至少拆分为“一级工程目标”和“二级实现任务”两个层级。阶段状态只有在对应的当前阶段验收与测试门槛通过后才能更新；一次会话或一次提交不等于一个开发阶段。早期文档中要求“字节码与原生一致”的条目属于后续集成契约，在第 09、10 阶段具备执行条件后补跑，不能反过来阻止为它们建设前端和运行时。

第 00A、11C、13 至 19 阶段的文档额外提供“Agent 交接上下文”：接手代理必须先阅读列出的前置文档，确认输入、交付物和不负责事项，再执行带编号的子任务；未决决策不得通过临时实现偷偷冻结。

## 实际开发顺序

下面的顺序是实施顺序，不只是文档分类。先完成 `00A` 工程骨架，再从 `01` 向 `19` 推进；`12` 仍是贯穿所有阶段的测试门槛。后续阶段可以提前完善规格，但不能绕过前置阶段当前可执行的退出条件开始正式实现。尚缺后置消费者才能运行的集成测试会作为显式债项带入对应后续阶段，不能被误记为已经通过。

**`01`–`11` 的阶段级「进行中」已逐条核实**（2026-10-03）：**没有一个是状态滞后，每个都有真实的验收缺口**。
逐条结论、证据与三类处置建议见 [12A. 阶段验收核实记录](12a-stage-acceptance-audit.md)。

| 顺序 | 阶段与文档 | 主要交付物 | 状态 |
| --- | --- | --- | --- |
| 00A | [工程框架与目录布局](00a-project-layout.md) | Rust 核心、TypeScript CLI、平台、测试、工具和资源骨架 | **已完成**（文档覆盖率实测 91.91% ≥ 90%、公共 API 100% = 100%，见 [12A](12a-stage-acceptance-audit.md)） |
| 00A.1 | [工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md) | 实际 workspace 清单、目录检查、UseDocs 登记和覆盖率报告契约 | 已完成 |
| 00C | [文档 lint 接线实现交接](00c-doc-lint-wiring.md) | 20 个 Rust crate 的 `missing_docs` 接线、44 个字段 Rustdoc、两套口径复测（提交 `6d296c3`） | 已完成（方案 B） |
| 00D | [Rust AST 适配器协议 v2 修正交接](00d-doc-adapter-protocol-v2-fixes.md) | `Declaration.end_line` 真实语义、大纲 `lines` 口径统一、`declaration_head` 文档对齐、`outline` 请求开关、协议 UseDoc 同步 | 已完成 |
| 00E | [单文件行数门禁交接](00e-file-size-gate.md) | `A0-SIZE-001` 判定与树形大纲渲染、TS 结构大纲、旁置 md 豁免机制、七处文档同步 | 已完成（门禁已启用，当前无尺寸债务） |
| 00F | [解析器第二批模块解耦交接](00f-parser-decoupling.md) | `parser.rs` 语句/控制流拆分、依赖边界、兼容契约和架构回归测试 | 已完成 |
| 01 | [词法 Token 与语法入口](01-lexical-and-grammar.md) | 源码位置模型、最小 Token 流、完整词法器和解析器入口 | 进行中 |
| 01A | [F0/L0 实现交接记录](01a-f0-l0-implementation.md) | UTF-8 源码位置、最小 Token、统一诊断和规格快照 | 已完成 |
| 01B | [L1 基础词法扩展交接记录](01b-l1-implementation.md) | 字面量、保留字、括号、运算符和错误恢复 | 已完成 |
| 01C | [L2 反引号、注释与缩进实现交接记录](01c-l2-implementation.md) | UTF-8 名称、文档注释、缩进状态机和结构诊断 | 已完成 |
| 01D | [P0 最小解析器与 AST 实现交接记录](01d-p0-parser-implementation.md) | 多条顶层语句、字面量/名称/简单赋值 AST、文档注释挂接和错误恢复 | 已完成 |
| 01E | [P1 表达式与选择器实现交接记录](01e-p1-expression-selectors.md) | Pratt 表达式核心、调用/转换、索引路径和高级选择器 AST | 已完成 |
| 01F | [P2-A 语法模块解耦交接记录](01f-p2a-syntax-decoupling.md) | 将语法门面拆为职责单一模块，保持 P0/P1 API 兼容 | 已完成 |
| 02 | [类型与值系统](02-type-system.md) | 类型表示、推断、静态检查和动态值边界 | 进行中 |
| 02A | [P2-B/S0 静态标量类型实现交接记录](02a-p2-static-types.md) | 声明 AST、HM 基础算法、作用域、转换和标量检查 | 已完成首批 |
| 03 | [容器、集合与索引路径](03-collections.md) | 数组、元组、集合、字典表、字典列和路径约束 | 进行中（C0、C1、C2-A、C2-B、C2-C 已完成静态闭环） |
| 03A | [C0 基础容器与精确路径实现交接记录](03a-c0-containers.md) | 容器 AST、结构化类型、声明路径和单项精确索引 | 已完成 |
| 03B | [C1 有序容器选择器实现交接记录](03b-c1-ordered-selectors.md) | 多选、范围、步长、随机计划、结果形状和标量广播 | 已完成静态阶段 |
| 03C | [C2-A 最小集合静态闭环交接记录](03c-c2a-sets.md) | 集合 AST、单一元素类型、可哈希诊断和成员判断 | 已完成静态阶段 |
| 03D | [C2-B 异构集合与动态成员静态闭环](03d-c2b-heterogeneous-sets.md) | 默认异构集合、`set<T | U>` 注解、动态尾标和并集成员判断 | 已完成静态阶段 |
| 03E | [C2-C 集合运算静态闭环](03e-c2c-set-operations.md) | `+`、`&`、`-`、`^`、集合比较、四种原地运算和动态检查计划 | 已完成静态阶段 |
| 04 | [函数与控制流](04-functions-and-control.md) | `def`、表达式、控制流和入口规则 | 已完成静态阶段（04-A 至 04-D） |
| 05 | [表、模块与工程模型](05-tables-and-projects.md) | 表生命周期、源码模块、依赖图和包外导出 | 进行中（05-A 至 05-D 已完成静态闭环） |
| 05A | [绝对导入语法实现交接记录](05a-import-syntax.md) | `import`/`from` AST、别名、嵌套位置和错误恢复 | 已完成 |
| 05B | [本地模块发现与依赖图实现交接记录](05b-local-module-resolution.md) | 文件模块、目录命名空间、绑定、再导出和循环诊断 | 已完成 |
| 05C | [表语法与静态生命周期闭环交接记录](05c-table-static-closure.md) | `[Table]`/`[[Table]]` AST、成员签名、可见性、`new/init/drop` 静态契约 | 已完成静态阶段 |
| 05D | [`config.xiao` 声明式配置静态闭环](05d-config-static-closure.md) | 独立配置树、静态值、项目身份、包外导出和不可执行诊断 | 已完成静态阶段 |
| 06 | [内存与运行时语义](06-memory-and-runtime.md) | 确定性释放、逃逸分析和引用计数 | 进行中（06-A、06-B 已完成；容器 Runtime 由 09R2 第二批交付） |
| 06A | [生命周期静态闭环交接记录](06a-lifetime-static-closure.md) | 作用域、控制流、逃逸事实、强/弱所有权图和退出释放计划 | 已完成静态阶段 |
| 06B | [Runtime 对象与表生命周期执行闭环](06b-runtime-objects-and-tables.md) | 不透明对象头、Strong/Weak、标量/str、表状态机和释放展开驱动器 | 已完成首版 |
| 07 | [错误模型与并发安全边界](07-concurrency-and-errors.md) | 统一错误核心、堆栈/报告契约、日志诊断、数据竞争策略和并发模型边界 | 进行中（07-A、07-B 已完成，07-C/07-D 后置） |
| 07B | [错误控制流与统一展开消费](07-concurrency-and-errors.md#07-b-已完成错误控制流与统一展开消费) | `try`/`catch`/`finally`/`raise` 的语法、静态恢复边界、生命周期展开和 Runtime 路由契约 | 已完成首版 |
| 08 | [前端与统一中间表示](08-frontend-pipeline.md) | 从词法到类型化 IR 的统一编译前端 | 进行中（08A/U0 已完成首版） |
| 08A | [U0 统一前端实现交接记录](08a-u0-frontend-implementation.md) | 单一前端流水线、递归类型化 IR、验证器和稳定 JSON 快照 | 已完成首版 |
| 09 | [字节码运行模式](09-bytecode-runtime.md) | Rust 字节码解释器、执行 Runtime 与 `xiao run` 接口 | 进行中（09R1–09R3 已冻结，B0-A/B/C/D 已交付；用户可见 CLI 仍留给 11/X0；批次边界见 [09-B0](09b0-bytecode-closure.md)） |
| 09R1 | [字节码寄存器机型特别研究](09r-bytecode-machine-research.md#一级工程目标统一三地址语义模型) | 统一三地址语义、三种候选机型、寄存器类别与编号空间、调用约定、异常与清理转移、编码草案和基准协议 | 已完成首版 |
| 09R2 | [字节码寄存器机型特别研究](09r-bytecode-machine-research.md#一级工程目标三种候选机型) | 三种机型的可运行原型、共享语义向量、事件接收器和容器/选择器/集合/迭代/表声明路径 | 已完成（R2a、R2C、R2D、R2b、R2b1、R2F/2F1、R2G、R2H 与 R3 全部交付并冻结） |
| 09R2c | [异常控制流实现交接文档](09r2c-exception-control-flow.md) | 运行时错误对象、错误类型名单一来源、handler 表与 catch 路由、`finally` 子程序和 `Check` 降低 | 已完成（R2B 选择器错误复用同一异常路由） |
| 09R2d | [两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) | 逐函数类别映射修复、`Carrier` 接口演进、活跃区间分析、寄存器与混合式机型、指令编码器、`pc -> span` 映射 | 已完成（编码器基线 31 个，后续扩展至 41 个 opcode、布局版本 3；研究模块仍保持 draft，待 R2 总阶段退出） |
| 09R2e | [研究编码器模块解耦交接记录](09r2e-research-encoder-decoupling.md) | `research::encode` 门面与 `codec`/`tags`/`validate`/`encoder`/`decoder`/`tests` 依赖 DAG、兼容契约和架构回归测试 | 已完成 |
| 09R2b | [选择器全量执行交接文档](09r2b-selector-execution.md) | 步长接线、`SelectionPlan` 消费方式、高级选择的 TAC 操作数格式与运行时执行、`RandomSource` 注入、结果形状构造、左值广播写入 | 已完成（31 条共享向量、53 条栈式测试、四类 RuntimeCheck） |
| 09R2b1 | [选择器执行验证缺口修复交接文档](09r2b1-selector-verification.md) | 选择器结果值的可观察性、能断言选择结果的区分度用例、错误码字面量回退清理 | 已完成（7 条三机型值断言、受控回退验证、错误码字面量清零） |
| 09R2f | [集合运算执行闭环交接文档](09r2f-set-operations.md) | `SetOp`/`SetCompare` 两条指令、`SetHandle` 代数与六种比较、四个集合类 RuntimeCheck、`sets.json` 共享向量 | 已完成（由 09R2F1 接续补齐运行时检查、验证夹具和文档） |
| 09R2f1 | [集合运算执行闭环续交接文档](09r2f1-set-operations-continuation.md) | 交接前七个问题的修复、RuntimeCheck 接线、三机型共享向量、运行时区分度与文档同步 | 已完成（前置阶段 59 条共享向量，四类集合检查接通；成员类型载荷由 R2G 收口，集合增删仍为后续债项） |
| 09R2g | [`for` 与迭代执行闭环交接文档](09r2g-for-and-iteration.md) | `Len`/`IndexGetDynamic` 两条指令、`for` 的 TAC 降低与 CFG、`iterable` 运行时检查、`iteration.json` 共享向量 | 已完成（opcode 36/37、三种载体、动态错误码 `X06-RUNTIME-024` 与释放边界均已验证） |
| 09R2h | [表声明执行闭环交接记录](09r2h-table-declarations.md) | 表签名与方法索引、构造与字段读写、初始化回滚和确定性析构 | 已完成（新增表定义段、opcode 38–40、布局版本 3；79 条共享向量；须以新布局进入 R3） |
| 09R3 | [跨平台基准与冻结](09r3-benchmarks-and-freeze.md)（权威定义见 [09R](09r-bytecode-machine-research.md) `:610-619`） | 语义差分、性能、内存与编码体积四份报告，以及机型/ABI/编码的冻结 | 已完成 Windows 原生与 Linux amd64/arm64、macOS arm64 CI 功能复现；性能冻结仍以 Windows 原生为准，Docker arm64 本机仿真与 macOS 真实终端不计入性能验收 |
| 09-B0 | [字节码最小运行闭环](09b0-bytecode-closure.md)（权威定义见 [12](12-tests-and-milestones.md) `:672-679` 与 [09R](09r-bytecode-machine-research.md) `:640-649`） | 生产字节码模型与验证器（B0-A）、生产 VM 执行闭环（B0-B）、前端到 VM 的内部驱动器（B0-C）、退出码冻结（B0-D） | 已完成（B0-A/B/C/D 均已落地；VM 中途取消检查点转入 [09-B0-E](09b0e-vm-cancellation-checkpoint.md)） |
| 09-B0-B | [生产 VM 执行闭环](09b0b-production-vm.md) | 生产入口 ABI（脚本/`[main]`）、规范化运行参数对象、结构化退出结果、栈回溯与事件接收器生产化 | 已完成（生产入口、前置验证、报告接线和有界事件接收器已落地） |
| 09-B0-C | [前端到 VM 内部驱动器](09b0c-frontend-to-vm-driver.md) | `xiao-driver` 的运行驱动器、三段错误的统一结构化表示、取消/超时边界 | 已完成（内部驱动器、公共契约测试和 UseDocs 已落地；`B0-C-CANCEL-001` 转入 [09-B0-E](09b0e-vm-cancellation-checkpoint.md)） |
| 09-B0-D | [退出码冻结与 Linux 容器实测](09b0d-exit-codes-and-linux-verification.md) | `ExitCode` 的语义与取值冻结、`DriverOutcome` 上的稳定派生、locale 中立性断言、容器实测记录 | 已完成（退出码契约与测试已落地；Linux 容器结果见交接文档；不改变 Windows 原生冻结口径） |
| 09-B0-E | [VM 中途取消检查点](09b0e-vm-cancellation-checkpoint.md) | `B0-C-CANCEL-001` 的出口：VM 热循环内的中途检查点、可注入取消源、清理/退出码回归、**单独记录**的性能对照 | 已完成（`Fault::Cancelled` 独立通道、`run_blocks`/`run_subroutine` 检查点、CLI `AbortSignal`、退出码 2 回归；别名层清理仍拆出；**三处收尾见 §九**） |
| 10 | [LLVM 原生后端](10-native-backend.md) | `xiao build` 的 LLVM 原生二进制（Windows → Linux → macOS） | **N0-A/B/C/D 全部交付**（N0-C 见 10E–10J，N0-D 见 10I/10K）；动态入口对齐实现见 10L（扩大范围首轮接线，溢出与高级选择结果待实测）；用户可见 `xiao build` 属 X0（11 阶段）；仅余 PE 内部节裁剪证据债（标 `unverified-coff-exports`） |
| 10E | [N0-C 错误路径与源码映射](10e-n0c-error-paths-and-mapping.md) | 统一错误路径降低到原生 ABI、`try`/`catch`/`finally` 展开、源码映射与诊断事件、`-debug` 独立诊断窗口、**字节码差分** | 已交付（**验收主体是差分**；收口批为 10F–10K，N0-D 见 10I/10K） |
| 10F | [N0-C 专项审核：`try`/`catch`/`finally` 的 LLVM 控制流](10f-n0c-audit-try-finally.md) | 复核 `31fb944` 对 `emit_try_cleanup` 的「结构性控制流问题」判定；限定 `try`/`catch`/`finally` 的原生发射与 `llvm-as` 验收 | 已复核（§七：`91a3392` 修复 16 种形态通过真实 `llvm-as`） |
| 10G | [N0-C-2 错误边界与字节码差分](10g-n0c2-error-boundary-and-differential.md) | N0-C 第 5、7 步 + §3.1：语言上下文接入、平台异常捕获与报告（三平台）、字节码差分（输出/错误/释放记录三样逐项对照） | 已交付（`d95b6ae`；Windows 门控全绿已实测；**跨平台与栈用量实测为欠账**，见 10H §二） |
| 10H | [N0-C-3 `-debug` 产物、诊断事件与 N0-C-2 欠账清算](10h-n0c3-debug-window-and-debt.md) | N0-C **收尾批**：先清 5 项欠账（CI 自动门控、栈用量实测、`find_renderer` 产物陷阱、登记对齐、行号更正），再做 `-debug` 产物启动入口（不依赖启动器/不受 GUI 抑制/失败不静默）、诊断事件转交、普通产物反向测试 | 已交付（`1edf6ec`；五项欠账全清、门控 22 项全绿已实测；**CI 首跑未发生**，见 10I §二） |
| 10I | [N0-D 产物验证与裁剪](10i-n0d-artifact-verification-and-trimming.md) | 10 阶段**收尾批**：三目标（Coff/Elf/MachO）固定宽度与错误行为验证、**产物层** Runtime 裁剪验证（含反例）、Runtime 组成可解释、调试标志产物级验证、`-O0` 基线 | 已交付（`1d66f9c`+`a5a5e6d`）；**CI `36841654803` 暴露跨平台缺口，收口见 10K** |
| 10J | [跨操作系统验证专项（Linux 侧交接）](10j-cross-os-verification.md) | CI 首跑（`36799534374`）四平台全红的收口：`crash.rs` 的 `#[cfg(unix)]` 分支从未在 Unix 上编译过（`unsafe_op_in_unsafe_fn` 等 lint 在 `-D warnings` 下变 error）；在 Linux 复现、修复、验证，macOS 靠 CI | 已交付（`6d55486`；CI `36805985940` 四平台全绿；macOS 无额外 libc 差异） |
| 10K | [Runtime 裁剪验证的跨平台收口（Linux 侧交接）](10k-n0d-runtime-trimming-verification.md) | N0-D 在 CI 上只有 Windows 通过：ELF 产物观察到 `containers/tables/weak`（链接器没裁），而 **Windows 的绿是「自己导出、自己检查」的自证**——`10:78` 在 PE 上从未被验证过 | 已完成（`6177931` 修复 ELF/Mach-O 裁剪边界并明确 COFF 未验证能力；CI `36850150346` 四平台全绿；后续仅保留 PE 内部节证据债） |
| 10L | [原生动态入口与 VM 对齐（N0-E）](10l-native-dynamic-alignment.md) | 先修 19D 审核的 G1–G3，再枚举原生动态入口拒绝的语句与运行时检查种类，按确认范围逐类对齐 VM；整数溢出单独决定 | 扩大范围首轮接线（2026-10-06）；3 类语句与 14 类检查已接入；**2026-10-07 审核实测发现原生构建回归（四个基准程序被 clang 拒绝），并入 10M** |
| 10M | [原生动态入口回归修复与验证闭环（N0-F）](10m-native-regression-and-verification.md) | 修 H1–H8：`%abi.fail` 未定义标签导致的基准程序构建回归、函数体动态差分、探测基线、LLVM 结构校验、Runtime ABI 拆分与文档回填 | 已完成（2026-10-07）；Windows 受控原生差分与 19D 探测通过，Linux/macOS 与性能基线待补；审核实测发现原生范围/随机选择静默返回源值，并入 10N |
| 10N | [原生选择器正确性与剩余覆盖（N0-G）](10n-native-selector-correctness.md) | 修复范围/随机选择静默错值，补齐选择器差分，枚举动态剩余拒绝面并对账 19 O6 | 选择器用例已交付，I6 当批未完成（现由10R修复）（2026-10-07）；动态表方法、CI 原生门控与跨平台证据待后续；审核发现豁免会吞掉值差异、选择器多一次释放、O6 对账条目不对应，并入 10P |
| 10P | [差分豁免收敛与剩余拒绝面（N0-H）](10p-differential-exemption-and-coverage.md) | 先让缺口豁免只豁免声明的字段（否则新用例等于没有守门），再判定选择器多出的那次释放、修嵌套 `finally` 的少释放，按规范五条重做 19 O6 对账，并枚举剩余拒绝面 | 实现中（2026-10-07）；J1/J2/J3 已提交，I6 未做；审核发现两条选择器用例无分辨力、失败信息丢证据、释放差异未核对，并入 10Q |
| 10Q | [用例区分度与释放账目核对（N0-I）](10q-selector-case-strength-and-release-audit.md) | 聚合断言、完整失败证据和释放对照取证 | K2/K3/K4 已实施；已推 c4b3cb3，维护回归 37604955900 绿（不含原生）；[I6](10q-i6-cleanup-followup.md) 已由 10R 修复 |
| 10Q-I6 | [函数清理链重构](10q-i6-cleanup-followup.md) | 函数返回、嵌套 finally 与作用域释放顺序 | 10R 的 7ca6bdf 已实现，Windows 严格差分通过，豁免摘除 |
| 10R | [释放账目收口与原生 CI 门控（N0-J）](10r-release-accounting-and-native-ci-gate.md) | Windows 原生门控、I6 清理链、扩展释放账目、静态溢出立项与 ABI 顺序 | I6 已修；最终CI 37614663496 绿、变异37608705803红，已撤回恢复绿；扩展账目已记录；表用户 drop 原生受方法 ABI 拒绝；审核确认门控与 I6 有效，余四处 Drops 差异与跨平台证据并入 10S |
| 10S | [跨平台证据刷新与释放账目收口（N0-K）](10s-cross-platform-evidence-and-release-closeout.md) | 用星崽的图形化 Linux 裸机补第三平台证据、手动刷新 `platform-reproduction`（macOS 唯一来源）、逐调用收口四处释放差异、Linux 桌面开窗取证、A1 表方法 ABI 只做准备 | 实施中（2026-10-07）；四处释放差异已逐调用定位并保留仅 Drops 豁免；A1 清单完成；四平台 CI 37642937815 通过（macOS 跳过真实终端且调试窗口为稳定失败路径），裸机新版与桌面开窗证据待补 |
| 10S-Linux | [裸机 Linux 证据采集交接](10s-linux-bare-metal-handoff.md) | 写给 Ubuntu 机器操作者：前置包与工具链版本、`reproduce.sh native` 的跑法与「不设 XIAO_USE_XVFB」、环境采集清单、诊断窗口行为证据的判据、回传模板 | 已收 PR #3 原始失败回传（该 PR 已关闭，内容经 `4fab901` 原样并入）；**本档为上一轮，勿再按它执行**，新一轮见 10T-Linux |
| 10Z-驱动 | [性能对照的计时驱动器（O6 条件 5 的前置）](10z-performance-driver.md) | Z-2 原要「C 档 + 取数」一起做，但实测**取数侧没有工具**：Java 与 LLVM 原生两侧都缺计时驱动器，`baseline.json` 冻结的协议（预热 3/测量 11/percentile-bootstrap/10000/seed 19015）只被声明未被实现。本批做那个驱动器：先三侧语义互校再计时、口径写死（整进程挂钟、构建不计入、不剔离群）、bootstrap 用已有的 `SeededRandom` 保证确定性、只填 `baseline.json` 槽位不另立格式；开发在本机、取数在受控主机 | 已完成；受控 Linux 原始取数见 [10Z 结果](10z-performance-measurement-results-20261011.md)，阈值冻结与判定待后续 |
| 10Z-取数 | [受控主机性能取数交接（O6 条件 5 的 Linux 格）](10z-performance-measurement-handoff.md) | 写给受控主机操作者：驱动器已就绪，本轮跑三路对照取数。**与 C 档不是同一轮**（原「同一轮」前提不成立，已拆开）。给出驱动器**实际读取**的环境（`java`/`javac` 21 + `XIAO_CLANG`/`XIAO_RUNTIME_LIBRARY`/`XIAO_TARGET_TRIPLE`，不读 `XIAO_LLVM_AS/LLC/STRIP`）、跑法、报告判读（含「`measured_cases == 0` 却是 `development-evidence` 就是缺陷」）、三样回传物与回传模板；并写明**取数完立即冻结阈值**、这一步直接决定 19 能否收口 | 原始取数已完成，阈值已冻结；O6 条件 5 仍受 Windows 标缺与 macOS 不可验证限制 |
| 10Z-Linux | [Z-2 裸机轮次交接：`19.14` C 档窗口取证](10z-linux-bare-metal-handoff.md) | 写给 Ubuntu 机器操作者：`19.14` C 档的窗口截图（含**不要设 `XIAO_DIAGNOSTICS_HOLD_MS=0`**——`reproduce.sh:121` 会设它，是前两轮失败的成因）、窗口相关环境采集、双文件回传模板。**并写明本轮不做性能取数**：实测 Java 侧与 LLVM 原生侧**都没有计时驱动器**，`baseline.json` 的协议只被声明未实现 | C 档已完成（PR #5）；性能取数已拆至 [10Z-取数](10z-performance-measurement-handoff.md) 并完成原始测量 |
| 10T-Linux | [裸机 Linux 复跑与诊断窗口定位交接](10t-linux-bare-metal-handoff.md) | 写给 Ubuntu 机器操作者：修复后提交 `0735846` 的复跑、窗口相关环境采集（`x-terminal-emulator` 指向与各终端模拟器）、`pgrep` 观察与换 `xterm` 的对照、回传模板 | 待执行（2026-10-08） |
| 10T | [表方法 ABI 实现（A1 / N0-L）](10t-table-method-abi-implementation.md) | 按 10S-A1 清单实现新描述符与调用入口、Runtime 侧初始化/`drop`、codegen 方法函数表，最后移除方法拒绝点；`table-user-drop` 转真执行、`container-dense` 可构建可比对 | A1 实际接通（2026-10-08）；ABI 1.8、代码生成 4、构建探针 5/5、容器 80 轮与 VM 同为 28760；Windows 完整矩阵通过，裸机 Linux 复跑退出码 0（PR #4 已合并），窗口仍无截图故 19.14 C 档未通过；星崽已接受口径差异并授权 Linux push 门控。证据回填与窗口取证程序修正并入 10U |
| 10U | [表构造参数与函数值 ABI（A2 / A3 / N0-M）](10u-table-construction-and-function-value-abi.md) | 回填裸机证据、修正真实 `-debug` 窗口取证并接通 A2 初始化参数；A3 仍先冻结能力边界 | 实施中（2026-10-08）；A2 V2 init 参数入口与 LLVM 构造路径已接通，10W 已补 VM 基线，A3 已完成边界取证；Linux 窗口 C 档已通过（2026-10-10，PR #5 合并 `efa9fd2`） |
| 10V | [诊断窗口会话结束后的保持（N0-N）](10v-diagnostic-window-hold.md) | 会话结束保留最后一屏、提供有界提示、测试与 CI 显式关闭保持 | 已实现（2026-10-08）；`XIAO_DIAGNOSTICS_HOLD_MS` 控制 0–3600000 ms，连接与 standalone 两入口覆盖；审核实测默认 6170 ms、`=0` 时 1154 ms；本地跑法的说明已补进 10D §4.5 |
| 10W | [A2 收口与 A3 边界冻结（N0-O）](10w-a2-closeout-and-a3-boundary.md) | 把 A2 从「能降低」推到有差分与回滚证据（求值顺序、默认值、缺参/多参、类型不符、构造失败回滚）；A2 剩余形状与 A3 只做 VM 实跑取证并三种分类改判，判为「两边都拒绝」就不改后端 | 已实施（2026-10-08）；5 条构造参数用例，原生侧经审核实跑并通过、推送后 CI 37756208604 的 Windows/Linux 原生作业均绿；A3 改判为两边一致拒绝 |
| 10X | [B 系列取证与分类（N0-P）](10x-b-series-triage.md) | 先把剩余 23 处拒绝逐条 VM 实跑取证并按三种分类判定（含可达性），再按结论定 B1/B2 的实现范围；同时补上一批欠的变异验证、替代理由、O6 运行号与 W2 确认状态 | 实施中（2026-10-08）；X2/X4 已收敛、**X3 已完成**（回填 `37756208604`，审核核实属实）；X1 的伪验证已删除测试、真实原生变异验证转 10Y；**B 系列净新增分类仍为 0**，主体与 10P 回写并入 10Y |
| 10Y | [返工与 B 系列分类收口（N0-Q）](10y-b-series-triage-rework.md) | 先真正重做 X1 变异验证（必须是原生变异 + 原生门控跑红）与 X3 回填（19D/10P 两处仍写「待补」），恢复 10P 被降级的判定并补可达性列，再完成剩余 23 处拒绝面的逐条实跑分类；另含 README 唯一性门禁、macOS 标「不可验证」与 10 系列内的受控性能取数 | Y3/Y4/Y5/Y6 的裁定与回填已完成；Y6 Linux 原始取数已归档，阈值与终局收口见 10Z |
| 10Z | [19 收口：B 系列实现与收尾取证（N0-R）](10z-19-closeout-and-b-series-implementation.md) | 入口是 10Y 的出口条件；按 Y3 分类结论兑现 B 系列（有缺口按 B1→B2 实现、无缺口如实关闭），一次裸机轮次**同时**补 `19.14` 的 Linux 桌面开窗 C 档与 O6 条件 5 的取数，把条件 1/3/4 的「部分」逐格拆开补，最后按事实对 O6 五条做终局判定并在收口声明里写明覆盖边界 | 入口审计更新（2026-10-10）；B1 已修复并回填，C 类关闭，D 转 10Z-D；Linux C 档与 Y6 原始取数已完成，阈值冻结与 19 终局收口待后续 |
| 10Z-CI覆盖 | [push 时的 CI 覆盖缺口](10z-ci-push-coverage-followup.md) | push 时让 `tools/`、`cli/`、`docs/` 和配置变化获得 Bun/Rust 轻量门禁，并把命令收敛成 `tools/gates/run.sh` 单一来源 | 已实施（2026-10-09）；Linux `workspace-gates.yml` 已接入，本批不加入全量 workspace 测试；该测试已由 10Z-CI覆盖2 的维护回归作业补上；干净运行 `37869869134`、负例 `37871964485`、恢复运行 `37879955296` 均已回填 |
| 10Z-收口 | [19 收口：O6 终局对账与 10 系列关闭](10z-19-closeout-declaration.md) | `10Z-收尾` 的 Z-1/Z-2/Z-3 已完成；本批已把 T1–T4、B1 状态和 O6 五条终局对账写入独立声明，保留四条「部分」+ 一条「不满足」及覆盖边界。10 系列只有在关闭条件全部满足后才关闭，不能把声明落盘写成已关闭 | **10 系列已关闭（2026-10-12，星崽裁定）**；七项关闭条件逐条满足；**关闭不等于 O6 达标**（四条部分+一条不满足原样保留）；T4 受控复跑、B1 成对实验与各 O6 欠格的**卡点与归属**见 [00. 决策基线](00-decisions.md)「10 系列关闭与遗留项」 |
| 10Z-收口审核 | [收口实施审核：三处返工与两处登记](10z-closeout-implementation-audit.md) | 审 `b55f59f`/`33b1ed0`/`b3eddb1` 三个提交的 T1–T4 与状态对账。**数值全对**（四个比值、四个保守区间、两个 sha256 审核逐位重算通过），但三件事没落地：F-1 19D 状态块自相矛盾；F-2 阈值依据**引错政策**（09R 的 10% 是「机型 vs 栈式基线、要求快 10%」）且达标线被从 1.0 放宽到 1.10，而 `baseline.json` 不自描述；F-3 D7 的执行点**触发不到** `tests/benchmarks/**`（`workspace-gates.yml` 的 paths 不含它，含它的维护回归只跑 core/rust）——**这一处是规划侧规范自己的洞**，且 D7 的验收证据恰好来自一个把缺口遮住的推送。另有两处口径登记 | **F-1/F-2/F-4/F-5 已核销**（`24c0a5c`，判据按审核倾向取 **≤1.0**，sha256 同步为 `6017605e…`）；F-3 执行点已在 CI 跑通（`38117244885`），**触发正例与负例待补**；本文的实施记录翻新见 [20C](20c-official-xiao-library.md) §2.3 |
| 10Z-收尾 | [10 系列收束：B 系列分类、取证与 19 收口](10z-closeout-execution.md) | 主线被工具侧吃掉三轮：`10Y` 的主体 Y3（B 系列逐条分类）已完成，`10Z` 按裁定收口 B/C/D 范围；本批把两批的未完成项收束成一条序列（Y1 → Y3 → 汇报 → B 实现/关闭 → 裸机取证 → 欠格 → O6 收口），**冻结「不再开新的工具侧批次」** | **已完成（2026-10-12）**；Z-4 终局对账已落盘，§2.5 七项关闭条件逐条满足，10 系列已关闭；遗留项写进决策基线 |
| 10Z-Z1 | [Y3 收口结论与 Z-1 范围](10z-z1-scope-and-gap-verdict.md) | 四组对照的判定与三项裁定：B 错误身份差分、C 两边都拒绝、D 合法程序的原生缺口；B1 已完成，D 转独立立项 | 已裁定（B1 已修；D 不纳入 Z-1） |
| 10Z-D | [动态错误构造参数实现记录](10z-dynamic-error-constructor-followup.md) | 动态 `ArithmeticError`/可恢复错误构造参数的 Runtime ABI、LLVM 发射、诊断位置与三路差分验收 | 已实现（ABI 1.12；2026-10-13 Windows workspace ignored 原生门禁通过，动态参数错误身份、位置和释放轨迹一致）；Linux/macOS 复跑与变异红色证据待后续 |
| 10Z-Y3 | [Y3 阶段汇报与收尾对照](10z-y3-stage-report.md) | Y3 四组对照已完成，正式选择计划测试已提交；B/C/D 裁定与 B1 复核见 10Z-Z1，原始日志保留前后证据 | 已完成（B1 已修；C 关闭；D 转 10Z-D） |
| 10Z-CI覆盖2 | [crate 过滤缺口与全量测试无覆盖](10z-ci-crate-and-test-coverage.md) | 上一轮只补了 `tools/`/`cli/`/`docs/`。再查一层：路径过滤**枚举**了 23 个 workspace 成员里的 13 个，其余 10 个 crate（含 `xiao-diagnostics`、`xiao-i18n`）push 后两个工作流都不触发；且 CI 里**没有任何地方跑全量默认测试**（唯一的 `--workspace` 带 `-- --ignored`，只跑被忽略用例）。本批把枚举换成 `core/rust/**` 并让全量默认测试有自动触发点 | 已实施（2026-10-09）；P1 Linux workspace 默认测试作业已接入，glob 触发、全量测试负例和恢复均有真实运行证据（`37902915111`、`37904957067`、`37905880682`） |
| 10Z-唯一性 | [DevDocs 主索引唯一性门禁收口（A0-DOCS-004）](10z-index-uniqueness-followup.md) | 按 B 口径收口：链接字符串（含锚点）唯一、不同锚点允许、每行只取首链接；补齐锚点/第二链接边界测试和 `A0-DOCS-003` 规则登记 | 已实施（2026-10-09）；`bun run check:docs` 通过，`tsc` 仍不并入 `bun run check` |
| 10S-A1 | [表方法 ABI 开工清单](10s-a1-table-method-abi-preparation.md) | 描述符、函数表、所有权与 drop 时机、版本边界、影响 crate 和验收用例 | 准备清单已完成；本批不实现 |
| 10R-溢出后续 | [静态溢出统一错误路径](10r-static-overflow-followup.md) | 七处失败模板、动态错误参数、语言错误与平台 Fatal 分界及验收 | 静态数值与动态参数路径已实现（ABI 1.12、Windows 边界已验证）；受控原生差分、静态清理链与跨平台证据待后续 |
| 10A | [LLVM 原生构建闭环](10a-n0-native-closure.md) | 手写 IR 文本 + 外部工具链、`xiao-runtime-abi`、四批交付（N0-A 纯静态 → N0-D 验证裁剪） | N0-A 已完成；N0-B Runtime ABI 已接续落地（Windows 原生、Linux amd64/arm64 与 macOS arm64 CI 功能复现；WSL/容器仅作功能证据） |
| 10B | [N0-B Runtime ABI](10b-n0-runtime-abi.md) | 动态值的 ABI 表示、真实引用计数与 `Weak`、容器与表 ABI、正常路径的释放计划 | 已落地（ABI/容器/表/正常释放计划；异常展开留 N0-C） |
| 10C | [原生 Runtime 链接缺陷修复交接](10c-native-runtime-link-fix.md) | `LNK1120` 的完整证据、Rust staticlib 原生库查询、MSVC ABI 调用约定修复与实际链接结果 | 已完成（Windows 原生、Linux amd64/arm64 与 macOS arm64 CI 动态 Runtime 功能闭环通过；WSL/容器仅作功能证据） |
| 10D | [环境依赖测试专项规范](10d-environment-gated-test-spec.md) | `#[ignore]` 取代静默 `return`、齐备环境的准备方式与三个坑、门禁的 `ignored` 计数要求 | 已完成（8 条门控测试默认可见；Linux amd64/arm64、Windows 各 8 条全绿，macOS 7 条执行且真实终端显式跳过） |
| 11 | [CLI、项目配置与平台](11-cli-config-and-platform.md) | TypeScript CLI、运行时配置、`-debug` 诊断入口和目标平台适配 | 进行中（X0-B/C/D/E/T 与 X0-SPEC 已落地；四平台 CI 功能证据通过；**仅余 X0 第 8 条的 macOS 真实终端待环境**——Linux 侧已在 WSL 验证，macOS CI runner 无 GUI 会话） |
| 11X0 | [跨平台工具链：协议与 CLI](11x0-cli-protocol-and-toolchain.md) | 进程协议 + 长度前缀 JSON、协议单一来源、X0-A/B/C/D/E/T 交付 | X0-A/B/C/D/E/T 已落地（Windows、Linux amd64/arm64、macOS arm64 CI 功能证据；macOS 真实终端显式跳过；`print` 不在本阶段） |
| 11X0-B | [TypeScript CLI 骨架](11x0b-cli-shell.md) | `xiao` 命令入口、命令解析与帮助、`xiao run` / `xiao config`、呈现层（颜色四层降级、**中文宽度**、非 TTY） | 已完成（Windows 原生回环；项目测试语义与接线由 X0-T 完成；`print` 仍后置） |
| 11X0-C | [独立可执行与平台矩阵](11x0c-packaging-and-platforms.md) | `bun build --compile` 独立可执行、`xiao-core` 同目录分发与生产发现契约、验证环境矩阵（验证 ≠ 验收） | 已落地（Windows、Linux amd64/arm64、macOS arm64 CI 的独立产物与发现回环通过；WSL/容器仅作功能证据；构建接续 X0-E） |
| 11X0-D | [`-debug` 与诊断窗口](11x0d-debug-diagnostics-window.md) | 诊断激活位、独立诊断进程与事件管道、平台终端启动与 TUI、启动失败与运行中断的区分 | 已完成（Windows、Linux amd64/arm64 CI 与 WSL/容器功能证据通过；macOS 构建回环通过，但真实终端显式跳过） |
| 11X0-E | [`xiao build` 与主机工具链发现](11x0e-build-and-toolchain.md) | `build` 命令路由、主机工具链发现、原生启动 shim、运行时配置固化；含跨批次待完善清单 | 已落地（Windows、Linux amd64/arm64 与 macOS arm64 CI build/run/debug 回环通过；macOS 真实终端保持未验证） |
| 11X0-F | [`protocol.rs` 解耦交接](11x0f-protocol-decoupling.md) | 2354 行（94%）预防性拆分：门面 + 九个子模块、依赖 DAG 与架构测试、七步分提交 | 已完成（门面 59 行；九模块与架构回归测试已落地；公开 API、协议夹具和既有测试未改） |
| 11X0-G | [`checker.rs` 解耦交接](11x0g-type-checker-decoupling.md) | 2200 行（88%）预防性拆分：`RuntimeCheckKind` 跨 crate 路径不变、常量求值作最安全起点、六步分提交 | 已完成（门面 166 行；六个职责子模块、架构回归测试和模块文档已落地；公开 API、类型规则与 `xiao-types/tests/` 未改） |
| 11X0-H | [`dynamic.rs` 解耦交接](11x0h-llvm-dynamic-decoupling.md) | 2172 行（87%）预防性拆分：逐字节不变为最强验收、静态/动态边界、**发现 escape_llvm/stable_hash 重复实现** | 已完成（门面 248 行；九个动态职责子模块、架构回归测试和模块文档已落地；Runtime ABI、静态边界与 LLVM 文本未变） |
| 11X0-T | [项目测试语义与结果协议](11x0t-project-test-semantics.md) | `xiao test` 的语义裁定、测试文件发现、确定性执行顺序、隔离/超时边界、机器可读结果协议 | 已完成（项目测试发现、协议夹具、Rust/TypeScript 接线、逐用例结果与 UseDocs 已落地；跨平台原生复现仍按清单记录） |
| 11X0-P | [跨平台复现（Linux 原生 / WSL / macOS）](11x0-platform-reproduction.md) | 清 X0 积压的平台债：容器多架构（含 arm64 架构缺陷）、WSL 反例探测、CI macOS runner；补齐容器工具链与复现脚本 | 已完成功能证据（Windows、Linux amd64/arm64、macOS arm64 CI 与 Ubuntu/Arch WSL 均通过；本机 Docker arm64 仿真仍受阻，macOS 真实终端未验证） |
| 11X0-P1 | [跨平台复现收口](11x0p1-platform-reproduction-closure.md) | 收掉 P 批留下的三项：修 `reproduce.ps1` 的 PowerShell 5.1 缺陷、推分支触发 CI（原生 arm64 绕开 QEMU + macOS runner）、WSL 装工具链；X0 第 2/4/5/6 条明确收口 | 已完成（CI 运行 `35955788547` 四平台成功；X0 第 2/4/5/6 条收口；macOS 真实终端与 X0 第 8 条单项保持未验证） |
| 11X0-SPEC | [`X0-SPEC-001` 规格测试债](11x0spec-project-rule-spec-tests.md) | 为阶段 07/08/10/11 补规格测试**目录与执行入口**；判据是「有加载者」而非「目录存在」；含四阶段的可行性分级与既有登记偏差更正 | 已完成（06 接线、07/08/11 规格入口、登记门禁；10 论证后不重复建夹具；**`tests/unit`/`tests/integration` 明确 out of scope**） |
| 11A.1 | [包源协议：决策、待决与风险（待审）](11a1-package-source-protocol-review.md) | 源身份规范化、JSON 权威与 Protobuf 派生、解析键与同一性、canonical JSON、源列表导入的三条规则；含六条待审风险 | 已审并处置（方向保留；源引用语义、导入顺序、摘要信任边界三处已修正） |
| 11A-D1 | [外部包契约与依赖图骨架](11ad1-package-contract-and-graph.md) | `config.xiao` 声明外部依赖、依赖解析器与内存依赖图、包粒度诊断；**不含缓存/锁文件/远程** | 已完成（本地路径包契约、图解析和规格门禁已收口） |
| 11A-E0 | [配置与环境指纹](11ae0-environment-fingerprint-and-activation.md) | 配置/工具链/目标指纹、`xiao venv` 目录规则、Shell 钩子激活与绿色前缀；**不含缓存/锁文件/网络** | 已完成（5 条输入已冻结；Bash/PowerShell 钩子 + `cmd` 降级已接；`fnv1a64` 已并入 `stable_hash`；真实父终端测试按冻结留给 E3D） |
| 11A-E1 | [本地依赖与共享缓存](11ae1-local-dependencies-and-cache.md) | 全局内容寻址缓存、`SHA-256` 内容摘要、环境到缓存条目的只读逻辑映射、损坏条目隔离；**不含锁文件/sync/install/CLI** | 已完成（源码对象、v2 映射、项目/全局环境复用、损坏隔离、规格夹具和 UseDocs 已通过） |
| 11A-E2A | [锁文件与环境映射](11ae2a-lockfile-and-mapping.md) | `xiao.lock.json` 的生成/校验/复用、环境映射原子更新、配置与锁文件不一致诊断；**不含 sync/install/CLI** | 已完成（完整本地图锁文件、三类过期诊断、跨平台原子覆盖、双份规格夹具、Clippy 门禁和 UseDocs） |
| 11A-E2B | [同步与安装命令](11ae2b-sync-and-install.md) | `sync` 的创建/补齐/激活流水线、`install`/`i` 的目标选择、激活通道与 11B 只读视图；**本批首次新增用户可见命令** | 已完成（激活通道落地为临时文件，实现**不 source** 而是提取路径直接赋值，代码执行面被消除；E2A 三处欠账已收敛） |
| 11A-E3A | [包源契约与多源配置](11ae3a-package-source-contract.md) | 静态源声明、离线目录适配器、联邦记录、跨源选择与 JCS 向量；**契约批次，不联网** | 已实现并测试；远程传输/缓存留 E3B/E3C，写回/源内求解留 E3D |
| 11A-CONC | [并发模型：工具链内部并发与语言级并发的边界](11a-concurrency-model.md) | 划清工具链内部并发（E3B 前置，**冻结**）与语言级并发（归 `07-D`，**不冻结**）；冻结原语选型、有界并行、结果确定性、失败语义、跨进程锁 | 已完成设计冻结并供 E3B 消费；语言级并发未冻结 |
| 11A-E3B | [快速解析与远程缓存](11ae3b-fast-resolution-and-cache.md) | 四类缓存分离（内容寻址 vs 源身份覆盖）、快速路径三类判据、三态源状态、有界并行与跨进程锁；**仍不联网，本地源驱动** | 已完成 |
| 11A-E3C | [静态源与 GitHub 适配器](11ae3c-static-and-github-sources.md) | 静态 HTTP 源、GitHub 稀疏索引、单仓库 Git 声明语法、不可变提交与 Range；**第一批联网** | 已实现核心适配器及本机网络规格；远程直接依赖的求解/安装归 E3D |
| 11A-E3D | [包操作、安全与发布](11ae3d-package-operations.md) | 完整 semver 约束与求解、`add`/`remove`/`lock`/`update`、`config.xiao` 保留注释的写回、来源审计与凭据边界；**签名不做** | 已完成 |
| 11A-E3D1 | [首个远程闭环](11ae3d1-remote-closure.md) | 求解器接到 `sync` 真实流程、远程正文下载与锁文件摘要校验、凭据边界（0600）、来源审计与信任失败诊断 | 已完成 |
| 11A-E4 | [环境交付](11ae4-shell-delivery.md) | bash/zsh/fish/PowerShell 四 Shell 矩阵、`fish_prompt` 保存恢复、`--install` 标记块安装（备份/幂等/可移除）、生产级取消激活；工具链安装仅评估 | 已完成；实际工具链安装未实现 |
| 11A | [虚拟环境与包管理](11a-environments-and-packages.md) | 项目隔离、多源包仓库、联邦源索引、共享缓存、锁定和管理命令 | **已完成**（D1、E0、E1、E2A、E2B、E3A、E3B、E3C、E3D、E3D1、E4 全部交付；条件候选兼容性上下文登记为 `E3D1-COMPAT-001`） |
| 11B-I0 | [终端骨架与单行会话](11bi0-terminal-skeleton.md) | 无参数启动交互会话、版权/版本/路径/环境段/`[X>` 提示符、Git 摘要（含无上游与超时降级） | **已完成**（首次探测阻塞已由[修复批次](11bi0-fix-git-probe-blocking.md)收口）；多行与包加载仍归 I1/I4 |
| 11B-I0 修复 | [Git 探测阻塞首个提示符](11bi0-fix-git-probe-blocking.md) | `execFile` → `spawn`（**可真正杀掉超时子进程**、stdout/stderr 各限 64 KiB、显式 `shell: false`）、冷进程耗时测试、`--inLF` 明确拒绝、I0 文档更正 | 已完成（目标环境 101 通过 / 5 跳过 / 0 失败；首屏 0.343s 带计数） |
| 11B-I1a | [多行缓冲区与编辑核心](11bi1a-multiline-buffer.md) | 多行缓冲区与五字符行号栏、软换行（含**双宽字符**）、多行粘贴、**完整编辑按键协议**、控制指令共享分派、`!inLF!`/`--inLF` 入口 | 已完成（134 通过 / 5 跳过 / 0 失败）；确认执行由 I1b 接续 |
| 11B-I1c | [光标模式、选区与剪贴板](11bi1c-cursor-selection-clipboard.md) | `Ins` 覆盖模式、锚点+光标选区、鼠标点击定位（SGR，**Shift 保留终端本地选中**）、程序内剪贴板 `Ctrl+Shift+C/X/A`、修 I1a 的 Alt 过度降级 | 已完成（144 通过 / 5 跳过 / 0 失败） |
| 11B-I1b | [运行确认与执行输出](11bi1b-confirm-and-run.md) | 确认态与自适应分隔线、Esc 无损取消、执行完整缓冲区、追加式输出、耗时/Runtime 峰值内存摘要、终端尺寸变化 | 已完成（CLI 163 通过 / Rust 全绿）；语言级交互输入由星崽确认留给独立 VM I/O 通道 |
| 11B-I2 | [文件编辑与保存](11bi2-file-save.md) | `--inLF <file.xiao>` 载入与绑定、保存态路径输入、`q`/Esc 无损取消、首次写回与已绑定直接写回、**原子写抽取共用**、路径/权限/编码/换行四类诊断 | 已完成（CLI 190 通过 / Rust 无回归；10 个翻车点全部守住） |
| 11B-I3 | [空命令面板](11bi3-command-panel.md) | 面板模态（单行与多行**共用**一份实现）、`Command Panel` 标题与自适应分隔线、Esc/快捷键开关、`!panel!` 剔除、空面板不虚构命令 | 已完成（CLI 210 通过 / Rust 无回归；8 个翻车点全部守住） |
| 11B-I4a | [环境包视图与延迟加载协议](11bi4a-package-view-protocol.md) | 环境包视图协议请求（`repl_packages`）、环境隔离、根名冲突诊断、只读接口元数据查询（**带不执行探针**）、旧核心降级、协议向量与 UseDoc | 已完成（6/9 条交付并核实；剩余三条由 I4a2 收口）。**搁置时的诚实标注已随门禁转绿一并解除** |
| 11B-I4a2 | [包模块的加载路径](11bi4a2-package-module-loading.md) | 外部包命名空间绑定、VM 的模块注册表与导出值载体、首次引用时的加载与编译、已初始化去重、失败诊断与重试 | 已完成（`module-registry` 转 `verified`）。**§3.1 查明证伪了「丢弃 `import` 可能是静态链接设计」的假设**；跨 `run` 的会话级语义归 I4b |
| 11B-I4b | [会话生命周期与 TS 接入](11bi4b-session-lifetime.md) | **核心进程从无状态变有状态**（`run` 复用 VM 与模块表）、环境指纹变更即重置会话、会话内请求串行、异常后状态恢复、TS 侧长驻客户端与包视图接入 | **已完成**（Rust/TS 会话、环境失效、失败恢复、串行队列、REPL 长驻和 `--inLF` 交接均已验证；协议不改字段只改语义） |
| 11B | [终端交互式解释器](11b-interactive-repl.md) | TypeScript 终端前端、Rust 执行接线、单/多行会话与延迟包加载 | 进行中（I0–I4b 已按各自范围交付；语言级交互输入通道及后续 I5 另行实施） |
| 11C-0 | [消息目录与接入](11c0-localization-core.md) | 消息模型与渲染器、有效语言上下文与配置优先级、**协议带 `locale`/`text`**、诊断 `text` 渲染与旧核心兼容 | **机制已交付**；文案搬运由 11C-1 收尾，资源型语言包由 11C-2 接续，L4 发布仍未开始 |
| 11C-1 | [文案搬运与渲染器贯通](11c1-message-migration.md) | Rust 侧各 crate 文案逐条判定与搬运、两套目录补全、`XiaoError`/日志/`-debug` 窗口用**同一渲染器**、CLI 其余状态文案 | **已完成**（Rust 119 组 / CLI 230 全绿；目录 **317 键**，`xiao-driver` 清零）。§3.5 台账由审核补了 `xiao-ir` 与 `xiao-syntax/lexer.rs` 两行 |
| 11C-2 | [语言包插件与安全](11c2-language-pack-plugins.md) | 资源型插件清单、**不执行插件代码**、安装复用 11A 链路（锁文件/源优先级/不可变缓存/来源审计）、命名空间冲突拒绝、缓存键含目录版本与 ABI、**进程内降级/进程间严格** | **已完成**（清单、冲突、只读缓存、降级和 UseDocs 已交付；`.xar` 与 L4 跨平台发布不在本批） |
| 11C | [国际化、系统提示与语言包插件](11c-localization.md) | `[language]` 配置、中英内置目录、统一错误文案和资源型语言包插件 | 进行中（L0–L3 已交付；**L4 跨平台与发布验收未开始**） |
| 13 | [优化契约与统一管线](13-optimization-contract.md) | 语义保持边界、优化配置指纹、共享 Pass 管理和验证 | **已完成**（13A 契约与统一管线、13B 差分夹具两批均已交付；本阶段**不做具体优化**，那归 14/15/16） |
| 13A | [优化契约与统一管线的实施](13a-optimization-boundary-implementation.md) | 把「可以怎样变换程序」的边界立起来：`-O0` 也走完整管线（生成/规范化/验证）、共享 Pass 接口与 IR 验证器、快照与回滚、效果/所有权只读接口、优化指纹（时间戳与临时路径不入产物）、未优化 vs 优化的三样差分 | 已交付主体（`e3a6bd5`+`f7839d7`+`bf4e446`：配置/指纹、Pass 边界、只读事实、O0 管线、失败阻断）；**差分套件与 `13.9`–`13.12` 归 13B** |
| 13B | [差分套件与边界输入](13b-differential-suite-and-boundary-inputs.md) | 13 阶段收尾批：把差分从**函数**变成**会跑的夹具**，**先备好边界输入**（固定随机源、溢出上下界、动态值检查失败、容器顺序）、逐 Pass 前后快照与验证结果、清除未定义排序等非确定差异并列「不可避免差异」清单 | **已完成**（O0 共享差分套件、边界输入、确定性排序、环境门控动态回环；跨优化级别差分已随 14B 的三方差分接入） |
| 14 | [字节码优化与 `.xiaoc` 产物](14-bytecode-optimization.md) | 单模块分段字节码、默认缓存、加载验证和调试映射 | **已完成**（14A 格式与编解码、14B 字节码 Pass 与三方差分两批均已交付） |
| 14A | [`.xiaoc` 格式与编解码](14a-xiaoc-format-and-codec.md) | 单模块确定性二进制：**非定长**的文件头与可跳过目录项、未知必需拒绝/未知可选跳过、稳定排序与无宿主依赖、不压缩、平台无关优先、格式检查工具与模糊测试、VM 只接受验证过的产物 | 已交付（`0673fee`+`f9411a1`：分区 1–8、字典序规范化、FNV-1a 摘要、`malformed_random_bytes_never_panic`、`run_xiaoc` 检查全通过前不建 VM） |
| 14B | [字节码优化 Pass 与三方差分](14b-bytecode-passes-and-differential.md) | 14 阶段收尾批：窥孔/跳转/常量池/槽布局 Pass、**每个 Pass 前后快照**、优化后五项重新验证、**三方差分**（源码/未优化字节码/优化字节码 × 输出/错误/随机序列/drop）、调试回溯、quickening 与不可变 `.xiaoc` 分离 | **已完成**（窥孔/跳转/常量池/槽布局 Pass、逐 Pass 前后快照与重新验证、三方差分；不可证明的 Pass 明确报告跳过，未冻结回退策略） |
| 15 | [LLVM 原生优化与链接](15-native-optimization.md) | 原生优化级别、Runtime 裁剪、链接和跨平台基线 | **已完成**（15A–15E 五批均已交付：Pass 映射与裁剪、可复现构建与基线、产物层可复现、链接器符号与 strip、真实产物 CI 门控） |
| 15A | [LLVM Pass 映射与 Runtime 裁剪](15a-native-pass-mapping-and-runtime-trimming.md) | 规范化配置→LLVM Pass 管线、Pass 开关（**建开关但不默认启用**）、**依据调用图与效果摘要的可证明裁剪**、Pass 报告并入 13A 指纹；**先清 14B 两笔债**（释放点显式断言、七类可观察操作逐类核查） | 已交付主体（`0491713`+`b3a1999`：安全优化参数、O0–O3 开关、计划指纹含 13A 配置、可解释裁剪报告、四方差分模型、14B 债的断言）；跨平台实测继续补齐 |
| 15B | [可复现构建与性能基线](15b-reproducible-builds-and-performance-baseline.md) | 15 阶段收尾批：调试映射与内联栈、**构建规范化（时间/路径/符号排序）**、strip/调试符号/可诊断模式独立验收、三平台链接器差异诊断、五维基线（编译/启动/运行/内存/体积）、**固定条件 + 噪声阈值**、差异白名单成文 | 已交付主体（`27db393`+`c5cad41`：**LLVM 文本层**可复现比较与五类白名单、五维基线与噪声模型、14B 断言提升为逐项序列核对）；**产物层与调试映射见 15C** |
| 15C | [产物层可复现与调试映射](15c-artifact-reproducibility-and-debug-mapping.md) | 15 阶段收尾批：**比较对象下到产物字节**（PE `TimeDateStamp`、链接器符号表顺序、调试路径）、调试映射与内联栈（**在优化过程中维护**）、strip/调试符号/可诊断模式的**产物级独立验收**（含各 `-O` 级别）、性能基线取数 | 已交付部分（`578a0e8`：产物字节比较、**PE `TimeDateStamp` 显式归一化并记录**、可还原内联栈结构）；**§2.3 三项只做一项、§2.2 未做，见 15D** |
| 15D | [链接器符号规范化与 strip 独立验收](15d-linker-symbols-and-strip-acceptance.md) | 15 阶段收口批：**链接器产出的符号表顺序**（三种对象格式；**不可读时拒绝或标不可验证**）、**调试信息绝对路径**归一化、**strip/调试符号/可诊断模式的产物级独立验收**（含各 `-O` 级别）、基线取数 | 已交付判定层（`5400f58`+`cc412ea`+`4519eca`：符号表 **Readable/Unavailable**「不把看不见当顺序一致」、调试路径归一化、三态独立验收的**实现与单元测试（受控夹具）**；PE 导入导出**不冒充**内部符号表）；**真实产物门控见 15E** |
| 15E | [真实产物验收上 CI 门控](15e-ci-gated-artifact-acceptance.md) | 15 阶段收口批：把 15D 的**受控夹具**推到**工具链真实产物**（strip 三态各 `-O` 级别、真实符号表顺序、调试路径、重复构建逐字节比较、按平台记录的基线）；**⭐ 不改 CI 配置**——`reproduce.sh:131` 的 `--ignored` 已覆盖四平台 | **已完成**（5 条 `#[ignore]` 真实产物门控、O0-O3 strip/Debug、符号与路径、重复构建、平台独立基线；已同步 10D §2，CI 四平台实测 `37096599280` 全绿） |
| 16 | [SHA-256 内容寻址与二进制索引](16-content-addressed-artifacts.md) | 整文件摘要、归档/全局 Protobuf 索引和缓存维护 | **已完成**（16A 对象与索引、16A-FIX 编解码对称与单一来源修复、16B 缓存维护三批均已交付；不含 LRU 与 CLI） |
| 16A | [内容寻址对象与二进制索引](16a-content-addressed-objects-and-indexes.md) | `.xiaoc` 完整文件 SHA-256（**不含摘要字段自身**）、64 字符小写 + 两位分片命名、流式哈希与**碰撞比完整字节**、**五类对象命名空间分离**、两种 Protobuf 索引的确定性序列化与原子提交、扫描重建（**仅全局缓存**） | 已交付对象与索引核心（`ArtifactStore`、流式摘要、碰撞/隔离、五类命名空间、归档/全局索引确定性 wire 编解码）；归档接入与缓存维护归后续批次 |
| 16A-FIX | [内容寻址边界的编解码对称与单一来源修复](16a-fix-content-addressed-boundaries.md) | 抽出 `validate_archive_index` 供 encode/decode **共用**、删除重复的 `XIAOC_MAGIC` 与裸 `72`、登记索引锁崩溃残留归 16B | 已完成（入口校验对称、`.xiaoc` 契约单一来源、回归与架构约束测试全绿；索引锁残留与文档覆盖警告按 §五登记） |
| 16B | [缓存维护](16b-cache-maintenance.md) | `16.9` 引用扫描、`16.10` 两阶段显式清理与损坏隔离（**不做 LRU**）、`16.11` 离线读取、`16.12` 多进程与终止恢复；**统一两套跨进程锁** | 已完成（`xiao-lock` 唯一锁实现、保守引用集合、两阶段清理、离线验证读取与回归测试；不含 LRU/CLI） |
| 17 | [`.xar` 字节码归档与启动](17-xar-archive.md) | ZIP/ZIP64 归档、第三方依赖、资源与双击启动 | **已完成**（17A 格式与编解码、17B 资源与索引接入、17C 入口与运行器、17C-FIX 语言上下文与回归、17D 平台关联设计与调试窗口均已交付；17D 已补 8 条验收逐条收口评估、窗口断线降级、内存加载和可证伪性证据；真实安装器和打包命令按文档分别归 19、18） |
| 17A | [归档格式与编解码](17a-archive-format-and-codec.md) | 受限 ZIP/ZIP64 容器（**自写容器 + `flate2`**）、成员按字节序与固定时间戳、`STORE`/`DEFLATE-6` 选择、`ArchiveIndex` 扩展六个字段、**负例测试为主体**；另承接 20AB-FIX §7.3 的移除验证补强 | **已完成**（`bc10ba3`；**12 条验收过 11 条**——标准 ZIP 工具实测可读、两次编码逐字节相同、时间戳固定 `1980-01-01`、16A 侧为 81 行纯插入未改既有测试、承接项有运行期用例。**安全拒绝面缺 6 类，转 17B §2.1**） |
| 17B | [资源与索引接入](17b-resources-and-index-integration.md) | `17.5` 显式声明资源的收集与逻辑路径规范化（**不扫描项目目录**）、`17.6` 资源/调试符号/源码正文的内容寻址、`17.7` 标准发布包只含紧凑映射、`17.8` 四类稳定诊断；另承接 17A 的六类负例补强 | **已完成**（`215ef33`+`e27da4a`；**13 条验收过 12 条**——`[resources]` 从语法层杜绝目录遍历、反斜杠与盘符穿越已补实现、六类负例补齐、调试包差分通过。**凭据副作用探针缺失，转 17C §2.8**） |
| 17C | [入口与运行器](17c-entry-and-runner.md) | `17.9` 入口解析（**只读索引不猜**）、`17.10` 执行前完整验证（**不得部分执行**）、`17.11` 调用 09 统一 VM 保留源码映射/回溯/`drop`、`17.12` 四类错误码续用 `X17-XAR-*`、`17.13` 接入 11C 语言上下文；含 `xiao -xar` 与 `xiao run -xar` 两条等价 CLI | **已完成**（`7ebb996`+`b7791ad`+`3fec7da`；**13 条验收过 11 条**——两种 CLI 形式与端到端**审核实测跑通**且输出一致、`run` 未受影响、四类错误码续用 `X17-XAR-006`–`009`、承接的凭据探针已补。**三处缺口转 17C-FIX**） |
| 17C-FIX | [语言上下文接入与端到端回归](17c-fix-language-context.md) | 接 `language_locale` 到归档运行路径（**`XarRunOptions` 原无语言入口**）、补成功路径的端到端回归、补「不得部分执行」的**无执行事件**断言 | **已完成**（`aa089d2`+`a3350c4`+17D 承接回归；语言优先级「显式 > 归档 > `zh-CN`」、降级诊断带机器可读参数、双语目录、成功输出和校验失败无执行事件均已锁定。注：初版「运行时零消费」的判断不准确——17C 协议层其实读了该字段，更正与出错原因见该文档 §八） |
| 17D | [平台关联与调试窗口](17d-platform-association-and-debug.md) | `17.14` 三平台关联**设计**与行为契约、`17.15` 双击的工作目录/参数/IO/退出码、`17.16` 无关联时的同一入口与安装提示、`17.17` 启动器权限/临时文件/可审计验证记录、`17.18` **`-debug` 接进归档路径**；另**承接 17C-FIX 的两条回归缺口**（§2.10） | **已完成**（三平台关联设计、双击契约、双语安装提示、诊断窗口失败整体拒绝、生产事件控制和 `run_archive` 结果审计记录均已交付；真实安装器仍归发布工程 19） |
| 18 | [优化与产物 CLI 接入](18-optimization-cli.md) | TypeScript CLI、默认 `-O0`、缓存/验证/打包命令 | 进行中（拆三批：**18A** 优化参数与配置归一化、**18B** 后端调用与可诊断工作流、**18C** 文件关联与脚本化——**18C 含 `18.9`–`18.13` 五个子任务**，其中 `18.13` 是「`-debug` 全链路合并」的重活） |
| 18A | [优化参数与配置归一化](18a-optimization-parameters-and-config.md) | `18.1`–`18.4`：`-O0`–`-O3` 参数（**五个入口缺省一致**）、四层优先级归一化为 13A 的配置对象、五类输入的互斥/缺失诊断、按有效语言的帮助与机器模式；另**承接 17 的收口项** | **已完成**（`bb0f721`+`7806a1b`；**15 条验收过 13 条**——五入口缺省**逐一列举**测试、`-Ox`/`-O4`/`-O10` 被拒、17 收口评估 8 条逐条、窗口断线受控验证、可证伪依据表均已落地。**两处缺口转 18B §2.9**：命令行覆盖不落盘缺副作用探针、一处必要的既有测试改动未说明） |
| 18B | [后端调用与可诊断工作流](18b-backend-calls-and-workflow.md) | `18.5`–`18.8`：调 13–17 库接口（**不在 TS 重写** Pass/哈希/Protobuf/ZIP）、缓存命中与回退原因的四类稳定字段、输出前验证、四入口同一运行契约；新增 **`verify` 与 `cache`** 两个命令；**前置：把已有的优化管线接进编译路径**；另承接 18A 两处 | **进行中**（已接通字节码 O1--O3 生产管线、verify/cache 协议与 CLI 形状；LLVM Pass、全局 `.xiaoc` 缓存物化、四入口统一运行契约和完整缓存展示仍在本批后续提交中） |
| 18C | [文件关联与脚本化](18c-file-association-and-scripting.md) | `18.9` 三平台关联的**安装/检查/移除**、`18.10` 非交互与机器可读、`18.11` 中断/重复/只读/并发四类破坏场景、`18.12` 记录优化指纹与对象摘要与归档版本、`18.13` **`-debug` 全链路**（含 GUI 子系统）；另承接 18B 一处 | 已交付代码链路（三平台关联执行体、非交互 JSON、使用记录、`[debug]` 合并；**Windows GUI 子系统已有本机证据；Linux 裸机已回传但未见窗口，macOS 缺桌面宿主且 CI 跳过真实终端，见 10S**）；**阶段收口评估只有一段概括、缺逐条证据，四入口一致性缺比较用例，均转 [19A](19a-differential-and-fuzz.md) §2.5** |
| 20 | [内置函数与标准库：边界、契约与实施路线](20-builtins-and-standard-library.md) | intrinsic 单一来源契约、官方标准库与普通包边界、外部资源句柄、能力模型和分阶段接入 | 方向稿（跨 09–17 实施；不替代各阶段格式契约） |
| 20A-研究 | [intrinsic 契约与 20 的批次切分（待审）](20a-intrinsics-contract-research.md) | 裁定 20A/20B 能否分开、`print`/`input` 的签名规则、契约 crate 形态与 `IntrinsicId` 编码；含类型层 5 处名称分派的实测盘点 | **待审**（决策前置件；未审不得据以实施） |
| 20AB | [intrinsic 契约与最小生产入口](20ab-intrinsics-contract-and-minimal-entry.md) | 契约 crate 与声明数据文件、5 处名称分派迁移为查表、`print`(可变参数)/`input` 接入 VM 与 LLVM 两条路径；含**移除验证** | 实现已交付（契约表、VM/LLVM `print`/`input`、Runtime ABI 接通；**`print` 实测可用**）。**审核发现两项未达标**，见 20AB-FIX |
| 20AB-FIX | [生命周期诊断与移除验证](20ab-fix-lifetime-and-removal-verification.md) | 补 20AB §八 第 4 条的**移除验证**（删表项/VM 绑定/ABI 包装任一环节则用例必须失败）、修 `print` 的两条 `X06-LIFETIME-005` 诊断（含 `DynamicValue` 兜底文本，违反方向稿 §六 1） | 已完成（生命周期契约消费、`print`/`input` 回归、三项真删验证均已通过；**ABI 包装那项的失败证据是编译失败，其运行期补强转 17A §2.9**） |
| 20C-入口 | [20C 入口准备：骨架、清单同笔与首批模块的前置](20c-entry-and-skeleton.md) | 已落地 N-1 三处清单与最小骨架、N-2 `.xiao` 发现与同步门禁、N-3 并列身份块及普通依赖负例；P-3 两平台原生作业改为 workspace ignored 并补 `XIAO_STRIP`，排除项写明理由；模块实现转入 20C-首批 | **入口已完成（2026-10-13）**：真实 push 红/绿运行号、受控原生差分、产物级重复构建和 O-1 受控取数仍待外部执行 |
| 20C-第二批 | [能力探测与第二批接口清单](20c-second-batch-probe.md) | 七项候选已按真写、真跑、VM/LLVM 双路径成表；按裁定只冻结并实现 `text.length(str text) -> int`，字面量常量折叠、子串前瞻、切分、大小写转换登记为缺口，字符串参数比较与集合能力保留探测结论；P-6/P-7/P-8/N-A 已并入首批前置 | **探测与 `text.length` 已实现（2026-10-13）**；首批 UseDocs/registry 已由维护回归 `38550284761` 证明双路径并在第三批改为 `verified`，骨架条目仍为 `draft`，10R 其它遗留证据按各自文档登记 |
| 20C-语言边界 | [逻辑/身份算子：语义裁定、双路径接通与探测覆盖](20c-language-boundary-and-operators.md) | 承接 20C-第二批 §九.2 推迟给「后续语言/容器批次」的四条被挡候选。审核实测发现 **`and`/`or`/`is`/`is not` 四个算子「前端接受、生产降低器拒绝」**；初始探测夹具的转换错误遮蔽了 `and`，现已拆为转换夹具与逻辑算子夹具；裁定 `and`/`or` 的短路与释放语义、`is`/`is not` 的去留，再双路径接通，并把「已接受构造不得进入 `unsupported`」做成由编译器强制的执行点；另并入 P-9/P-10/P-11、遮蔽记名与审计侧自我更正。**第二轮四笔已并入**：探测 harness 缺「后端阶段拒绝」期望位（§2.7，规划侧的洞）、`second-case-conversion-logic.xiao` 无执行点（§2.8）、第二批 §3.3 标题计数（§2.9，已当场修）、`second-string-comparison.xiao` 未收窄（§2.10）。**第三轮更正**（§2.11）：三态 harness 的两条负例经变异实测确认为真，但 **VM 半边只钉错误码、不钉算子**（算子区分由原生半边承担），且**对账口径分「在册」与「准确」两层**——差集为空不等于 README 描述准确 | **§二 第一轮六笔、第二轮 F1/F2/F4、第三轮三条更正均已落地并审核通过**（`aac1237`、`fc114c8`、`e76513b`）；**§5.0 的三态 harness 与夹具接线已完成**；`README.md` 描述已同步，受控原生运行号仍待补。**§三 已由星崽 2026-10-14 裁定**：`and`/`or` 短路、先左后右、右侧未求值不产生释放计划（三条照审核倾向）；`is`/`is not` 取**乙**——类型层以 `X02-TYPE-005` 移出接受面（语法 token 原样保留）。**§三至§五已实现（2026-10-14）**：**受控运行号已取得**（维护回归 `38550284761`：两平台 `c20_capability_probe` 原生半边各 `2 passed`、各 14 行 `20C-PROBE`）；**§10.2 两条已裁定**——守卫**同时**声明为 09B0 的出口条件（落 [09B0](09b0-bytecode-closure.md) §3.3.1）、**继续 20C 不先推 20D**。首批模块已由 [20C-第三批](20c-third-batch-reprobe-and-status.md) 依据受控运行号翻为 `verified`；骨架条目仍为 `draft` |
| 20C-原生门禁修复 | [原生门禁修复专项（维护回归 38484238759 红）](20c-native-gates-red-fix.md) | `167a03e` 推送后两个原生作业都红，`main` 不绿；这是 2026-10-11 之后 **34 个提交的第一次 CI 曝光**。三条：**N-1** `and`/`or` 原生短路把 `i1` 当 `i8` 传（定位到动态路径 `dynamic/expression.rs:171`，静态 `ir.rs` 那条未覆盖）、**N-2** `value = 2 ** 3` 裸赋值原生 `动态整数幂` 不支持（既有缺口，被新守卫首次抓到）、**N-3** 硬件故障退出码跨平台不一致（Linux `132` vs Windows 硬编码 `139`，测试写死 Windows 值，归 10R `24f3f9b`）；另列让下一次能自动发现的五条执行点与审核侧两条自己的账 | **三条红已修复，`main` 已转绿（2026-10-14）**：`fb1f435` 后维护回归 **38493485483** 五个作业全绿（Windows `115623868914` / Linux `115623869640`），两平台 c20 原生半边各 `2 passed`、各 14 行 `20C-PROBE`；Linux 上加 `--no-fail-fast` 后**第一次真的跑到**该用例集。N-1/N-2 属 20C 语言边界批，N-3 属 10R。**三条处置已裁定**：N-2 修降低、N-3 按故障类别分派、**`--no-fail-fast` 加进 workspace 门禁**；**第二轮审核并入四笔返工**（§十一，没有一条是红，`c595c82` 已改）：F-A 释放对账是同义反复、F-B 注释把新转换说成既有惯例、F-C `allow(dead_code)` 与注释过期、F-D `ALL` 清单仍手写。**第三轮审核并入四笔**（§十二，同样没有一条是红）：`--no-fail-fast` 只治了跨测试二进制的遮蔽、`set -Eeuo pipefail` 让跨步骤仍是失败即停（被遮的是纯静态门禁）；§5.2.1 的落点与判据要改（审核侧的账）；F-A 的新断言只比总数不比位置。**返工后的 CI（前次记录）**（`5d32140`）维护回归 `38522338923` 与 workspace 门禁 `38522338905` 都绿，但 §5.2.1 第 5 条写明**全绿运行不构成该条取证**；当时仍留两条待做（跨步骤遮蔽 + 故意红取证、位置断言），本轮已完成，见下方「20C-门禁残余项」 |
| 20C-门禁残余项 | [原生门禁残余项修复：跨步骤遮蔽与释放位置断言](20c-native-gate-residuals-fix.md) | 门禁修复专项收口时故意留下的两条，都不是红、都不阻塞 main，但各让一条判据名不副实，故单独成文各自给「最小实施步骤 + 最小验证步骤 + 负例」。**R-1** 修复前 `tools/gates/run.sh:3` 的 `set -Eeuo pipefail` 让跨步骤仍是失败即停（第 12 行一红、13–21 行全不跑，被遮的是 `bun run check`/`fmt --check` 等纯静态门禁）；**R-2** 短路释放断言只比总数不比位置 | **已完成（2026-10-14）**：选择方案甲前置静态门禁；红运行 [38534726488](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38534726488) 与还原后的绿运行 [38535786407](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38535786407) 已回填（**绿跑的头是 `f5210a3`，不是 `5b693cd`**），R-2 位置断言的临时变异已让测试变红并已还原。**第三轮审核并入三条已全部收口**（§七，没有一条是红）：运行与提交的对应已写准（绿跑的头是 `f5210a3`）、方案甲的清单宽度——`tsc` 已一并前置到两个 `cargo test` 之前、远端 8 条历史一次性分支已清（现只剩 `main`，删前逐条核过未并入内容） |
| 20C-第三批 | [复探、状态收口与欠账对账](20c-third-batch-reprobe-and-status.md) | 承接 [20C-语言边界](20c-language-boundary-and-operators.md) §六 冻结的复探：四个被挡候选真跑双路径成表，由结果决定继续库批次还是转语言批次（乙的最小范围随后收口为整数族 → `str`）。同时收口首批两模块的 `draft → verified`——第②条件（CI 双路径运行记录）已由维护回归 `38550284761` 满足（Windows `115806283501` / Linux `115806283515` 均 `first_modules_match_native_results ... ok`）。**并入欠账对账**：关掉两条可关的，改回三处文档与代码不符（`ir.rs` 的 `llvm.trap` 实测 **0 处**，README 仍写「未实现/未通过」） | **已完成（2026-10-14）**：四条候选双路径复探全部被既定边界挡住，选择乙并冻结下一批接通整数族 → `str`；A-1/A-2 已关闭，B-1/B-2/B-3 已改回事实，B-4 复核后保留原限制。**四条裁定已下（星崽直接裁定 2026-10-14）**：甲乙**由复探结果决定、选出后先汇报**；乙的范围由第四批实施记录收口；守卫**同时**声明为 09B0 的出口条件；**继续 20C、不先推 20D** |
| 20C-第四批 | [`int`/`sint`/`lint` → `str` 转换](20c-fourth-batch-int-to-str.md) | 第三批复探四行全被既定边界挡住 → 按裁定选**乙**（语言批次）。本批把三种整数的 `as str` 与 `str(value)` 接到同一 Runtime 文本表示，动态入口复用既有 `xiao_runtime_value_cast`；不扩浮点、不改常量折叠、不新增 intrinsic。**并入三条返工**：语言边界批 §六 复探承接、第三批 A-2 落点措辞、**`20C-首批` 行状态格粗体闭合** | **已完成（2026-10-15）**：维护回归 `38597139383` 的 Linux 作业 `115966670682` 与 Windows 作业 `115966670696` 均执行 15 行 `20C-PROBE`；`second-int-to-str` 两平台输出均为 `42:42:42:42:42:42:42`；`1.5 as str` 仍 `X02-TYPE-006`，静态 `ir.rs` 未改经审核补测证实；F-1 仍为既有缺口，已登记 |
| 20C-第五批 | [库侧复探与推进](20c-fifth-batch-reprobe-and-library.md) | 承接第四批 §7.2 预告的「回到库批次」，按第二批 §3.2 真跑 VM/Windows LLVM 原生双路径并成表：数字格式化与 ASCII 大写分类解封；大小写转换、字面量常量折叠、子串/切分和集合公共接口继续登记。**并入两笔**：R-a 回填第四批 CI 运行号；R-b 登记 F-1 的 `lint` 数值转换债并同步注释 | **已完成（2026-10-15）**：`text.is_ascii_upper(str value) -> bool` 已加入纯 Xiao `text` 模块；`format_int` 候选因与 `str(value)` 零增量由第六批撤回；复探六行（子串/切分各一夹具）两平台结果逐字一致；F-1 仍为既有缺口，仅登记、不修复 |
| 20C-第六批 | [大小写转换的表达力与接口收口](20c-sixth-batch-case-conversion-and-api-closeout.md) | 承接第五批漏答的「大小写转换能否绕开 `str` → `int`」补探；并入 `format_int` 零增量撤回、模块状态与 CI 证据收口。**规模线**预先固定为超过约 30 行机械展开即算绕不开；复探夹具不使用变量下标、临时 intrinsic 或 Rust 逻辑 | **已完成（2026-10-15）**：63 行机械展开在 VM/Windows LLVM 原生逐字输出 `abcxyz`、`abcxyz`、`az-9` 和空行，按规模判据选择乙；仅登记下一语言批次的 `str` → `int` 码点前置，未实现语言能力；`format_int` 已三处同笔撤回；F-3 由维护回归 `38682801761` 覆盖并闭合；「最小接口」口径补严落在 20C §3.4.1 |
| 20C-第七批 | [`str` → `int` 码点转换与 `stdlib/**` 的 CI 覆盖](20c-seventh-batch-str-to-int-and-ci-coverage.md) | 承接第六批 §四乙立项；补齐三种整数宽度的单码点去程、`chr` 构造式回程，并新增纯 Xiao `text.lower_ascii`。同时修复 `stdlib/**` 与 `platform-reproduction.yml` 的轻量门禁触发缺口 | **已实施并完成 CI 取证（2026-10-15）**：只改 `stdlib/text.xiao` 的临时分支 push 同时触发 workspace gates `38753601863` 与维护回归 `38753601939`，两者均成功；维护回归 Linux/Windows 原生作业为 `116486098841`/`116486098536`。此前手动派发的 `38736878202` 仍作为 `7eeffbc` 上的 F-3 复核证据。**R-1/G-2 已收口**（§十三）：`stdlib/**` 已加入两套工作流的 `push`/`pull_request` paths，临时分支已删除；`38734698592` 仅保留为被证伪的历史运行。**审核后并入四条返工（§十二，无一为红）**：**F-2**（中）动态路径只修了整数目标，`str`/`bool` 目标的通用 `RuntimeChecked` 守卫仍报 `X06-RUNTIME-002`，取**乙：只登记**、进下一批前置；**F-1**（中）头注释已在 `7eeffbc` 改为事实；**F-3**（低）已在 `7eeffbc` 还原 `is_ascii_upper` 模块内调用，并由 `38753601939` 双平台原生回归复核；**F-4**（低）§六 残留计划期措辞已改完；G-3/G-4 已订正（G-4 的粗体未闭合曾在 `77228bb` 里**复发过一次**，见 [20C-第八批](20c-eighth-batch-conversion-and-dynamic-boundaries.md) §2.3） |
| 20C-第八批 | [转换与动态边界的收口](20c-eighth-batch-conversion-and-dynamic-boundaries.md) | 收两笔：第七批 §12.1 的前置 **F-2**（动态路径的通用守卫），以及第四批 §8.1 登记最久的 **`lint`/`lfloat` 数值转换债**；并把**欠了两批的「诊断身份」裁定**落定。**写规划时实测那笔 `lint` 债，发现登记表述是错的**：它不是「转换没实现」，而是**声明一个 `lint`/`lfloat` 形参就让整个产物在 VM 验证阶段被判损坏**（连从不使用该参数、`lfloat` 同样、`sint` 形参对照通过）；`lint → str` 这条第四批声称已接通并验过的也坏——因为其夹具 `second-int-to-str.xiao` **全是顶层写法，从未覆盖形参形态** | **已实施并完成 CI 取证（2026-10-16）**：F-2 提交 `acb0f90` 的维护回归 `38773875689` 成功；完整实现 `3bb7605` 的维护回归 `38779535750` 五作业全绿、workspace gates `38779535811` 成功，Linux/Windows 原生作业 `116575834999`/`116575834732` 均执行第八批夹具并通过。H-1/H-2/H-3 收口提交 `e94e1f8`/`bdf6428` 的维护回归 `38831166255` 与 workspace gates `38831166039` 均成功，覆盖 H-1 大整数回归；P-1 修复 `class_for_value` 形参类别根因，P-2 接通整数族 ↔ `lint`，浮点/lfloat 方向继续登记；进程内验证失败改报 `fatal.internal`，`X07-FATAL-002` **保留为外部产物校验预留**。**审核后并入三条（§十二）**：**H-1（高）** `lint` 大数值字面量**静默变 `0`**（`i64::MAX` 为界；同形状的 `int` 溢出却是正规前端错 `X02-TYPE-007`）——**既有**缺陷，已在 `lower/expr.rs:254-258` 增加 `ScalarType::Lint` 文本分支并以 `i64::MAX+1`/`i128::MAX` 回归；旧版本 `3bb7605` 与父提交 `acb0f90` 的临时探针均复现 `0`，故直接修、不转登记。**H-2（低）** P-1 两处改动中 `parameter_names` 那处经**变异 + 正对照**实测不承重（`class_for_value` 承重），其注释所述理由已被覆盖——已保留改动并改成不变量。**H-3（很低）** §十一 行号已订正；三条审核账已收口 |
| 20C-第九批 | [`unsupported` 的两分：已知未实现 vs 降低器缺陷](20c-ninth-batch-unsupported-diagnostic-split.md) | 承接第八批：它把**进程内验证失败**改成 `fatal.internal`，却没碰**另一条也通向 `fatal.internal`** 的路径——09B0 的 `unsupported` 守卫（`X09-BYTECODE-001`）。第八批审核实测：`lint a = 42` → `lint b = -a` 报 `X09-BYTECODE-001`，而那是**普通源码就能写出来的、已知未实现的操作**。审核按 14 类归组逐一复探，确认方法值与展开实参是可达的已知未实现，内部候选均由前端/IR 不变量拦截。 | **已实施（2026-10-17，`6a96198`）**：新增 `X09-BYTECODE-003` 分档，`X09-BYTECODE-001` 仅保留给内部降低缺口；`lint`/`lfloat` 一元负号用文本符号翻转接通，覆盖 `-0`、已有负号、`lfloat` 精度与零值归一化；其它高精度算术逐项登记为债项；H-5 定宽解析失败改为显式记录内部缺口；O0–O3 均保留分档；永久回归与第九批夹具已接线。受控 maintenance regression `38932473938`、workspace gates `38932473862` 均成功。**审核后并入（§十）**：**N-1/N-2/N-3/Z-1–Z-3 已完成**；Z-2 由共享尾数判零覆盖 `0e5`、`0.0e0`、`0E-3`，非零 `1e10`、`1.50` 保留文本；Z-3 将规范化函数抽到 `xiao-types`，字节码、LLVM 和 Runtime 共用同一判零谓词。 |
| 20D-规划 | [平台资源：阶段规划与 20C 出口对账](20d-platform-resources-stage-plan.md) | 路线稿 §七 把下一阶段定为 **20D 平台资源**（文件/目录/进程首批 API、能力上下文、三平台差分），**前置是「Windows 平台适配稳定，资源句柄契约冻结」**。规划前核了两件事：**(a) 20C 没有成文的阶段出口条件**；**(b) 20D 的前置未满足**——方向稿 §八 把与资源句柄直接相关的三条明列为「仍待冻结」，而「**能力上下文**」在仓内**不存在**（`Capability` 只有 `None`/`Stdout`/`Stdin` 三个变体，消费点只有 `exec.rs:1956`/`:1980` 两处**形状校验**，`xiao-runtime`/`xiao-driver` 里没有运行时载体）。另核出 **20C 的路线稿交付物「集合」模块未交**（被「公共函数参数类型未冻结」挡着），以及 **20C:395 一条坏掉的交叉引用** | **规划完成、三条裁定齐（2026-10-17，均取甲）**：**20C 按 §4.1 四条关闭**，并把「**集合**」按三要素立项带走（需要什么=**公共函数的容器参数类型**；挡点是函数参数类型而非运行时能力；阻塞官方 `set`/`collections` 全部公共接口），归属下一个语言批次；**20D 第一批是「冻结批」**，冻结 → 能力上下文 → 资源承载 → 首批 API **顺序不可倒**；「Windows 平台适配稳定」**定成可判定条件**（近端 5 次 push 触发的 Windows 原生作业全绿 **且** Windows 专属未结项清单为空）。**而判据的 (b) 半现在就不满足**：`00-decisions.md:697` 登记着 PE 专属的 `unverified-coff-exports`（工具链条件，**证据债而非缺陷**）。⇒ **20D 的前置不满足，第一步只能是冻结批**。**未改任何代码** |
| 20D-冻结 | [平台资源的前置冻结](20d-freeze-resource-contract.md) | 20D 的**第一步，且只做决定、不写代码**（依 [20D-规划](20d-platform-resources-stage-plan.md) §4.2 的裁定：冻结 → 能力上下文 → 资源承载 → 首批 API，顺序不可倒）。要冻六件：首批**资源种类**、**显式关闭**语义与关闭后别名、**复制/继承与跨调用边界**、句柄的**跨模块 ABI**（方向稿 §八 第 3 条）、首批 `os` API 与**平台能力清单**、**缺失能力的稳定错误码**（§八 第 6 条），外加**能力上下文的形状**。并一并写 20D 的阶段出口条件——**不重演 20C「走到第九批才补出口条件」** | **规划完成、四道裁定齐（2026-10-17）**：首批种类取**乙（只 `file` + `directory`，`process` 单独立项）**；关闭后访问别名取**甲（新开 `X06-RUNTIME-025`，与 `X06-RUNTIME-012` 的「值不满足操作要求」分开）**；能力码取**甲（能力不可用 = `X06-RUNTIME-026`）**；「目标不支持」的编译期码取**甲（新建 `X20-TARGET` 族，落地为 `X20-TARGET-001`）**；**句柄跨模块取甲（能跨模块，形式由本批读 ABI 现状后定）**——这一条与审核倾向（首版收窄）**不一致**，按 G-2 以直接裁定记。**它的连带把本批顺序绑住**：先读 `xiao-runtime-abi` 的现状，再定跨模块形式，然后才冻其余五件。**已核事实**：`is_hashable` 是**排除式**匹配（`containers/mod.rs:36-48`）⇒ 新增资源变体**会被误判为可哈希**。**已完成（2026-10-17，`4a16b37`）**：已读 `xiao-runtime-abi` 并冻结资源种类、关闭/别名、共享句柄、容器/产物边界、跨模块 ABI、首批 `os` API（含裁定后增补的 `os.read_text`）、三平台能力和 `X06-RUNTIME-025/026`、`X20-TARGET-001`；只登记诊断常量与 i18n，未实现资源 Runtime 或能力上下文。**审核核实**：ABI 是真读出来的（`XiaoValueTag` 用到 `Error = 16` ⇒ `ExternalResource = 17` 是真正的下一个空位；`XiaoAbiStatus`/`XiaoHandle`/`value_copy`/`_release` 逐一核过）。**审核后并入两条（§十一）**：**F-1** §四 第 1 条的「首批」有两个意思——文件/目录首批、**进程后置但仍计入 20D 出口**；**F-2** 首批 API 原只有开关、**无法验收**，已裁定**首批必须包含读**并补冻 `os.read_text`（**裁定后的增补**）。另认一笔审核侧的错：交接写「出口条件写进本文并生效」是错的，出口条件要裁定才生效 |
| 20D-能力上下文 | [能力上下文的载体与校验](20d-capability-context.md) | 冻结批 §2.7 已把**形状**定死（`u64` 位集、驱动器创建、随请求携带、执行期不可变、不暴露给 Xiao、缺位 → `X06-RUNTIME-026`）；**本批把它实现出来**。实施前核出两处与冻结直接相关：**(1)** 冻结的五个位里**没有 `stdout`/`stdin`**，而现有**唯一的两个外部入口**恰恰是 `print`/`input`——不解决它们的归属，本批**做不出负面证据**；**(2)** 方向稿 §5.2 那句「LLVM 仍处接口准备阶段、只能声称字节码侧完成」**已经过期**——`dynamic/expression.rs:504` 直接发射 `@xiao_runtime_print_values`，照旧文把本批缩到字节码侧会让原生路径**绕过能力检查** | **已实施（2026-10-17）**：按裁定新增位 5=`stdout`、位 6=`stdin`，位 7+ 请求拒绝；`DriverRequest`/`VmRunRequest` 携带不可变能力掩码，VM 和原生 LLVM 的带上下文 Runtime ABI 入口均先检查能力，缺位统一报 `X06-RUNTIME-026`；旧 ABI 符号保留默认全开兼容包装，Runtime ABI 次版本为 1.13。VM 缺 stdout/stdin、原生缺 stdout、未知位拒绝均有回归；未接资源 intrinsic |
| 20D-资源承载 | [`ExternalResource` 的 Runtime 变体](20d-resource-carrier.md) | 20D 的第三步（冻结 → 能力上下文 → **资源承载** → 首批 API）。冻结已定死种类、共享强引用、关闭幂等、容器/产物边界与 ABI 标签（`ExternalResource = 17`），本批新增 `RuntimeValue` 变体。**核心风险是方向稿 §4.2 警告的那条**：`is_hashable` 是**排除式**匹配，新变体漏改会被**误判为可哈希**。审核把新增变体的匹配点逐处核过，分成**两类**：**四处编译器不会提醒**（`is_hashable` 判可哈希、`type_name` 落 `"dynamic"`、`PartialEq` 让 `x == x` 为假、`value_to_runtime` 拒绝新标签），**五处编译器强制**（`Hash`、`scalar_type`、`display_text`、`runtime_to_value`、`protocol/mapping.rs`） | **规划完成、三道裁定齐（2026-10-17，均取甲）**：`is_hashable` **改正向允许**（以后忘处理的新变体默认不可哈希，失败安全）；**同一资源的两个别名算相等**（与冻结的「共享同一逻辑资源」一致）；`display_text` 印 **`resource<kind>`**（与 `type_name` 同形）。**三条的共同点是都选失败安全的那一侧**。**新加的一条验收**：`is_hashable` 是一次**语义反转**而非单纯加臂，**既有 19 个变体的判定要逐个确认不变**，不许只测「资源被拒了」。**未改任何代码** |
| 20C-首批 | [官方库首批模块：`text` 与 `path`](20c-first-modules.md) | 接口已冻结为 `text.contains_char(str text, str needle) -> bool` 与 `path.join(str base, str leaf) -> str`；两模块均为纯 Xiao、无新增 intrinsic。固定 `stdlib.` VM 加载器、LLVM 静态函数接线、移除验证和边界同步已落地；P-5 与覆盖口径订正已登记 | **受控双路径已由维护回归 `38550284761` 复核**（Windows `115806283501` / Linux `115806283515`）；10R 其它遗留证据按各自文档登记 |
| 20C | [官方 Xiao 库：能力探测、模块清单与兼容身份](20c-official-xiao-library.md) | 10 系列已按七项条件正式关闭，O6/B1/T4/F-3 的剩余证据保留原归属；20C 双路径能力探测、六条冻结、入口骨架、固定标准库加载边界和首批 `text`/`path` 接口已完成；官方模块仍不进入普通依赖图 | **首批实现与受控双路径实跑已完成（2026-10-14）**；维护回归 `38550284761` 已回填，10R 其它遗留证据仍按各自清单登记 |
| 10R-收尾 | [静态数值路径的收尾、证据接线与 20C 入口准备](10r-closeout.md) | 前置批次的 D1/D2 已经审核实跑复核（那行 `X11-DIAGNOSTIC-CHANNEL-001` 确已消失、身份/span/退出码不变；6 个 ignored 测试全过且断言不弱：IR 里没有 `llvm.trap`、依赖面不含 `WS2_32.dll`、13 个边界用例含除零 013、真非法指令仍 Fatal）。本批收 10R 出口条件的剩下三项（19D 基线判定、产物可重复性、F-3），并**并入两条修补**：**P-2 在修补前的核心证据在 push 上没有执行点**（当前两平台原生作业改用 workspace ignored）、**P-1 归档证据过期且 20C 那句建议方向已反**；另实现 10Z-D 并修复动态错误参数的原生释放差分/VM span；20C 入口四项只建骨架、不写库模块 | **实施中（2026-10-13）**：Windows workspace ignored 原生门禁已通过（含 15E、D19A 和 D4）；Linux/macOS 复跑、真实 push 运行号、产物级重复构建、F-3 与 O-1 仍待外部执行点 |
| 10R-实现 | [静态失败块接入可恢复错误通道](10r-static-overflow-implementation.md) | [10R-溢出后续](10r-static-overflow-followup.md) 原始立项只写了「把 `ir.rs` 的 7 处 `llvm.trap` 接到可恢复错误通道」，**但审核实测那条通道在静态路径上没有下游**：静态降低器不支持 `try`/`raise`（`ir.rs:871`）、**没有任何 drop/清理发射**、且非 debug 静态构建不链接 Runtime。**本批真正的题不是「trap 换成 call」，是「静态产物要不要从此链接 Runtime」**——这决定产物体积、可重复构建白名单与精确字节基线的走向。**链接探测、D1 解耦和 D5 逐程序补测已完成：四个基准与 19C 基线实际均 `uses_runtime=true`，D1 后样本依赖不再含 `WS2_32`**；7 处逐条定错误身份（除零必须是 `X06-RUNTIME-013` 而非 009），并保留平台非法指令 Fatal 分界。 | **D2/D4 已实现（2026-10-12）**：失败块和动态错误参数统一进入 Runtime 错误通道，ABI 次版本为 1.12；Windows 13 个静态边界用例、外部 Fatal、动态 ABI 结构测试通过。受控原生差分、静态异常清理、19D 基线判定与 F-3 仍待后续 |
| 20C-冻结 | [10 系列关闭与 20C 六条冻结](20c-freeze-and-10-series-closeout.md) | 按 [10Z-收尾](10z-closeout-execution.md) §2.5 的**七项关闭条件逐条对账**：**七项全满足**，而满仓库写的「不能关闭」给的理由（O6 缺口、B1、T4）**不是关闭条件**（O6 按 §2.5 第 2 条写明覆盖边界即不阻塞；B1/T4 按 §2.1 第 2 条不是前置）。本批已写出 §2.5 第 5 条要求的带走清单并按星崽裁定冻结 20C §3.2 六条；入口骨架与 N-1/N-2/N-3、普通依赖负例已由 [20C-入口](20c-entry-and-skeleton.md) 落地，官方库模块仍待后续批次 | **已完成冻结与入口状态回写（2026-10-13）**；10R 的真实 push、Linux/macOS 受控证据、F-3 和 O-1 仍按遗留清单登记；当前清单与兼容身份关系见 §3.4/§3.8 |
| 21 | [单线程 RC、强环与并发模型：架构建议讨论稿](21-rc-cycles-and-concurrency.md) | 一份外部架构建议的完整收录、逐条前提核实（含出处）、五个待议问题与初步评估 | 讨论稿（跨 06/07/20；结论落回各阶段文档前不得实现） |
| 12A | [阶段验收核实记录（01–11）](12a-stage-acceptance-audit.md) | 逐条核实 01–11 的验收标准：缺口清单、证据、三类处置建议；含三条**跨阶段共因**（LLVM 不支持选择器、快照测试不在 CI、有实现缺断言） | **核实已完成**（结论：无一是状态滞后，`00A` 可标已完成，其余各有真实缺口） |
| 21A | [单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md) | 收束 21 的待议问题，固定 DAG、`CrossThread`、原子计数债项、`Arena` 边界和 07-D 后置交接 | 已完成文档决策；实现后置 |
| 19 | [优化、兼容性与发布验收](19-optimization-release.md) | LLVM 原生 Java 对照、版本矩阵、跨平台和安全发布 | 进行中（19A 已交付，19B、19C 实现中，19D 规划稿已立；拆四批：**19A** 差分与模糊 `19.1`–`19.5`、**19B** 跨平台与发布 `19.6`–`19.9`、**19C** 维护与调试三平台 `19.10`–`19.14`、**19D** 性能对照 `19.15`–`19.18`） |
| 19A | [差分与模糊测试](19a-differential-and-fuzz.md) | 四路差分（未优化/优化字节码、`.xar`、原生）、固定随机源、级别间摘要相同与不同、四类解析器的确定性种子模糊测试；前置：原生 `-O1`–`-O3` 闸门（已按 A 放开）与 18 逐条收口评估 | 已实现（2026-10-06，仅 Windows 本机实跑；原生差分 7 例中 3 例一致、4 例登记为未覆盖；D2 已转 19B 修复，D3/D4 仍按登记口径保留） |
| 19B | [跨平台与发布](19b-cross-platform-and-release.md) | 先转绿 CI 并认领 19A 欠账，再建兼容矩阵、跨平台差分、`19.7`/`19.8` 补测和发布报告 | 主体已完成（2026-10-06；D3/D4 已裁定并登记；R1–R4 审核返工已落地；Linux/macOS 仍待 CI 证据；受控双构建仍未测量） |
| 19C | [维护策略与调试三平台](19c-maintenance-and-debug.md) | 先返工 19B 审核发现的 R1–R8，再定义格式废弃与缓存保留规则、实测不支持平台的拒绝、体积与安全回归接入 CI、三平台 `-debug` 失败路径与 `18.13` 遗留 | 实现中（2026-10-06；R1–R7、19.10–19.14 本机实现完成；Windows GUI 子系统已验证；跨平台最新结果见 10S；Linux 桌面可见窗口证据已补（2026-10-10，PR #5 合并 `efa9fd2`）、macOS 无桌面宿主；2026-10-06 审核发现 8 项返工 F1–F8，并入 19D 前置） |
| 19D | [性能对照](19d-performance-comparison.md) | 先返工 19C 审核的 F1–F8（含 `cache clean` 补引用来源），再做原生可构建性探测、Java 基线材料、测量与「通过/回归/数据不足」判定、按平台分栏的可追溯报告 | 实施中（Linux 受控取数与阈值已落地；4/5 可比工作负载实测，Windows 性能标缺、macOS 不可验证，O6 条件 5 仍不满足） |

## 当前最近里程碑

### A0 工程骨架与文档门槛

A0.1–A0.4 已完成：Rust 核心、TypeScript CLI、平台、测试、工具和资源目录均已登记，目录/README、UseDocs 链接和文档覆盖率门禁已可执行。公共 API 达到 100%，全仓库声明项达到 90% 以上；Bun workspace 与原生 AST 适配器协议已冻结。00E 已启用 2500 物理行门禁与大纲报告；`xiao-bytecode` 研究编码器和 `xiao-syntax/parser.rs` 均已完成解耦，当前 `check:layout`/`check` 应保持全绿。该阶段不添加语言功能实现。00A 后续仍需在进入各实现期时维护目录边界和交接文档。

A0 通过后才进入第 01 阶段的最小 Token 闭环：读取 UTF-8 源码，稳定记录字节偏移与行列号，识别 ASCII 标识符、十进制整数、`=`、换行和文件结束，并为非法字符给出稳定诊断。具体任务与退出条件分别见 [00A. 工程框架与目录布局](00a-project-layout.md)、[01. 词法 Token 与语法入口](01-lexical-and-grammar.md) 和 [12. 测试与开发里程碑](12-tests-and-milestones.md)。

01 当前已完成 F0/L0/L1/L2、严格最小 P0、P1 表达式/选择器和 P2-A 语法模块解耦；实际 API、快照格式和后续代理交接边界分别见
[01A. F0/L0 实现交接记录](01a-f0-l0-implementation.md) 与
[01B. L1 基础词法扩展交接记录](01b-l1-implementation.md) 与
[01C. L2 反引号、注释与缩进实现交接记录](01c-l2-implementation.md) 与
[01D. P0 最小解析器与 AST 实现交接记录](01d-p0-parser-implementation.md) 与
[01E. P1 表达式与选择器实现交接记录](01e-p1-expression-selectors.md) 与
[01F. P2-A 语法模块解耦交接记录](01f-p2a-syntax-decoupling.md)。类型与容器阶段的 C0、C1 静态阶段已完成，C2-A 最小集合静态闭环、C2-B 异构集合静态闭环和 C2-C 集合运算静态闭环也已完成；09R2F1 已消费集合运行时检查并执行集合代数、比较和成员判断，09R2G 已在三种研究 VM 载体接通 `for` 与六类值的迭代。其实现边界、交接清单和未负责事项分别见 [03A. C0 基础容器与精确路径](03a-c0-containers.md)、[03B. C1 有序容器选择器](03b-c1-ordered-selectors.md)、[03C. C2-A 最小集合静态闭环](03c-c2a-sets.md)、[03D. C2-B 异构集合与动态成员](03d-c2b-heterogeneous-sets.md)、[03E. C2-C 集合运算](03e-c2c-set-operations.md) 和 [03. 容器、集合与索引路径](03-collections.md)。集合增删和正式生产 Runtime 仍属于后续阶段。

04 阶段已完成 04-A 至 04-D 的静态闭环：函数参数/调用 AST、签名预登记与推断、条件/循环/返回约束以及脚本/工程入口元数据均已通过定向测试。该阶段只产出公开 AST、类型结果和 Runtime 检查计划，不执行 Xiao 代码；交接边界、诊断编号和后置消费者见 [04. 函数与控制流](04-functions-and-control.md)。

05 阶段当前完成了 05-A、05-B、05-C 和 05-D 的静态闭环：解析器能保留绝对导入与表 AST，模块分析器能从项目根发现 `.xiao` 文件、建立纯目录命名空间、解析词法绑定、传播顶层再导出并生成确定性依赖图，类型层能检查表成员、可见性和构造生命周期签名，配置层能把根 `config.xiao` 转换为独立的不可执行配置树并校验项目身份/包外导出。该闭环不下载外部依赖、不执行模块初始化或 `init`/`drop`，也不提供 Runtime 表值；接手实现时先阅读 [05A. 导入语法](05a-import-syntax.md)、[05B. 本地模块解析](05b-local-module-resolution.md)、[05C. 表静态闭环](05c-table-static-closure.md) 和 [05D. 配置静态闭环](05d-config-static-closure.md)，再进入 06 Runtime 或 11A 包管理。

06-A 已完成静态生命周期闭环：`xiao-lifetime` 能从既有 AST 和类型结果建立程序/函数/分支/循环/表作用域、控制流基本块、绑定与匿名对象、强/弱边、闭包/返回/容器逃逸事实，并为八类退出边生成确定性释放计划；强对象环报告 `X06-LIFETIME-001`，动态值保守提升并报告 `X06-LIFETIME-005`。该阶段不创建 Runtime 对象、不执行引用计数或 `drop`，后续 06-B/08 接手时先阅读 [06A. 生命周期静态闭环](06a-lifetime-static-closure.md)，再实现 Runtime/IR 消费。

06-B 已完成首版 Runtime 执行闭环：`xiao-runtime` 消费 05-C 的 `TableSignature` 和 06-A 的 `ReleasePlan`，提供私有不透明对象头、单线程非原子 Strong/Weak 句柄、UTF-8 `str`、固定宽度标量、严格 `bool ± 整数`、表构造/`init`/`drop` 状态机和 `finally -> drop -> catch/传播` 释放展开。清理错误进入 `suppressed`，不覆盖主错误；本阶段仍不实现 VM、LLVM、并发或数组/元组/集合/字典的真实 Runtime。接手 08/09/10 时先阅读 [06B. Runtime 对象与表生命周期](06b-runtime-objects-and-tables.md) 及 [Runtime 使用文档](../UseDocs/language/memory/runtime/README.md)。

07-A 已完成统一错误模型与报告器闭环：`xiao-diagnostics` 现在是 `XiaoError`、`FatalError`、统一 `StackFrame`、`ReportRecord` 和消息渲染接口的唯一实现；`xiao-runtime` 通过兼容门面迁移而不改变 06-B 行为。07-A 不实现错误控制流语法、日志文件、`-debug` 窗口或并发调度；接手后续 07-B/07-C/07-D 时先阅读 [07. 错误模型与并发安全边界](07-concurrency-and-errors.md) 的 07-A 交付记录和 [UseDocs 错误报告](../UseDocs/troubleshooting/errors-and-reports.md)。

07-B 已完成错误控制流与统一展开消费首版：`xiao-syntax` 提供 `try`、`catch`、`finally`、`raise` 的 AST 与恢复；`xiao-types` 固定按错误类型名检查可恢复边界、Fatal 禁止捕获和具体到宽泛的顺序；`xiao-lifetime` 建立 try/catch/finally 作用域、正常/错误/未匹配/控制转移的释放计划与展开图；`xiao-runtime` 测试驱动器验证具体类型路由、未匹配传播和 Fatal 隔离。接手 07-C/07-D 或 08 前端降低时，先阅读 [07-B 交接记录](07-concurrency-and-errors.md#07-b-已完成错误控制流与统一展开消费)、[UseDocs 错误控制流](../UseDocs/language/control-flow/error-handling.md) 和 [Runtime 展开](../UseDocs/language/memory/runtime/errors-and-unwind.md)。07-B 不包含错误码/条件/模式匹配、`Result` 泛型、`?`、VM、LLVM、日志文件、`-debug` 窗口或并发实现。

08A/U0 已完成统一前端首版：`xiao-driver` 按固定顺序串接解析、模块、类型和生命周期分析，累积诊断并在错误时停止降低；`xiao-ir` 输出覆盖当前已完成静态语义的递归类型化 IR，提供控制流、所有权、释放计划、选择器和错误边界；`IrValidator` 拒绝非法结构，稳定 JSON 快照带版本字段。该阶段不执行用户代码、不启动 VM/LLVM、不实现优化 Pass；接手 09/10 前端消费者时先阅读 [08A 交接记录](08a-u0-frontend-implementation.md) 和对应 [UseDocs 前端/IR](../UseDocs/language/compiler/README.md)。

09 阶段的前置特别研究工程 `09R1 → 09R2 → 09R3` 已完成并冻结：三机型原型、指令编码器、源码映射、79 条共享向量、容器/选择器/集合/迭代/表声明执行路径和 Windows 原生基准均已交付，冻结结论为**栈式机型、`FORMAT_VERSION = 3`、opcode `0..40`**。B0-A 已将 `xiao-bytecode` 与 `xiao-vm` 的实质实现迁入生产 `src/`，`research` 仅保留兼容重导出；B0-B 已接上生产运行契约（`RunRequest`/`run_request` 固定函数 0 与栈式载体，运行前无条件走 `verify_for_execution`，栈回溯经 `PcMap::span_at_pc` 接入 `ReportRecord`，生产事件接收器改为有界 `BoundedSink`）；B0-C 已在 `xiao-driver` 接上前端到 VM 的内部驱动器（`DriverRequest`/`DriverOutcome`、三段结构化失败、边界取消/超时和公共契约测试）；B0-D 已冻结五个退出码语义并记录 Linux 容器开发门禁，容器不改变原生基准口径。**冻结七项是本阶段的输入契约而不是待决项**；VM 中途取消检查点记为 `B0-C-CANCEL-001`，出口批次改为 `09-B0-E`。批次边界、迁移方案与各批可执行清单见 [09-B0. 字节码最小运行闭环](09b0-bytecode-closure.md)、[09-B0-B](09b0b-production-vm.md)、[09-B0-C](09b0c-frontend-to-vm-driver.md) 与 [09-B0-D](09b0d-exit-codes-and-linux-verification.md)。

09-B0-E 已完成 VM 两处热循环检查点、独立取消/清理通道、deadline 注入、CLI `AbortSignal`
接线和退出码回归；附加开关 A/B 结果见 [09-B0-E 性能报告](09b0e-checkpoint-performance.json)，
不改动 09R3 冻结报告。别名层清理仍按交接文档 §七.1 另行登记。

### 10 系列已关闭（2026-10-12）

[10Z-收尾](10z-closeout-execution.md) §2.5 的**七项关闭条件逐条满足**，星崽 2026-10-12 裁定关闭。
**关闭不等于 19 收口达标**：O6 五条仍是**四条「部分」+ 一条「不满足」**，覆盖边界见
[10Z-收口](10z-19-closeout-declaration.md)，**一个字未改**。关闭时按 §2.5 第 5 条把遗留项
逐条写进 [00. 决策基线](00-decisions.md) 的「10 系列关闭与遗留项」一节，**每条都写明卡在哪里**
（受控主机、真推送、工具链条件、未实现、结构性不可补等）。下一批按 [20C-冻结](20c-freeze-and-10-series-closeout.md)：
先落关闭判定与清单，再按 §3.6/§3.8 的守卫开官方库；**静态溢出先于 20C 库代码**。

## 文档变更规则

1. 新决策先写入 [00. 决策基线](00-decisions.md)，再更新受影响的阶段文档。
2. 实现按主表顺序推进，并同步满足 [12. 测试与开发里程碑](12-tests-and-milestones.md) 中对应的退出条件。
3. 规格存在冲突时先标记待定，不在实现中悄悄选择行为。
4. 阶段完成后更新本页状态，不在 README 重复抄写语言规格。
5. 模块代码与测试完成时必须同步提交 UseDocs；UseDocs 页面未达到 `verified` 不得把模块标记为已完成，具体规则见 [00B. UseDocs 同步政策](00b-usedocs-policy.md)。

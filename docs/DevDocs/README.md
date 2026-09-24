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

| 顺序 | 阶段与文档 | 主要交付物 | 状态 |
| --- | --- | --- | --- |
| 00A | [工程框架与目录布局](00a-project-layout.md) | Rust 核心、TypeScript CLI、平台、测试、工具和资源骨架 | 进行中 |
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
| 09R1 | [字节码寄存器机型特别研究](09r-bytecode-machine-research.md) | 统一三地址语义、三种候选机型、寄存器类别与编号空间、调用约定、异常与清理转移、编码草案和基准协议 | 已完成首版 |
| 09R2 | [字节码寄存器机型特别研究](09r-bytecode-machine-research.md) | 三种机型的可运行原型、共享语义向量、事件接收器和容器/选择器/集合/迭代/表声明路径 | 已完成（R2a、R2C、R2D、R2b、R2b1、R2F/2F1、R2G、R2H 与 R3 全部交付并冻结） |
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
| 10 | [LLVM 原生后端](10-native-backend.md) | `xiao build` 的 LLVM 原生二进制（Windows → Linux → macOS） | 进行中（N0-A/N0-B 已落地；N0-C/D 与用户可见 CLI 仍后置） |
| 10A | [LLVM 原生构建闭环](10a-n0-native-closure.md) | 手写 IR 文本 + 外部工具链、`xiao-runtime-abi`、四批交付（N0-A 纯静态 → N0-D 验证裁剪） | N0-A 已完成；N0-B Runtime ABI 已接续落地（Windows 原生、Linux amd64/arm64 与 macOS arm64 CI 功能复现；WSL/容器仅作功能证据） |
| 10B | [N0-B Runtime ABI](10b-n0-runtime-abi.md) | 动态值的 ABI 表示、真实引用计数与 `Weak`、容器与表 ABI、正常路径的释放计划 | 已落地（ABI/容器/表/正常释放计划；异常展开留 N0-C） |
| 10C | [原生 Runtime 链接缺陷修复交接](10c-native-runtime-link-fix.md) | `LNK1120` 的完整证据、Rust staticlib 原生库查询、MSVC ABI 调用约定修复与实际链接结果 | 已完成（Windows 原生、Linux amd64/arm64 与 macOS arm64 CI 动态 Runtime 功能闭环通过；WSL/容器仅作功能证据） |
| 10D | [环境依赖测试专项规范](10d-environment-gated-test-spec.md) | `#[ignore]` 取代静默 `return`、齐备环境的准备方式与三个坑、门禁的 `ignored` 计数要求 | 已完成（8 条门控测试默认可见；Linux amd64/arm64、Windows 各 8 条全绿，macOS 7 条执行且真实终端显式跳过） |
| 11 | [CLI、项目配置与平台](11-cli-config-and-platform.md) | TypeScript CLI、运行时配置、`-debug` 诊断入口和目标平台适配 | 进行中（X0-B/C/D/E/T 已落地；Windows、Linux amd64/arm64 与 macOS arm64 CI 功能证据通过；macOS 真实终端与 X0-SPEC-001 仍后置） |
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
| 11A | [虚拟环境与包管理](11a-environments-and-packages.md) | 项目隔离、多源包仓库、联邦源索引、共享缓存、锁定和管理命令 | 未开始 |
| 11B | [终端交互式解释器](11b-interactive-repl.md) | TypeScript 终端前端、Rust 执行接线、单/多行会话与延迟包加载 | 未开始 |
| 11C | [国际化、系统提示与语言包插件](11c-localization.md) | `[language]` 配置、中英内置目录、统一错误文案和资源型语言包插件 | 未开始 |
| 13 | [优化契约与统一管线](13-optimization-contract.md) | 语义保持边界、优化配置指纹、共享 Pass 管理和验证 | 未开始 |
| 14 | [字节码优化与 `.xiaoc` 产物](14-bytecode-optimization.md) | 单模块分段字节码、默认缓存、加载验证和调试映射 | 未开始 |
| 15 | [LLVM 原生优化与链接](15-native-optimization.md) | 原生优化级别、Runtime 裁剪、链接和跨平台基线 | 未开始 |
| 16 | [SHA-256 内容寻址与二进制索引](16-content-addressed-artifacts.md) | 整文件摘要、归档/全局 Protobuf 索引和缓存维护 | 未开始 |
| 17 | [`.xar` 字节码归档与启动](17-xar-archive.md) | ZIP/ZIP64 归档、第三方依赖、资源与双击启动 | 未开始 |
| 18 | [优化与产物 CLI 接入](18-optimization-cli.md) | TypeScript CLI、默认 `-O0`、缓存/验证/打包命令 | 未开始 |
| 20 | [内置函数与标准库：边界、契约与实施路线](20-builtins-and-standard-library.md) | intrinsic 单一来源契约、官方标准库与普通包边界、外部资源句柄、能力模型和分阶段接入 | 方向稿（跨 09–17 实施；不替代各阶段格式契约） |
| 21 | [单线程 RC、强环与并发模型：架构建议讨论稿](21-rc-cycles-and-concurrency.md) | 一份外部架构建议的完整收录、逐条前提核实（含出处）、五个待议问题与初步评估 | 讨论稿（跨 06/07/20；结论落回各阶段文档前不得实现） |
| 21A | [单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md) | 收束 21 的待议问题，固定 DAG、`CrossThread`、原子计数债项、`Arena` 边界和 07-D 后置交接 | 已完成文档决策；实现后置 |
| 19 | [优化、兼容性与发布验收](19-optimization-release.md) | LLVM 原生 Java 对照、版本矩阵、跨平台和安全发布 | 未开始 |

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

## 文档变更规则

1. 新决策先写入 [00. 决策基线](00-decisions.md)，再更新受影响的阶段文档。
2. 实现按主表顺序推进，并同步满足 [12. 测试与开发里程碑](12-tests-and-milestones.md) 中对应的退出条件。
3. 规格存在冲突时先标记待定，不在实现中悄悄选择行为。
4. 阶段完成后更新本页状态，不在 README 重复抄写语言规格。
5. 模块代码与测试完成时必须同步提交 UseDocs；UseDocs 页面未达到 `verified` 不得把模块标记为已完成，具体规则见 [00B. UseDocs 同步政策](00b-usedocs-policy.md)。

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

实现边界速览：字节码 VM 与执行 Runtime 的 Rust 决策见 [00. 决策基线](00-decisions.md) 和 [09. 字节码运行模式](09-bytecode-runtime.md)；TypeScript CLI/REPL 边界见 [11. CLI、项目配置与平台](11-cli-config-and-platform.md)；Java 对照性能目标与验收口径见 [19. 优化、兼容性与发布验收](19-optimization-release.md)。

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
| 04 | [函数与控制流](04-functions-and-control.md) | `def`、表达式、控制流和入口规则 | 未开始 |
| 05 | [表、模块与工程模型](05-tables-and-projects.md) | 表生命周期、源码模块、依赖图和包外导出 | 未开始 |
| 06 | [内存与运行时语义](06-memory-and-runtime.md) | 确定性释放、逃逸分析和引用计数 | 未开始 |
| 07 | [错误模型与并发安全边界](07-concurrency-and-errors.md) | 结构化错误传播、堆栈/日志诊断、数据竞争策略和并发模型边界 | 未开始 |
| 08 | [前端与统一中间表示](08-frontend-pipeline.md) | 从词法到类型化 IR 的统一编译前端 | 未开始 |
| 09 | [字节码运行模式](09-bytecode-runtime.md) | Rust 字节码解释器、执行 Runtime 与 `xiao run` 接口 | 未开始 |
| 10 | [LLVM 原生后端](10-native-backend.md) | `xiao build` 的 LLVM 原生二进制（Windows → Linux → macOS） | 未开始 |
| 11 | [CLI、项目配置与平台](11-cli-config-and-platform.md) | TypeScript CLI、运行时配置、`-debug` 诊断入口和目标平台适配 | 未开始 |
| 11A | [虚拟环境与包管理](11a-environments-and-packages.md) | 项目隔离、多源包仓库、联邦源索引、共享缓存、锁定和管理命令 | 未开始 |
| 11B | [终端交互式解释器](11b-interactive-repl.md) | TypeScript 终端前端、Rust 执行接线、单/多行会话与延迟包加载 | 未开始 |
| 11C | [国际化、系统提示与语言包插件](11c-localization.md) | `[language]` 配置、中英内置目录、统一错误文案和资源型语言包插件 | 未开始 |
| 13 | [优化契约与统一管线](13-optimization-contract.md) | 语义保持边界、优化配置指纹、共享 Pass 管理和验证 | 未开始 |
| 14 | [字节码优化与 `.xiaoc` 产物](14-bytecode-optimization.md) | 单模块分段字节码、默认缓存、加载验证和调试映射 | 未开始 |
| 15 | [LLVM 原生优化与链接](15-native-optimization.md) | 原生优化级别、Runtime 裁剪、链接和跨平台基线 | 未开始 |
| 16 | [SHA-256 内容寻址与二进制索引](16-content-addressed-artifacts.md) | 整文件摘要、归档/全局 Protobuf 索引和缓存维护 | 未开始 |
| 17 | [`.xar` 字节码归档与启动](17-xar-archive.md) | ZIP/ZIP64 归档、第三方依赖、资源与双击启动 | 未开始 |
| 18 | [优化与产物 CLI 接入](18-optimization-cli.md) | TypeScript CLI、默认 `-O0`、缓存/验证/打包命令 | 未开始 |
| 19 | [优化、兼容性与发布验收](19-optimization-release.md) | LLVM 原生 Java 对照、版本矩阵、跨平台和安全发布 | 未开始 |

## 当前最近里程碑

### A0 工程骨架与文档门槛

A0.1–A0.4 已完成：Rust 核心、TypeScript CLI、平台、测试、工具和资源目录均已登记，目录/README、UseDocs 链接和文档覆盖率门禁已可执行。公共 API 达到 100%，全仓库声明项达到 90% 以上；Bun workspace 与原生 AST 适配器协议已冻结。该阶段不添加语言功能实现。00A 后续仍需在进入各实现期时维护目录边界和交接文档。

A0 通过后才进入第 01 阶段的最小 Token 闭环：读取 UTF-8 源码，稳定记录字节偏移与行列号，识别 ASCII 标识符、十进制整数、`=`、换行和文件结束，并为非法字符给出稳定诊断。具体任务与退出条件分别见 [00A. 工程框架与目录布局](00a-project-layout.md)、[01. 词法 Token 与语法入口](01-lexical-and-grammar.md) 和 [12. 测试与开发里程碑](12-tests-and-milestones.md)。

01 当前已完成 F0/L0/L1/L2、严格最小 P0、P1 表达式/选择器和 P2-A 语法模块解耦；实际 API、快照格式和后续代理交接边界分别见
[01A. F0/L0 实现交接记录](01a-f0-l0-implementation.md) 与
[01B. L1 基础词法扩展交接记录](01b-l1-implementation.md) 与
[01C. L2 反引号、注释与缩进实现交接记录](01c-l2-implementation.md) 与
[01D. P0 最小解析器与 AST 实现交接记录](01d-p0-parser-implementation.md) 与
[01E. P1 表达式与选择器实现交接记录](01e-p1-expression-selectors.md) 与
[01F. P2-A 语法模块解耦交接记录](01f-p2a-syntax-decoupling.md)。类型与容器阶段的 C0、C1 静态阶段已完成，C2-A 最小集合静态闭环、C2-B 异构集合静态闭环和 C2-C 集合运算静态闭环也已完成；其实现边界、交接清单和未负责事项分别见 [03A. C0 基础容器与精确路径](03a-c0-containers.md)、[03B. C1 有序容器选择器](03b-c1-ordered-selectors.md)、[03C. C2-A 最小集合静态闭环](03c-c2a-sets.md)、[03D. C2-B 异构集合与动态成员](03d-c2b-heterogeneous-sets.md)、[03E. C2-C 集合运算](03e-c2c-set-operations.md) 和 [03. 容器、集合与索引路径](03-collections.md)。Runtime 消费仍属于后续阶段。

## 文档变更规则

1. 新决策先写入 [00. 决策基线](00-decisions.md)，再更新受影响的阶段文档。
2. 实现按主表顺序推进，并同步满足 [12. 测试与开发里程碑](12-tests-and-milestones.md) 中对应的退出条件。
3. 规格存在冲突时先标记待定，不在实现中悄悄选择行为。
4. 阶段完成后更新本页状态，不在 README 重复抄写语言规格。
5. 模块代码与测试完成时必须同步提交 UseDocs；UseDocs 页面未达到 `verified` 不得把模块标记为已完成，具体规则见 [00B. UseDocs 同步政策](00b-usedocs-policy.md)。

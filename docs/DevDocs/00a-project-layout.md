# 00A. 工程框架与目录布局

> 本阶段是进入代码实现前的工程骨架约定。它把 Rust 核心、TypeScript CLI、平台适配、测试、工具和资源分开，确保后续代理可以按工程期接手，而不会把实验代码塞进一个不可扩展的目录。本阶段只建立目录与职责，不假装已经实现编译器、Runtime 或 CLI。

## Agent 交接上下文

### 接手前提

- 先阅读 [00. 决策基线](00-decisions.md)、[08. 前端与统一中间表示](08-frontend-pipeline.md)、[09. 字节码运行模式](09-bytecode-runtime.md)、[10. LLVM 原生后端](10-native-backend.md) 和 [11. CLI、项目配置与平台](11-cli-config-and-platform.md)。
- 已冻结：字节码 VM 与执行时 Xiao Runtime 使用 Rust；CLI/REPL 终端边界使用 TypeScript；性能发布标准是 `xiao build` 的 LLVM 原生模式；原生平台按 Windows → Linux → macOS 接入。
- 本阶段的目录 README 是交接契约。新增代码目录必须先有 README，说明职责、工程期、依赖边界和不应放入的内容。

### 本阶段交付与不负责事项

- 交付：代码目录骨架、Rust crate/TypeScript 包的职责分配、测试与工具分层、平台适配顺序和文档注释质量门槛。
- 不负责：添加占位实现、冻结尚未确认的跨语言协议、选择具体 GUI 框架、选择 Apple 签名服务或编写业务功能。
- A0 的 manifest 字段、目录检查算法、稳定诊断码和覆盖率报告格式由 [A0. 工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md) 进一步细化；在其中列出的待确认项冻结前，不得提交实际 workspace manifest。

### 交接检查

接手代理必须能回答三件事：一段代码属于哪个工程期；它应依赖哪些层、不能依赖哪些层；相应目录 README 和文档阶段是否已经更新。没有归属和文档的临时文件不得进入主分支。

## 一级工程目标：建立顶层工作区

### 顶层目录

```text
core/                         Rust 语言核心与后端
  rust/                       Rust workspace 根
    crates/                   可独立测试、按职责拆分的 Rust crate
cli/                          用户可见命令与终端交互
  ts/                         TypeScript workspace 根
    src/                      CLI、REPL、配置和平台接线
tests/                        规格、单元、集成、差分、基准和模糊测试
tools/                        文档覆盖率、构建编排、Schema 和发布工具
resources/                    语言目录、Schema、模板等非代码资源
docs/DevDocs/                 规格与 SOP（README 只做索引）
```

`core`、`cli`、`tests`、`tools` 中的每一个存放代码的目录都必须有同级 `README.md`。`resources` 虽然不存放执行代码，也保留 README 记录资源格式、来源和使用期。

### 依赖方向

依赖只能从上层指向下层，禁止环形依赖：

```text
xiao-source
    ↓
xiao-diagnostics ← xiao-i18n
    ↓
xiao-syntax → xiao-config
    ↓
xiao-types → xiao-modules
    ↓
xiao-ir
    ├─ xiao-optimizer
    ├─ xiao-bytecode → xiao-vm → xiao-runtime
    └─ xiao-codegen-llvm → xiao-runtime ABI
                         ↓
                    xiao-driver
                         ↓
       xiao-artifacts / xiao-xar / xiao-platform
```

图示表达的是职责方向，不是要求所有 crate 直接互相依赖。共享数据结构应放在最小的稳定层；平台、归档和 CLI 不得反向进入语法或类型层。

## 一级工程目标：划分 Rust 核心

本框架暂按“核心 crate 优先使用 Rust”组织，以减少跨语言边界并复用 LLVM/Runtime 类型；已经冻结的硬性范围是字节码 VM 与执行 Runtime。若后续决定让某个前端或后端使用其他语言，只能通过 `xiao-driver`/IR 的稳定接口替换，不得把该替换扩散到 CLI 或语言语义。

### Rust crate 分配

| 目录 | 主要职责 | 使用工程期 | 不应放入 |
| --- | --- | --- | --- |
| `core/rust/crates/xiao-source` | UTF-8 源码、游标、字节偏移、行列和源码区间 | 01 | Token 规则、终端输出 |
| `xiao-diagnostics` | 稳定错误、诊断事件、原因链、源码标注接口 | 07、08、11C | 具体命令行解析 |
| `xiao-i18n` | 消息目录、回退、格式化和资源型语言包 | 11C | 编译语义、插件代码执行 |
| `xiao-syntax` | Token、缩进、AST、表达式和代码块解析 | 01–04 | 类型检查、VM 执行 |
| `xiao-config` | `config.xiao` 声明式子集、模式和规范化配置树 | 05、11、11A、11C | 执行项目代码 |
| `xiao-types` | 类型表示、推断、转换、溢出和容器约束检查 | 02–04、08 | 后端指令编码 |
| `xiao-modules` | 文件模块、命名空间、导入图、导出和初始化顺序 | 05、08、11A | 包下载和 Shell 激活 |
| `xiao-ir` | 类型化 Xiao IR、效果、所有权、源码映射和后端契约 | 08、13 | 平台路径、CLI 文案 |
| `xiao-runtime` | 值、容器、内存、引用计数、`drop`、错误运行库 | 03、06、07、09、10 | 命令参数解析 |
| `xiao-bytecode` | 指令集、常量池、验证输入和 `.xiaoc` 相关接口 | 09、14 | 解释循环本身 |
| `xiao-vm` | Rust 字节码解释循环、调用栈、调试事件和加载器 | 09、14 | TypeScript REPL 状态 |
| `xiao-optimizer` | 共享 IR/字节码优化 Pass、效果和验证 | 13、14、15 | CLI 级别解析 |
| `xiao-codegen-llvm` | LLVM IR 降低、链接、Runtime ABI 和目标代码 | 10、15 | `.xar` 物理打包 |
| `xiao-package` | 包源、依赖求解、锁定和环境物化的核心逻辑 | 11A | 终端提示符绘制 |
| `xiao-artifacts` | SHA-256 内容寻址对象、缓存和 Protobuf 索引 | 16 | 语言语义优化 |
| `xiao-xar` | ZIP/ZIP64 `.xar` 清单、成员和归档验证 | 17 | CLI 参数路由 |
| `xiao-platform` | 主机路径、进程、窗口、工具链和平台 API 适配 | 10、11、19 | 修改公共 IR 语义 |
| `xiao-driver` | 编译/运行/构建请求编排和跨 crate 稳定服务接口 | 08–18 | 终端 UI、翻译文本 |

每个 crate 的具体边界以其目录 README 为准；crate 名称是当前框架名称，不构成 Xiao 语言表面语法。

### Rust 源码内部约定

- `src/lib.rs` 或 `src/main.rs` 只负责模块装配；复杂实现放入职责明确的子模块。
- 对外数据结构使用版本化、可验证的类型，不暴露宿主内存布局。
- `unsafe` 只能出现在有专门 README/审计记录的极小边界；默认使用安全 Rust。
- 平台代码只能通过 `xiao-platform` 的 trait/接口进入上层，不能在语义 crate 中散落 `cfg` 分支。
- 生成的字节码、索引和归档都必须经过验证器；不能把“来自 Rust”当作格式安全证明。

## 一级工程目标：划分 TypeScript CLI

### CLI 目录分配

| 目录 | 主要职责 | 使用工程期 | 不应放入 |
| --- | --- | --- | --- |
| `cli/ts/src/commands` | `run`、`build`、`test`、`config`、`venv`、`sync`、`install` 等命令路由 | 11、11A、18 | 类型检查和 VM 指令 |
| `cli/ts/src/repl` | 单行/多行缓冲区、保存、确认、命令面板和按键处理 | 11B | 第二套解析器或执行器 |
| `cli/ts/src/config` | 项目/全局配置发现、覆盖和写入编排 | 11、11A、11C | 执行 `config.xiao` 中的代码 |
| `cli/ts/src/environments` | Shell 钩子、虚拟环境激活和提示符前缀 | 11A | 包解析算法的另一份实现 |
| `cli/ts/src/packages` | 包管理命令的用户交互和进度展示 | 11A、18 | 依赖求解核心 |
| `cli/ts/src/protocol` | Rust 核心的进程/库调用适配和版本协商 | 09、11 | Rust 内部结构镜像 |
| `cli/ts/src/diagnostics` | 结构化错误转终端显示、颜色和退出码 | 07、11、11C | 根据译文猜测错误类别 |
| `cli/ts/src/platform` | CLI 侧路径、Shell、文件关联和终端能力适配 | 11、18、19 | Runtime 平台语义 |
| `cli/ts/src/ui` | 提示符、分隔线、颜色、进度和无色降级 | 11B、11C | 用户程序输出重写 |

CLI 的 TypeScript 产物必须打包为无需用户另装 Node.js 的独立可执行程序。它只发送规范化请求、接收结构化结果并渲染；Rust 核心才执行 Xiao 用户代码。

### 跨语言边界

进程协议还是库 ABI 由第 09/11 阶段另行冻结。无论最终选择哪一种，边界都必须包含：请求版本、语言/Runtime 版本、目标条件、优化配置、源码/模块身份、结构化错误、退出码、取消和诊断事件。不得以解析人类可读 stdout 代替协议字段。

## 一级工程目标：平台适配和 Apple 预留

### 适配顺序

平台目录统一位于 `core/rust/crates/xiao-platform/src`，实现顺序为：

1. `windows/`：第一适配平台，先完成路径、进程、终端、动态库、诊断窗口和 LLVM 目标接线。
2. `linux/`：Windows 回归矩阵稳定后接入，补齐 POSIX 路径、进程、终端和桌面环境差异。
3. `macos/`：最后接入 Mach-O、`.app`、Universal Binary、签名/公证和 Apple SDK 检测。

平台无关代码放在 `common/`。后续平台不能修改公共 IR、数值、错误码、字节码或 Runtime 语义来通过测试。

macOS/iOS 的最终签名、模拟器和 App Store 分发需要 macOS/Xcode 或受控 macOS CI；Windows 上可以开发和准备交叉编译，但不把它当作完整 Apple 发布链。

## 一级工程目标：测试、工具和资源隔离

### 测试目录

- `tests/spec`：语法、类型、配置和格式的正反例。
- `tests/unit`：单个 Rust/TypeScript 模块的行为。
- `tests/integration`：Rust 核心与 CLI/平台的稳定接口。
- `tests/differential`：字节码、LLVM 原生和未优化/优化结果差分。
- `tests/benchmarks`：LLVM 原生 Java 对照、解释器诊断基线和资源指标。
- `tests/fuzz`：Token、IR、字节码、Protobuf、ZIP 和配置模糊测试。
- `tests/fixtures`：固定源码、损坏产物、平台样本和期望输出；不放测试逻辑。

### 工具与资源目录

- `tools/doc-coverage`：扫描 Rust/TypeScript/测试辅助代码的文档注释覆盖率。
- `tools/xtask`：跨平台构建、测试矩阵和本地开发任务编排。
- `tools/schema`：IR、字节码、Protobuf 和配置 Schema 的生成/校验工具。
- `tools/release`：发布报告、摘要复核、归档和签名流程编排。
- `resources/locales`：内置语言目录和插件样例，不执行代码。
- `resources/schemas`：机器格式 Schema、版本和兼容样本。
- `resources/templates`：新 crate、命令和测试的文档齐全模板。

## 一级工程目标：冻结文档注释质量门槛

### 覆盖率定义

- 全仓库由项目维护的 Rust、TypeScript、测试辅助和构建工具代码中，函数、方法、闭包包装类型、类及模块文档注释覆盖率不得低于 **90%**。
- Rust 的 `pub` 函数、方法、结构体、枚举、trait、字段、模块和 crate 文档必须达到 **100%**；TypeScript 的 `export` 函数、类、接口、类型、常量、模块和包入口必须达到 **100%**。
- 模块级文档使用 Rust `//!` 或 TypeScript JSDoc；公共项使用 `///` 或 `/** ... */`。文档必须描述职责、参数/返回值、错误、线程/生命周期约束和最小示例（适用时）。只有写“TODO”不计为覆盖。
- 测试和内部工具同样纳入 90% 统计；生成代码必须在生成模板中提供文档，外部依赖不纳入仓库统计。自动生成的公共包装层仍必须由 Xiao 代码提供 100% 文档。

### 自动化门槛

1. `tools/doc-coverage` 解析 Rust 和 TypeScript AST，输出总覆盖率、每包覆盖率、公共 API 缺口和未达标文件。
2. Rust 使用 `rustdoc`/`clippy` 的缺失文档检查，TypeScript 使用 JSDoc/TypeDoc 检查；自定义扫描器负责统一分母和跨语言报告。
3. CI 先检查公共 API 100%，再检查全仓库 90%；任一门槛失败都阻止合并。
4. 新增导出项必须在同一提交提供文档和至少一个规格/单元测试；重命名或移动模块必须同步更新目录 README。
5. 覆盖率报告保存到构建产物，但不能把报告本身当作 API 文档；文档质量仍需人工抽查示例、错误语义和版本约束。

## 二级实现任务

### A0：建立骨架

1. 创建本文件列出的顶层目录、Rust crate 目录、TypeScript 源目录、测试分层和资源目录。
2. 为每个代码目录添加 README，写明职责、工程期、允许依赖和禁止事项。
3. 建立 Rust/TypeScript workspace 的最小清单；本任务不添加语言功能实现。
4. 建立目录检查器，阻止未登记的代码目录和缺失 README 的提交。

详细的 A0.1–A0.4 交接顺序、工作区成员清单、UseDocs 模块登记和检查器输出契约见 [A0. 工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md)。A0 的“完成”必须同时包含清单一致性、目录 README、UseDocs 结构和文档注释门禁；只有目录骨架而没有可执行检查，不算完成。

### A1：建立核心接口占位契约

1. 为 `xiao-driver`、`xiao-vm`、`xiao-codegen-llvm` 和 CLI protocol 定义版本化接口草案。
2. 将请求、结果、错误、诊断事件和目标描述写成结构化 Schema；不绑定具体传输方式。
3. 为 Windows、Linux、macOS 生成平台能力清单，按顺序启用适配器。

### A2：建立文档覆盖率门禁

1. 先对新增 Rust/TypeScript 模板执行 100% 公共 API 文档检查。
2. 接入全仓库 90% 函数/类/模块覆盖率统计和差异报告。
3. 将检查接入本地 `xtask` 和 CI，提供缺口文件、符号和修复建议。

### A3：按工程期交接

1. 01–08 只在 `xiao-source`、`xiao-syntax`、`xiao-config`、`xiao-types`、`xiao-modules` 和 `xiao-ir` 增加前端代码。
2. 09–10 再启用 `xiao-runtime`、`xiao-bytecode`、`xiao-vm` 和 `xiao-codegen-llvm` 的执行代码。
3. 11–11C 启用 TypeScript CLI、平台接线、包管理、REPL 和国际化目录。
4. 13–19 启用优化、产物、缓存、归档和发布工具；不得为提前测试把后置逻辑复制到前置 crate。

## 验收标准

### 目录与依赖验收

- 顶层代码目录和所有已创建的子代码目录都有 README，README 能指出对应工程期和模块归属。
- Rust 核心、TypeScript CLI、平台适配、测试、工具和资源没有职责交叉；CLI 不执行 Xiao 语义，平台层不改变公共语义。
- Windows、Linux、macOS 的适配任务和 CI 门槛按既定顺序排列，后续平台不会跳过前一平台回归。
- 性能报告的正式标准是 `xiao build` LLVM 原生模式；解释器基准不会被误报为 Java 发布门槛。

### 文档质量验收

- 全仓库函数/方法/类/模块文档覆盖率 ≥90%。
- Rust `pub` 项、TypeScript `export` 项和公共模块/包入口文档覆盖率 =100%。
- CI 能在新增缺失文档时失败，并给出可定位的文件、行号和符号名称。

## 待定决策

- Rust 核心与 TypeScript CLI 最终采用进程协议还是库 ABI，以及协议编码格式。
- 编译器前端、共享优化器和 LLVM 后端是否在实现层统一使用 Rust；当前目录允许 Rust 实现，但不把它写成语言语义要求。
- Java 性能基线的发行版/版本、基准套件、统计阈值和发布优化配置。
- macOS GUI 框架、Apple 签名/公证服务和 iOS 宿主方案。
- workspace 的具体构建工具、依赖版本、缓存位置和 CI 提供商。

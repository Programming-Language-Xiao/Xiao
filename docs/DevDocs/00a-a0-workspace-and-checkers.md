# A0.1. 工作区清单、目录完整性与文档门禁实现方案

> 本文是 A0 的冻结实现契约。它把 `00a-project-layout.md` 中的目录目标落实为可检查的文件、命令、报告和退出条件。Bun workspace 与 Rust/TypeScript 原生 AST 适配器已经由星崽确认。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：跨语言边界、平台顺序和文档质量硬门槛。
2. [00A. 工程框架与目录布局](00a-project-layout.md)：目录职责、依赖方向和 crate 分工。
3. [00B. UseDocs 同步政策](00b-usedocs-policy.md)：用户文档树、页面元数据和模块交付规则。
4. [12. 测试与开发里程碑](12-tests-and-milestones.md)：A0 的退出条件以及后续阶段的测试门槛。

### 当前状态与边界

- 当前仓库已经包含 Rust/Bun workspace manifest、20 个 Rust crate 骨架、三个 TypeScript workspace 包，以及 A0 目录、单文件行数与覆盖率检查器；20 个 Rust workspace 成员均已 opt-in 到共享 `missing_docs` lint。
- `A0-SIZE-001` 已按 2500 物理行上限启用。`encode.rs`（3847 行）与 `parser.rs`（3040 行）是待后续拆分的已知债务，因此当前 `check:layout` 与 `check` 有意返回失败；不得为它们补豁免说明来伪造绿态。
- 本阶段建立工程可审计性，不实现 Xiao Token、类型、Runtime、VM、LLVM 或包管理语义。
- 代码、测试和 UseDocs 的同步要求从 A0 起生效；尚未实现的功能不得为了满足文档数量而伪造 `verified` 页面。
- CLI 工具本身使用 TypeScript；Rust `syn` 解析器作为内部解析组件，不形成第二套用户 CLI。

### 已落地文件与入口

- workspace 清单：根 `package.json`、`bun.lock`、`core/rust/Cargo.toml`、`core/rust/Cargo.lock` 和 `core/rust/rust-toolchain.toml`。
- 政策与登记：`tools/repo-check/repository.manifest.json`、`docs/module-registry.json` 及 `resources/schemas/` 下的两个 Schema。
- 目录与尺寸检查器：`tools/repo-check/src/cli.ts`、`tools/repo-check/src/size.ts`；覆盖率检查器：`tools/doc-coverage/src/cli.ts`；Rust 原生 AST 适配器：`core/rust/crates/xiao-doc-coverage-rust/`。
- 本地入口：`bun run check` 执行全套门禁；需要单项结果时使用 `bun run check:layout`、`bun tools/repo-check/src/cli.ts docs|usedocs` 或 `bun run check:coverage`。超标 Rust 文件会让 `check:layout`/`check` 调用 Rust 适配器，因此需有 Rust 工具链、预先构建适配器或设置 `XIAO_RUST_DOC_ADAPTER`。
- A0 用户页面：`docs/UseDocs/tooling/cli/repo-check.md`、`doc-coverage.md` 和 `doc-coverage-rust.md`，均已登记并验证。

## 一级工程目标：建立唯一且可交叉验证的工作区清单

### 清单的三层职责

A0 不把一个容易漂移的列表当作全部事实，而是交叉核对三个来源：

| 来源 | 作用 | 是否可被构建工具直接消费 |
| --- | --- | --- |
| `core/rust/Cargo.toml` | Cargo 的实际成员和共享 Rust 配置 | 是，Rust 构建的权威声明 |
| 仓库根 `package.json` | Bun workspace 的实际 TypeScript 包成员 | 是，TypeScript 构建的权威声明 |
| `tools/repo-check/repository.manifest.json` | 目录、工程期、README 和模块登记策略 | 否，属于仓库政策清单；必须与前两者一致 |

优先级固定如下：构建 manifest 决定“能否构建”，政策清单决定“是否允许进入仓库”，实际文件系统决定“当前观察到什么”。三者不一致时检查器报错，不能自动改写其中任何一个文件。

### Rust workspace 成员（已冻结）

`core/rust/Cargo.toml` 使用 virtual workspace，成员必须逐项列出，禁止 `crates/*` 等宽泛 glob。当前包含 19 个语言核心 crate 和 1 个仅供文档工具调用的内部 AST 适配 crate：

1. `xiao-source`
2. `xiao-diagnostics`
3. `xiao-i18n`
4. `xiao-syntax`
5. `xiao-config`
6. `xiao-types`
7. `xiao-lifetime`
8. `xiao-modules`
9. `xiao-ir`
10. `xiao-runtime`
11. `xiao-bytecode`
12. `xiao-vm`
13. `xiao-optimizer`
14. `xiao-codegen-llvm`
15. `xiao-package`
16. `xiao-artifacts`
17. `xiao-xar`
18. `xiao-platform`
19. `xiao-driver`
20. `xiao-doc-coverage-rust`（内部工具适配器，不属于 Xiao Runtime）

每个成员都必须有自己的 `Cargo.toml`、源码目录和同级 `README.md`。A0 可以使用最小可编译库骨架，但不得在骨架中加入语言功能或复制其他 crate 的职责。`Cargo.lock` 在首次引入依赖后提交，并由 CI 验证未被构建命令偷偷更新。

### TypeScript workspace（已冻结：Bun）

仓库根 `package.json` 作为 Bun workspace 协调器，显式登记当前真实包：

```text
cli/ts                  @xiao/cli
tools/repo-check        @xiao/repo-check
tools/doc-coverage      @xiao/doc-coverage
```

`cli/ts` 保留现有 `src/` 分层；检查器和覆盖率工具各自拥有 `src/`，并补齐 `README.md`。未来新增 TypeScript 包必须先加入根 `package.json`、政策清单、目录 README 和模块登记，再提交源文件。A0 不使用 `*` 或 `**` 自动发现成员，以免新目录绕过审查。

锁文件统一使用 Bun 生成的 `bun.lock`；不提交 npm、pnpm 或 Yarn 的第二套锁文件。Bun 只负责仓库工具和 CLI 的开发依赖，Xiao 用户包管理器的源、锁定和环境语义仍由第 11A 阶段单独实现。

### 版本与工具链的暂缓项

- Rust edition、最低 Rust 版本、Bun 最低版本和具体依赖版本已在 A0 manifest 中冻结，并由工具链/manifest 文件记录。
- A0 只冻结“成员和边界”，不冻结 LLVM 版本、打包器、GUI 框架或用户包源。
- 生成目录（如 `target/`、`node_modules/`、`dist/`、覆盖率报告）不是 workspace 成员，也不能被登记为源码目录。

## 一级工程目标：定义机器可读的仓库政策清单

### `repository.manifest.json` 的职责

文件固定为 `tools/repo-check/repository.manifest.json`，使用 JSON 便于 TypeScript 和 Rust 工具读取。它不是构建 manifest 的替代品，而是把“哪些目录必须存在、属于哪个工程期、如何交接”写成可审计数据。

结构如下（字段名和 Schema 已在 A0.1 实现中落地）：

```json
{
  "schemaVersion": 1,
  "rust": {
    "manifest": "core/rust/Cargo.toml",
    "members": [
      "core/rust/crates/xiao-source"
    ]
  },
  "typescript": {
    "manifest": "package.json",
    "members": [
      "cli/ts",
      "tools/repo-check",
      "tools/doc-coverage"
    ]
  },
  "codeRoots": [
    "core/rust",
    "cli/ts",
    "tools",
    "tests"
  ],
  "excludedDirectories": [
    ".git",
    "target",
    "node_modules",
    "dist",
    "coverage",
    "generated",
    "vendor"
  ],
  "readmeFile": "README.md",
  "moduleRegistry": "docs/module-registry.json"
}
```

示例只展示一项 Rust 成员以保持可读；实际文件必须包含完整 19 项和已确认的 TypeScript 成员。路径统一使用相对仓库根的 `/` 分隔、大小写敏感、不可包含绝对路径或 `..`。清单变更必须在同一变更中更新对应目录 README 和 DevDocs 索引。

### 模块登记表

`docs/module-registry.json` 记录代码、测试和 UseDocs 的可追踪关系，至少包含：

```json
{
  "schemaVersion": 1,
  "modules": [
    {
      "id": "rust.xiao-source",
      "stage": "01",
      "status": "planned",
      "code": ["core/rust/crates/xiao-source"],
      "tests": ["tests/unit/source"],
      "usedocs": ["docs/UseDocs/language/basics"]
    }
  ]
}
```

`status` 沿用 `planned`、`draft`、`verified`、`deprecated`。只有实现、测试和 UseDocs 页面都存在且验证通过时才能使用 `verified`；尚未实现的后续模块仍只登记规划关系，不把空目录标作已完成。

## 一级工程目标：实现目录完整性检查器

### 工具边界与入口

目录检查器放在 `tools/repo-check/`，命令行实现使用 TypeScript 并由 Bun 执行。它只读取和校验仓库，不修改清单、README、源代码或用户配置。提供四个稳定子命令：

```text
xiao-repo-check layout     # workspace、路径、README、源目录和单文件行数
xiao-repo-check docs       # DevDocs/UseDocs 登记、链接和状态
xiao-repo-check usedocs    # UseDocs 页面元数据、示例和阅读图
xiao-repo-check all        # 按固定顺序执行全部检查
```

`all` 的顺序固定为：加载清单 → 校验 Rust workspace → 校验 Bun workspace → 校验目录/README 与单文件行数 → 校验 DevDocs → 校验模块登记和 UseDocs → 调用覆盖率检查器。只有发现超标 Rust 文件时，行数检查才按需调用 Rust 大纲适配器。检查器不通过解析人类可读的编译器输出判断 workspace 状态；使用 `cargo metadata --format-version 1` 和 JSON manifest。

### 目录检查算法

1. 从显式 `--root` 或包含 `.git` 的最近祖先确定仓库根；拒绝根外路径。
2. 解析并校验 `repository.manifest.json` 的版本、路径和数组去重。
3. 读取 Cargo manifest 和 TypeScript manifest，展开显式成员，逐项确认目录、构建 manifest、源码入口和 README 存在。
4. 在 `codeRoots` 内枚举 `.rs`、`.ts`、`.tsx` 等项目源文件及其所在目录；排除清单中的生成/第三方目录。任何源目录缺 README 或未登记都报错。
5. 用统一换行口径统计每个源文件的物理行数；超过 2500 行时报告 `A0-SIZE-001`，并只为超标文件按需取得树形结构大纲。
6. 检查所有登记目录是否仍在仓库内，拒绝符号链接逃逸、绝对路径、大小写折叠冲突和同名 Rust crate/Bun 包。
7. 检查 README 至少有标题、目录职责、工程期、依赖边界（或明确“不适用”）四类信息；内容质量由人工审查补充，检查器不把中文句子当作语义证明。
8. 读取 `docs/module-registry.json`，确认代码、测试和 UseDocs 路径存在且路径大小写一致，并把结果交给文档链接检查。

当前遍历器不会扫描符号链接形式的源文件；仓内源码树已经确认没有此类链接。该限制必须保持
显式可见，后续若要支持，应先补齐越界与循环链接测试，不能另写一套文件遍历器绕过现有规则。

### 单文件行数与豁免

`A0-SIZE-001` 对 `codeRoots` × `sourceExtensions` 中的项目维护源码使用 2500 物理行硬上限，
注释与测试模块同样计入。行数判定独立于 AST；大纲不可用时仍保留尺寸 `error`，并额外报告
`A0-PARSER-001`。文本报告把树形大纲放在诊断详情中，JSON 原样保留 `details`，SARIF 则放入
结果 `properties.details`，主消息保持单行。

只有同目录的 `<文件名>的硬耦合需要的说明.md` 同时包含“边界”“理由”“替代方案”“移除计划”
四个非空章节时，尺寸诊断才降为仍然可见的 `warning`。空白或缺段说明会产生错误且不使豁免
生效；不提供目录级或全局白名单。

### 稳定诊断与退出契约

每条机器诊断包含 `code`、`severity`、`path`、`line`（可用时）、`subject`、`message_id` 和 `hint`。A0 的稳定代码如下：

| 代码 | 含义 |
| --- | --- |
| `A0-MANIFEST-001` | 政策清单缺失、版本不支持或格式错误 |
| `A0-MANIFEST-002` | 清单引用的仓库路径非法或不存在 |
| `A0-WORKSPACE-001` | Cargo/Bun 成员与政策清单不一致 |
| `A0-WORKSPACE-002` | 清单成员目录、manifest 或入口缺失 |
| `A0-WORKSPACE-003` | workspace 工具失败或输出无法解析 |
| `A0-WORKSPACE-004` | workspace 成员名称重复 |
| `A0-LAYOUT-001` | 源目录缺少 README |
| `A0-LAYOUT-002` | 源目录未登记、路径越界或大小写冲突 |
| `A0-LAYOUT-003` | 符号链接损坏 |
| `A0-LAYOUT-004` | 目录无法读取或枚举 |
| `A0-SIZE-001` | 单个项目维护源文件超过 2500 物理行，或其豁免说明不完整 |
| `A0-DOCS-001` | 模块登记路径、UseDocs 元数据或 Markdown 链接失效 |
| `A0-DOCS-002` | 已完成模块缺少 `verified` UseDocs |
| `A0-COVERAGE-001` | 公共 API 100% 或全仓库 90% 门槛未达 |
| `A0-COVERAGE-002` | 单个声明缺少代码文档 |
| `A0-PARSER-001` | 源文件 AST 解析失败，或超长文件结构大纲不可用 |
| `A0-PARSER-002` | 源文件无法读取 |
| `A0-PROTOCOL-001` | AST 适配器协议版本或响应形状不匹配 |

进程退出码只表达大类（0=通过，非 0=失败）；详细原因必须在 JSON/SARIF 报告中用上述稳定代码表达。任何检查器内部异常都使用单独的 `A0-INTERNAL-001`，不能伪装成通过。

## 一级工程目标：实现文档注释覆盖率检查器

### 统计对象与门槛

- 纳入 Rust、TypeScript、TSX、测试辅助和构建工具中的项目维护源文件。
- 排除第三方依赖、生成代码、快照输出、`target/`、`node_modules/` 和明确登记的机器生成目录；排除项必须写入清单并在报告中列出。
- 统计函数、方法、闭包包装类型、类、结构体、枚举、trait、接口、类型别名、模块/包入口等声明项。纯代码块、局部变量和测试数据不进入分母。
- Rust `pub` 项（含公共字段、模块和 crate 入口）以及 TypeScript `export` 项/包入口必须 100% 有代码文档注释；每个 Rust workspace 成员必须在 `Cargo.toml` 声明 `[lints] workspace = true`，标准 Clippy 命令以 `-D warnings` 执行。
- 全仓库声明项文档覆盖率至少 90%；同时报告每个 workspace 成员和每个文件，不能只给一个总数。
- “有注释”必须是与声明直接关联、包含实质描述的 Rustdoc/JSDoc；单行形式同样有效（例如 Rust `///`、`#[doc = "..."]` 和 TypeScript `/** ... */`），不要求注释或声明之间存在换行。只有 `TODO`、空注释或复制的机器标记不计入。UseDocs 页面永远不能抵扣代码注释缺口。

### 解析器适配层（已冻结：原生 AST）

覆盖率工具由 TypeScript 编写统一 CLI、配置加载和报告生成；语言适配器各自使用对语言语义最了解的解析器。用户可见的 CLI 始终由 TypeScript/Bun 提供，Rust 适配器是内部库/辅助进程，不形成第二套用户命令：

| 语言 | 适配器 | 负责识别 |
| --- | --- | --- |
| Rust | 独立 `xiao-doc-coverage-rust` crate 使用 `syn` Rust AST | `pub`/私有可见性、`mod`、trait、属性文档、方法和字段 |
| TypeScript/TSX | TypeScript Compiler API | `export`、默认导出、重载、类/接口/类型、JSDoc 归属和模块入口 |

适配器输出统一的中间记录：`language`、`file`、`line`、`kind`、`name`、`isPublic`、`hasDoc` 和 `parser`。解析失败必须终止该文件检查，不得退回正则扫描，以免把复杂语法误报为已覆盖。

Rust 适配器使用版本为 `2` 的 JSON 行协议。请求形如 `{"protocol_version":2,"files":["..."],"outline":false}`，其中 `outline` 可省略且缺省为 `false`；响应固定包含 `protocol_version`、`declarations`、`outlines` 和 `errors`，未请求大纲时 `outlines` 为空数组。覆盖率判定只读取 `declarations`；`A0-SIZE-001` 是 `outlines` 的首个消费者，只在 Rust 文件确实超标后请求该独立通道。TypeScript 尺寸大纲使用字段同形但类型独立的 `TypeScriptOutlineNode`，不进入版本化 Rust 响应校验。TypeScript 编排器在消费 Rust 响应前校验版本号和数组/记录字段，缺失或不兼容时报告 `A0-PROTOCOL-001`；Rust 源文件解析错误仍报告 `A0-PARSER-001`。协议版本与覆盖率报告的 `schemaVersion` 独立递增。

统一 tree-sitter 不作为 A0 实现路径；它对 Rust 属性/可见性、宏边界和 TypeScript 导出重载需要额外语义补全。若未来增加其他解析器，必须新增版本化适配器并在报告中标记，不能静默混用。

### 报告与差异门禁

覆盖率命令固定支持：

```text
xiao-doc-coverage --format text
xiao-doc-coverage --format json --out coverage/docs.json
xiao-doc-coverage --format sarif --out coverage/docs.sarif
```

报告至少包含扫描器版本、源快照摘要、排除目录、总分母/分子、公共分母/分子、每成员统计、缺口符号和行号。CI 先执行公共 API 100% 门槛，再执行全仓库 90% 门槛；任何新导出项没有同一变更中的注释和规格/单元测试都失败。A0 当前提供 text、JSON 和 SARIF 三种报告；基线差异显示属于后续增强，不能用基线掩盖公共 API 缺口。

## 一级工程目标：把 UseDocs 纳入交付闭环

### 同步时点

一个模块的实现顺序固定为：

```text
写实现 → 写/更新单元与规格测试 → 通过测试 → 写 UseDocs → 执行链接/示例/覆盖率检查 → 同一变更提交
```

UseDocs 页面可以在代码前以 `planned` 或 `draft` 状态存在，但模块状态不能在代码和测试完成前变为 `verified`。页面必须从主题索引可达，并链接到相关主题和故障排查页；移动代码或页面时同步更新模块登记和所有入链。

### A0 已交付的用户文档

A0 创建了 UseDocs 的多级目录、总索引、主题索引和模板，并为三个质量工具交付了可验证的使用页面；没有预写尚未实现的语言功能教程。结构约定见 [00B. UseDocs 同步政策](00b-usedocs-policy.md)，根入口为 [`docs/UseDocs/README.md`](../UseDocs/README.md)。

## 二级工程任务与交接顺序

### A0.1：manifest 与最小可构建骨架（已完成）

1. 将 Bun workspace 和原生 AST 适配器结论写入 `00-decisions.md`。
2. 创建 Rust virtual workspace、18 个核心 crate 与 1 个内部 Rust AST 适配 crate 的最小 manifest，以及 Bun workspace 成员 manifest。
3. 创建 `repository.manifest.json`、`module-registry.json` 和对应 Schema；不加入语言功能。
4. 运行 Cargo metadata、TypeScript workspace 清单解析和目录审查，记录初始基线。

### A0.2：目录检查器（已完成）

1. 实现根目录解析、路径规范化和清单 Schema 校验。
2. 实现 workspace 交叉核对、源目录发现、README 规则和符号链接/大小写安全检查。
3. 实现 Markdown 链接、模块登记和 UseDocs 元数据检查。
4. 为 workspace 成员漂移、缺少 README 和 Markdown 断链添加负例测试，并在同一变更中更新 `tools/repo-check` UseDocs（工具使用说明）。

### A0.3：覆盖率检查器（已完成）

1. 先实现统一声明记录和 JSON/SARIF 报告模型。
2. 接入 Rust `syn` 与 TypeScript Compiler API AST 适配器，覆盖私有项统计和公共 API 识别，并用版本化 JSON 协议连接两者。
3. 实现 100% 公共 API、90% 总体门槛和 text/JSON/SARIF 报告。
4. 为 TypeScript/Rust 解析失败、协议版本不匹配和未注释公共 API 建立固定负例；工具代码和测试辅助本身也必须满足门槛。

### A0.4：CI 与交接（本地门禁已完成，CI 接入由仓库流水线继续维护）

1. 在本地 `bun run check`/`xiao-repo-check all` 和 Rust `cargo metadata` 中接入固定顺序。
2. CI 保存 text、JSON、SARIF 三种报告；失败日志不依赖中文译文。
3. 更新根 README、DevDocs README、目录 README 和 UseDocs 索引，给接手代理列出输入、命令、输出和不负责事项。
4. A0 完成后才能进入第 01 阶段；A0 不以“已有语言功能”作为完成证明。

## 验收标准

### 工作区与目录

- Cargo 的 19 个成员、Bun 的实际成员、政策清单和文件系统完全一致。
- 每个源目录都有同级 README，README 能定位职责、工程期、依赖边界和对应模块。
- 路径越界、符号链接逃逸、大小写冲突、重复包名和未登记源目录均能稳定失败。

### 文档与 UseDocs

- UseDocs 不与 DevDocs 平铺；根索引、主题索引和子主题索引可沿相对链接往返。
- 已完成模块不存在缺失、孤立、断链或非 `verified` 页面；未实现模块不会被误标完成。
- 代码、测试、UseDocs 和登记表在同一可审计变更集中出现。

### 覆盖率

- 公共 API/导出项文档注释为 100%，全仓库声明项为至少 90%。
- 报告能列出每个缺口的文件、行号、符号、成员和修复提示，并可生成 JSON/SARIF。
- 解析失败、阈值失败和内部异常使用不同稳定诊断码；不以正则回退导致假通过。

## 已冻结与后置事项

1. TypeScript workspace 使用 Bun，成员显式列出，锁文件为 `bun.lock`。
2. 覆盖率使用 Rust `syn` AST 与 TypeScript Compiler API 原生适配器；统一 tree-sitter 不属于 A0 路径。
3. Rust 适配器作为内部 `xiao-doc-coverage-rust` workspace crate，由 TypeScript 检查器以稳定 JSON 协议调用；协议版本和报告版本独立记录。
4. Rust/Bun 工具链的最低版本在 manifest 中固定；升级必须有兼容性检查和同一变更中的文档更新。
5. WASM、并行扫描、增量缓存和宏展开后的生成 API 属于后续优化，不改变 A0 的报告契约。

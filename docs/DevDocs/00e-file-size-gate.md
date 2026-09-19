# 00E. 单文件行数门禁（`A0-SIZE-001`）交接

> **本文记录的门禁已完成实施。** `A0-SIZE-001` 已进入 `00a` 的规则总表并在
> `tools/repo-check` 中执行；其判定与接线提交为 `22fd395`，大纲渲染与适配器超时提交为
> `5cccee0`。当前 `encode.rs`（3847 行）与 `parser.rs`（3040 行）仍会使 `check:layout` /
> `check` 返回失败，这是留给后续拆分批次的显式债务，不得用豁免说明压低级别。
>
> 目标：**单个代码文件超过 2500 物理行即报错**，**报错时附上该文件的树形结构大纲**
> （结构 / 字段 / 函数 / 枚举 / 对象 / 成员的名字、所在行、定义句、嵌套层级、行数），
> 让接手者一眼看出该拆哪里、拆出什么。

## Agent 交接上下文

### 接手前提

1. [00D. Rust AST 适配器协议 v2 修正交接](00d-doc-adapter-protocol-v2-fixes.md) ——
   **本批的前置，已交付**。本文依赖它修好的 `end_line` 与 `lines` 语义，以及 `outline` 请求开关。
2. [A0. 工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md) —— A0 规则码总表在 `:188-206`，
   子命令契约在 `:165-182`。**新规则必须进那张表**。
3. [00. 决策基线](00-decisions.md) —— `:517-525` 的「架构耦合硬约束」是本门禁的成文依据，
   也是豁免机制的授权来源（本批要改它，见第八节）。
4. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 门禁命令、
   区分度验证、单一来源原则的主表在这里，**本文不复制**。
5. [Rust AST 适配器](../UseDocs/tooling/cli/doc-coverage-rust.md) —— 大纲通道的字段契约。
6. [仓库完整性检查](../UseDocs/tooling/cli/repo-check.md) —— 本批要改的用户文档。

### 本批交付与不负责

**交付**：`A0-SIZE-001` 的判定与诊断、树形大纲渲染、TS 侧结构大纲、旁置 md 豁免机制、
文档同步。

**不负责**：

- **不拆 `encode.rs`（3847 行）与 `parser.rs`（3040 行）**。门禁落地后 `bun run check` **为红**，
  这是**有意的**，另开批次处理（见第十三节）。
- **不统一 `tools/doc-coverage/src/checker.ts:174` 那个私有的目录遍历副本**。它与
  `repo-check/src/paths.ts` 是同一段逻辑的两份实现、错误处理还不同，但改它会改变覆盖率工具
  扫描的文件集——**覆盖率是 100% 门禁依赖的已验证数字**，不许在本批动摇它。记为已知漂移。
- 不把阈值写进 manifest。

### 现状数据（实测，可直接引用）

| 项 | 值 |
| --- | --- |
| 扫描范围 | 157 个文件、58205 行（`codeRoots` × `sourceExtensions`） |
| 超标 | `core/rust/crates/xiao-bytecode/src/research/encode.rs` **3847 行** |
| 超标 | `core/rust/crates/xiao-syntax/src/parser.rs` **3040 行** |
| 第三名（余量参照） | `core/rust/crates/xiao-types/src/checker.rs` 2087 行 |
| `encode.rs` 大纲规模 | 99 个顶层节点 / 196 个总节点 / 最深 3 层 |
| `parser.rs` 大纲规模 | 16 / 102 / 2 |

---

## 一、规则

- **`A0-SIZE-001`**，`severity: "error"`，`message_id: "a0.size.file_too_long"`，
  实现在新文件 `tools/repo-check/src/size.ts`。
- 阈值 **2500 物理总行数**（含注释与测试模块），写成模块内常量 `MAX_SOURCE_LINES`。
- **不进 manifest**。要进就得同时改三处：`types.ts`、`manifest.ts:144-163` 的手写解析器、
  以及 `resources/schemas/repository-manifest.schema.json`（它是 `additionalProperties: false`）。
  收益不抵成本。
- 扫描范围直接复用 manifest 的 `codeRoots` × `sourceExtensions`，排除 `excludedDirectories`。

**`severity` 写 `error` 就足够硬失败**，已核实：`report.ts:101` 的 `mergeResults` 与
`cli.ts:89` 的退出码都只看 `severity`。**不要**写成 warning 再加开关——那正是本仓
「规则看起来生效、实际没有」的病史形态。

**`line` 留空。** `types.ts:19` 已注明「未知时为空」，`00a:186` 写的是「`line`（可用时）」，
`layoutDiagnostic`（`layout.ts:139-148`）本来就从不设它。文件级规则没有单一有意义的行号；
设成 2501 会让 SARIF 的 `region.startLine` 把编辑器跳到无意义的物理行。
「从哪开始拆」的锚点放进 `message` 首行（例如「最大顶层节点 `impl Reader` 起于第 2901 行，占 300 行」）。

**`subject` 不要等于 `path`。** `report.ts:24` 渲染的是 `${location} ${item.subject}`，
`layoutDiagnostic` 把两者设成同一个路径，今天就在重复打印。本规则用
`subject` 放「3847 行 / 上限 2500」这类摘要，让首行有信息量。

---

## 二、行数口径：一个计数器，独立于任何解析器

**行数必须永远可得**——它就是 `readFileSync` 加一次计数，不依赖 cargo、不依赖 TypeScript。
大纲随时可能取不到（见第六节），行数不行。

**必须是一个共享函数**，`.rs` 与 `.ts` 走同一套：

```ts
const lines = text.split(/\r\n|[\r\n  ]/);
if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
return lines.length;
```

孤立 `\r`、U+2028、U+2029 都是 JS/TS 的行终止符；末尾换行不该多算一行。

**禁止**用 TypeScript 的 `getLineStarts()` 数 `.ts`、用别的办法数 `.rs`——两套口径会让阈值形同虚设。
已知：两个超标文件都以 `\n` 结尾，本口径与 `wc -l` 一致（3847 / 3040），**验收时要对得上**。

---

## 三、文件发现：扩展现有遍历，不要新写

`tools/repo-check/src/paths.ts:118` 的 `discoverSourceDirectories` 返回类型加 `files: string[]`：

- 在现有 `visit` 里**唯一判定「这是源文件」的那一处**（`:215` 的 `extensionSet.has(extension)`）
  顺手 push。
- **返回仓库相对路径**（与现有 `directories` 一致），由 `checkFileSizes` 内部
  `resolveRepoPath(root, file)` 还原成绝对路径再喂适配器。**适配器只接受绝对路径**：
  `scanTypeScriptFile` 的 `readFileSync` 与 `relativePath` 都按 `process.cwd()` 解析，
  而 `--root` 是真实入口（`cli.ts:36`，`repo-check.test.ts:38` 就在用它传子目录）。
- **push 之后必须 `sort()`**。`readdirSync` 的返回顺序依赖 OS；大纲要进报告、要被人 diff，
  不排序则 JSON/SARIF 输出跨平台不可复现。
- **把排除判断下移到 `entry.isDirectory()` 分支**。现在 `:178` 的
  `excludedNames.has(entry.name)` 排在 `isFile()` 之前，语义是「排除同名**文件**」。
  今天不误伤（7 个排除名都过不了扩展名检查），但那是巧合。一行改动换来规则不可被
  将来的排除名破坏。
- 其余行为（realpath 去重、越界 `A0-LAYOUT-002`、坏链接 `A0-LAYOUT-003`、
  读目录失败 `A0-LAYOUT-004`、符号链接一律不递归）**一律不动**，唯一调用方
  `layout.ts:57` 的 `directories` 结果逐字节不变。

**已知限制，写进文档不要修**：符号链接的源文件会被跳过（`:180-207` 无条件 `continue`），
理论上可被一条 `mklink` 绕过。已实测**仓内源码树没有任何符号链接**，本批不动这段逻辑。
不新建 walker，也不要去统一 `checker.ts:174-208` 的重复实现（理由见「不负责」）。

---

## 四、接线：接在 `checkLoadedLayout`，并接受它变成 async

接在 `tools/repo-check/src/layout.ts:39` 的 `checkLoadedLayout` 内（`:57` 的 `discovered` 已在手）。

**不要只接 `cli.ts`**——`bun run check:layout` 走的是 `checkLayout` → `checkLoadedLayout`
（`layout.ts:29`），接在 cli.ts 会让 `check:layout` 漏检。

**`checkLayout`（`:25`）与 `checkLoadedLayout`（`:39`）必须改为 async**，波及
`cli.ts:62`、`cli.ts:68` 与两处测试。这是有意的取舍：

- 把它们保持同步、把接线点挪到本就是 async 的 `runCommand`（`cli.ts:59`），签名就不用动；
  但**直接调用 `checkLayout()` 的路径会静默跳过一条结构规则**。
- 那正是本仓「规则看起来生效、实际被绕过」的病史形态。改为 async 会让 TypeScript
  编译器强制暴露**所有**调用点，代价是机械的，收益是不留暗门。

**Rust 侧用静态 `import`**：`tools/doc-coverage/src/rust-adapter.ts` 只依赖
`node:child_process` / `node:path` / `./types.ts`，**不成环，且 `import` 本身不 spawn 任何东西**。
「绿态不 spawn cargo」是**调用**的性质，用动态 import 换不来它，只会平白把 layout 检查器拖成异步。

**TS 侧才用动态 `import`**：`typescript` 约 9 MB，且 repo-check 的 `package.json`
**并未声明该依赖**（只有 `tools/doc-coverage/package.json` 声明了）。
沿用既有惯例的相对路径写法：`await import("../../doc-coverage/src/typescript-adapter.ts")`
（先例见 `cli.ts:73-76`）。包名 `@xiao/doc-coverage` **不可用**——没有 `exports`/`main`，
`node_modules/@xiao` 也不存在。

---

## 五、只在确实超标时才取大纲

- Rust：`scanRustFiles({ root, files: [全部超标文件的绝对路径], outline: true })`。
- TS：上面那个动态 import 出来的新函数。

**必须一次传全部超标文件，不要逐文件调用。** Rust 适配器每次调用都要启动 cargo，
逐文件调用会把启动开销乘以文件数。实测本机：

| 调用 | 耗时 |
| --- | --- |
| `spawnSync("cargo", ["--version"])` | **10 167 ms** |
| `spawnSync("node", ["--version"])` | 230 ms |
| 直调已构建的适配器二进制 | **64 ms** |

也就是说这 10 秒是 **cargo 进程创建本身的固定开销**（与 Xiao 无关），**每次 spawn 都付**，
不是冷启动效应。实现提交 `1d546fc` 把逐文件改成批量后：`check:layout` 31.5 s → 21.1 s，
`check` 44.3 s → 33.9 s。

**两个已知后果，必须写进本文档、UseDocs 与根 README**：

1. 门禁落地当天两个文件就超标，所以 `bun run check:layout` **每次都会 spawn cargo**（约 10 s）。
   `bun test` 里仓库级用例的超时已相应放宽到 `120_000`。
2. **`check` 从此依赖 Rust 工具链**。前置命令：`cargo build -p xiao-doc-coverage-rust`
   或设 `XIAO_RUST_DOC_ADAPTER` 指向预编译二进制——后者把每次调用从 10 s 压到 **64 ms**，
   是开发期的推荐姿势，而不只是一个排错选项。

---

## 六、取不到大纲时怎么办

**核心原则：行数与大纲解耦，`A0-SIZE-001` 的 severity 恒为 `error`。**

**不要**实现成「拿不到大纲就降级为 warning」——那会让门禁在没装 Rust 工具链的机器上
**静默失效**，而这恰恰是最该报错的场景。

- **给 `rust-adapter.ts:42-54` 的 `spawnSync` 加 `timeout`**（建议 `120_000`）。
  已实测它现在**没有 timeout**：另一个终端正在 `cargo build` 时会锁住 build dir，
  `check:layout` 会**永久挂起**——比报错糟得多，开发者会因此直接关掉门禁。
  `response.error` 的 `ETIMEDOUT` 归入「大纲不可用」，`reason` 写明「适配器 120 s 未返回，
  可能有并发的 cargo 构建持锁」。
- **大纲不可用时另报一条 `A0-PARSER-001`**。**复用冻结表里已有的码**（`00a:204`），
  **不要发明新码**——新码要连带改规则总表与 UseDocs，收益为零。
- 该诊断的 `hint` 必须**能手工复现**：给出请求体 JSON
  （`{"protocol_version":2,"files":[...],"outline":true}`）与两条恢复路径
  （`export XIAO_RUST_DOC_ADAPTER=<预编译二进制>` / `cargo build -p xiao-doc-coverage-rust`）。
- 源文件本身语法错误时，适配器走 `Err` 分支、`outlines` 为空——**同样只是没有大纲，
  行数照报**，`A0-SIZE-001` 照常是 error。

---

## 七、渲染：加结构化字段，不要只往 `message` 里灌

`Diagnostic`（`types.ts:13-30`）加可选 `details?: string[]`；`message` 只留一行摘要。

**理由（不要退回「全塞 message」）**：`mergeResults` 那条核实只覆盖**退出码**，没覆盖渲染质量。
全塞 message 会同时坏三处：

- **SARIF**：`report.ts:60` 的 `message: { text: item.message }` 是给 UI 单行渲染的字段，
  196 行树形结构在查看器里必然变成一坨。
- **JSON**：换行被转义，整个大纲成为报告里的一行巨长行，**任何 `--out` 报告都无法 diff**。
- **i18n**：`types.ts:29` 写明 `message_id` 是「供国际化层使用」的稳定编号。
  一个带 `message_id` 又塞了 20 KB 逐文件数据的 `message`，意味着重建文案时丢大纲。
  **大纲是数据，不是文案。**

改法（约 8 行）：`renderText`（`report.ts:19`）缩进打印 `details`；`renderJson`（`:35`）
天然带上；**`renderSarif`（`:51`）把 `details` 放进 result 的 `properties`**，`text` 保持一行。

**截断按结构，不按字符**：

- **顶层节点全给**——那才是「该拆哪里」的答案。
- 总节点超过 **400** 时，保留较大的子节点，并**明示省略量**（如「……省略 42 个更小的节点」）。
- 今天**不会触发**（最大 196 个节点），但 2500 是**下限不是上限**；一个 20000 行的文件
  能产出上千节点，会撞上 SARIF / GitHub 的长度上限并让报告彻底不可读。
- **不得**把完整大纲写到文件——`00a:163` 冻结了「检查器只读且不自动修复」，写文件是越界。

---

## 八、豁免：旁置 md，且**必须有内容**

同目录存在 **`<文件名>的硬耦合需要的说明.md`** 时，该文件降级为 `warning`：
**仍然报出来**（债要可见），但不使门禁失败。

**不做目录级或全局白名单**——那正是本仓「规则看起来生效、实际被绕过」的病史形态。

**但一个空文件就能关掉门禁，等于没有门禁。** 照 `paths.ts:244-250` 的
`readmeMissingSections` 范式写一个 `exemptionMissingSections(content: string): string[]`，
要求旁置 md 含**四段非空**内容：

| 段 | 要求 |
| --- | --- |
| 边界 | 为什么这个文件不能拆、拆了会破坏什么 |
| 理由 | 为什么不选替代方案 |
| 替代方案 | 试过或考虑过哪些拆法 |
| 移除计划 | 什么条件下可以摘掉这份豁免 |

（这四项正是 `00-decisions.md:524-525` 要求的「边界、理由、替代方案和移除计划」。）
**内容不全时豁免不生效**，并额外报一条诊断说明缺了哪一段。

### 必须同时改 `00-decisions.md`

`00-decisions.md:517-525` 的例外条款原文是：

> 该约束优先于短期代码复用便利；若确需例外，必须先在本文件记录边界、理由、替代方案
> 和移除计划，未记录的跨层依赖一律视为错误。

它的适用面是**「跨层依赖」**。旁置 md 的名字沿用了这段措辞，但「3847 行」**不是跨层依赖**，
所以必须**显式把适用面扩到「单文件行数」**，并写明旁置 md 的四段内容要求。
不改这一步，读者会认为旁置 md 答非所问。

`DevDocs/README.md:126` 规定新决策先写 00-decisions、再更新受影响的阶段文档——
**所以这是前置项，不是可选项**。用户已明确授权修改该条。

**豁免不是本批的首选路径**：`encode.rs` 与 `parser.rs` 的**预期是拆分**，
不是写一份说明把它们降级。豁免是给「确实拆不动」的文件的出口，不是避难所。

---

## 九、TS 结构大纲

在 `tools/doc-coverage/src/typescript-adapter.ts` 新增 `outlineTypeScriptFile`，
字段形状与 Rust **完全一致**（`kind` / `name` / `line` / `end_line` / `lines` / `signature` /
`source_line` / `children`），**并新增独立的 TS 类型**——不要去改 `RustOutlineNode`，
它被 `validateRustAdapterResponse` 的形状校验和两条测试锁死，改动爆炸半径大而收益为零。

**以下八条全部实测过，是这条路径最容易翻车的地方：**

1. **`node.name` 可能是 `undefined`**。探针实测：`export default class { m() {} }`、
   `export default function () {}`、`constructor` 的 `name` **全为 `undefined`**，
   `name.getStart()` 会**抛 TypeError**。必须统一兜底 `const anchor = node.name ?? node`，
   名字文本用 `node.name ? node.name.getText(sf) : "<default>"`。
   现有 `declarationRecord` 已经为其中两类打过补丁，这证明它不是假想问题。
2. **`line` 取 name 节点的 `getStart`，不是 `node.getStart`**。实测 `export class C` 的
   `node.getStart()` 指向 `export`，装饰器更会带上好几行。这与 Rust 侧
   「header 取 ident 跨度、full 取整项跨度」同构。
3. **`kind` 必须手写映射表**。探针实测 `ts.SyntaxKind[VariableStatement]` 打印出来是
   **`FirstStatement`**（同值别名）。任何「用 `ts.SyntaxKind[kind]` 当 kind 字符串」的
   偷懒写法都会漏出内部名。词表要与 Rust 侧对齐成一套可读的公共词表。
4. **`const a = 1, b = 2` 要展开每个 declarator**。实测是两个 `VariableDeclaration` 子节点；
   现有 `declarationRecord` 只取 `[0]`，会静默丢掉 `b`。
5. **接口成员要收进来**。实测 `interface I { foo(): void; bar: number }` 产出的是
   `MethodSignature` / `PropertySignature`；`PropertyDeclaration` / `EnumMember` /
   `IndexSignature` 同理——现有分类器**一个都不认**，而它们正是「字段 / 成员」。
   **TS 大纲需要独立分类器，不能复用 `declarationRecord`。**
6. **`source_line` 用 `sourceFile.getLineStarts()` 切片**，不要 `split(/\r?\n/)`。
   孤立 `\r` 与 U+2028/U+2029 都是 TS 的行终止符，split 会与行号**错位**——
   而 `source_line` 是接手者唯一用来核对「这个节点是什么」的字段，**错位比没有更危险**。
7. **`signature` 按名字的字符区间切片拼接**，不要照抄 Rust 的 `head.find(name)`。
   那是按**子串首次出现**删名；TS 里 `T`、`id`、`x` 这类短名极常见，会被放大成大量错乱签名。
   （Rust 侧同一个隐患存在但暴露概率低，本批不动它。）
8. **best-effort**。不要照抄 `scanTypeScriptFile` 的「有 parse diagnostic 就抛」：
   Rust 侧解析失败时仍尽力产出（`outline_item` 用 `_ => return None` 收尾，注释写明
   「大纲是尽力而为，不该让它让整个扫描失败」）。TS 侧一个可恢复的 `}` 就丢掉整个大纲，
   会让最有价值的诊断在最需要的时候消失。

**如实记录**：仓内最大的 `.ts` 是 `tools/repo-check/src/docs.ts`（约 327 行），
离 2500 有 7 倍差距，所以**这条分支在可预见的将来不会被真实文件触发**。
门禁的强制力是完整的（行数与语言无关），推迟的只是可读性。正因如此，
**TS 大纲必须靠 fixture 测试覆盖**，不能指望真实仓库。

---

## 十、可测性：大纲提供者必须可注入

```ts
checkFileSizes(root, manifest, files, options?: { outlineProvider?: OutlineProvider })
```

默认走真适配器，**测试注入假提供者**。否则 fixture 测试根本跑不了：
`rust-adapter.ts:41` 硬编码了 `core/rust/Cargo.toml`，而 fixture 里没有
`xiao-doc-coverage-rust` 这个包；而且不能让 `bun test` 依赖 Rust 工具链。

**必须覆盖的行为**（每条都要有用例）：

1. 超标报 `error`、`passed === false`；
2. **恰好 2500 行不报**；
3. 旁置 md 存在且四段齐全 → 降级 `warning`、`passed === true`、诊断仍在；
4. **旁置 md 内容不全 → 豁免不生效**，仍是 `error`，并报出缺哪一段；
5. **大纲不可用 → 仍报行数**、severity 仍是 `error`；
6. 截断规则与省略量文案；
7. **输出顺序稳定**（排序生效）。

---

## 十一、两条回归断言的改法

`tools/repo-check/test/repo-check.test.ts:29-34` 断言
`expect(checkLayout(process.cwd()).diagnostics).toEqual([])`——它是**「当前仓库布局零诊断」
的全局哨兵**，不只是「没有尺寸错误」。**保住哨兵，同时把债显式化**：

```ts
const KNOWN_OVERSIZED = [
  "core/rust/crates/xiao-bytecode/src/research/encode.rs",
  "core/rust/crates/xiao-syntax/src/parser.rs",
];
const sizeErrors = result.diagnostics.filter((item) => item.code === "A0-SIZE-001");
const others = result.diagnostics.filter((item) => item.code !== "A0-SIZE-001");
expect(others).toEqual([]);                                      // 原有哨兵，保住
expect(sizeErrors.map((item) => item.path)).toEqual(KNOWN_OVERSIZED); // 相等而非包含
```

用 `toEqual`（**集合相等**）而不是「每个都在名单里」：名单长了、短了都会红，
语义正好是「**这份名单只应缩短，不应增长**」。注释里写明这一点。
**不要删掉这条用例**，也不要用宽泛过滤把它变成永远通过。

`:36-41` 那条**要单独处理**：它断言 `result.result.passed === true`，只要两个文件
还是 error 就必然 `false`——**这条救不回来**。它真正要验的是「从子目录执行时报告根目录」，
所以保留 `expect(result.root).toBe(expectedRoot)`，把 `passed` 断言改成 `false`
并注释「已知债未清」，或直接删掉它（它本来就不是这条测试的目的）。

---

## 十二、文档同步清单（同一变更集内完成）

| # | 文件 | 动作 |
| --- | --- | --- |
| 1 | `00a-a0-workspace-and-checkers.md` | `:188-206` 规则总表加 `A0-SIZE-001` 一行（`A0-SIZE-` 是新前缀，与 `A0-LAYOUT-*` 同族，插在 `A0-LAYOUT-004` 之后）；`:165-182` 的子命令说明与 `all` 固定顺序；`:232` 补「A0-SIZE-001 是 `outlines` 的消费者」 |
| 2 | `docs/UseDocs/tooling/cli/repo-check.md` | `:22-25` 运行方式（`layout` 现在可能调 cargo）、`:32` 错误码段落加 `A0-SIZE-001`；**`status: verified` 的行为描述变了，按 00B 要重验** |
| 3 | `tools/repo-check/README.md`、`src/README.md` | 后者 `:13` 明文要求「每个模块新增代码时必须同步更新本 README」，其模块枚举 `manifest`/`workspace`/`layout`/`docs`/`report` 要加 `size` |
| 4 | `00-decisions.md:517-525` | 例外条款的适用面扩到「单文件行数」+ 写明旁置 md 四段要求（第八节） |
| 5 | `12-tests-and-milestones.md:43-59` | 照 `missing_docs`（`:57-59`）的先例**追加**第 10 条退出条件，**不要改写**已完成的 A0 条目 |
| 6 | `docs/UseDocs/tooling/cli/doc-coverage-rust.md`、`tools/doc-coverage/src/README.md` | 门禁成为 `outlines` 的**第一个消费者**（此前后者只是「预留通道」）；TS 侧同形类型要与该页交叉引用，否则两边会漂移 |
| 7 | 根 `README.md:26, 59-67` | `check` 从此依赖 Rust 工具链、且当前为红，写明前置条件与原因 |

`docs/module-registry.json` **不用改**：登记校验是**目录级**前缀匹配
（`layout.ts:87-105`），`ts.repo-check` 的 `code: ["tools/repo-check"]` 已覆盖新增的 `src/size.ts`。

---

## 十三、提交切分

按可独立验证的单元分三次，**每次提交后 `cargo test` / `clippy` / `fmt` / `bun test` 全绿**
（`bun run check` 在本批结束时**预期为红**，见下）：

1. **判定与接线**：`paths.ts` 返回 `files`、新建 `size.ts`（行数口径 + `A0-SIZE-001`）、
   `checkLoadedLayout`/`checkLayout` 改 async、注入式提供者、七类 fixture 用例、
   两条回归断言改造。**这是核心**，此时诊断的 `message` 只有一行摘要。
2. **大纲渲染**：`Diagnostic.details`、`report.ts` 三处渲染、`spawnSync` 加 timeout、
   大纲不可用时的 `A0-PARSER-001` 路径、TS 的 `outlineTypeScriptFile` 与它的边界用例。
3. **豁免与文档**：`exemptionMissingSections`、`00-decisions` 的例外条款、
   第十二节的七处文档同步。

**若中途必须停**：第 1 次提交本身完整可验证，门禁已经生效（只是没有大纲可读）。

---

## 验收

命令见 09R2D 文档 2.4 节。**关键验收不是「测试通过」**：

1. **两个入口都生效**：`bun run check:layout` 与 `bun run check` 都报出 `A0-SIZE-001`，
   且**不重复**报同一条。
2. **行数对得上 `wc -l`**：两个超标文件的诊断行数是 **3847** 与 **3040**。
3. **大纲可达且为树**：对真实超标文件跑一次，确认 ① 顶层项与 `impl` 块都在；
   ② 字段、枚举变体、方法下钻到位；③ 每行都有名字与定义句；④ 节点行数满足
   `line + lines - 1 == end_line`。
4. **`line` 为空、`subject` 不等于 `path`**：终端输出不出现同一路径打印两遍。
5. **SARIF 没被撑坏**：`--format sarif` 下 `message.text` 是**一行**，大纲在 `properties` 里。
6. **区分度**：撤掉判定 → 用例必须失败；撤掉旁置 md 的四段校验 → 「内容不全」用例必须失败；
   把 `spawnSync` 的 timeout 撤掉 → 超时用例必须失败。
7. **豁免四态**：无旁置 md（error）/ 齐全（warning、passed）/ 缺段（error）/ 空文件（error）。
8. **恰好 2500 行不报**、2501 行报——边界两侧都有用例。
9. **文档同步到位**：`00a` 规则总表有 `A0-SIZE-001`；UseDocs 能查到它。
10. **明确记录**：`bun run check` **预期为红**（两个已知超标文件），
    在**提交正文与文档里写清这是有意为之、由谁接手**。**不要**为了让门禁变绿而给这两个文件
    写旁置 md——它们的预期是拆分。

---

## 不要重复做的事

- **不要重写 00D 的大纲遍历**：`outline_item` / `node` / `declaration_head` 已交付且被测试锁定，
  本批只**消费**它们。
- **不要把 `impl` 块塞进 `declarations`**：覆盖率只读 `declarations` 是有意的，
  合并会平白改变一个已验证的 100% 数字。
- **不要统一 `checker.ts:174` 的重复遍历**（理由见「不负责」）。
- **不要发明新的规则码**：大纲不可用复用 `A0-PARSER-001`。
- **不要把完整大纲写到文件**：`00a:163` 冻结了「只读且不自动修复」。
- **不要让门禁在没有 Rust 工具链时静默通过**：`A0-SIZE-001` 的 severity 恒为 `error`。
- **不要给 `tools` 或任何目录加白名单**：`discoverSourceDirectories` 本就覆盖 `tools`，
  加白名单等于制造第二个豁免口。
- **不要为了 `bun run check` 变绿去写旁置 md**：那两个文件的预期是拆分。

---

## 相关页面

- [A0. 工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md)
- [00D. Rust AST 适配器协议 v2 修正交接](00d-doc-adapter-protocol-v2-fixes.md)
- [00. 决策基线](00-decisions.md)
- [12. 测试与开发里程碑](12-tests-and-milestones.md)
- [仓库完整性检查](../UseDocs/tooling/cli/repo-check.md)
- [Rust AST 适配器](../UseDocs/tooling/cli/doc-coverage-rust.md)

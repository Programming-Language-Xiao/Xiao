# 00D. Rust AST 适配器协议 v2 修正交接

> **实现状态（2026-09-19）**：本文约定的跨度语义、大纲行数口径、定义句测试、
> `outline` 请求开关及协议文档同步均已完成；后续可在此基础上实现 `A0-SIZE-001`。

> `0e9a42b` 把 `xiao-doc-coverage-rust` 的协议从 v1 升到 v2，为「单文件过长必须解耦」的新门禁
> 预备结构大纲。该提交**六项门禁全绿**（已逐条复跑），大纲本身也**真的可用**
> （实测 `b06_runtime.rs` 的 `span` → `line 19` / `fn() -> SourceSpan`，并下钻到字段与枚举变体）。
>
> 但审核发现：**同批进入的 `Declaration.end_line` 是死字段**，`OutlineNode.lines` 与
> `line`/`end_line` 三者口径互斥，另有成文契约未同步。本文记录**已核实的六条**，交给接手 Agent 修正。
>
> **本批只修这六条**，不重写大纲遍历，不实现门禁本体。
> 唯一一处协议改动是给**请求体加一个可选开关**（缺陷 6，用户已裁决取 B），
> 响应形状与版本号都不动。

## Agent 交接上下文

### 接手前提

1. [A0. 工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md) —— A0 规则码与检查器契约；
   本批涉及的 `A0-PROTOCOL-001` / `A0-PARSER-001` 都在这里。
2. [00B. UseDocs 同步政策](00b-usedocs-policy.md) —— 「协议变更必须同批更新契约文档」的依据（缺陷 5）。
3. [00C. 文档 lint 接线实现交接](00c-doc-lint-wiring.md) —— 同一条工具链的上一轮交接，门禁口径与格式沿用。
4. [Rust AST 适配器](../UseDocs/tooling/cli/doc-coverage-rust.md) —— **本批要改的成文契约本体**。
5. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 门禁命令、区分度验证、
   单一来源原则、解耦约束的主表在这里，**本文不复制**。

### 本批交付与不负责

**交付**：`Declaration.end_line` 的真实语义、`lines` 口径一致、`declaration_head` 的文档与实现对齐、
协议契约文档同步、**请求体的大纲开关**（缺陷 6，已裁决取 B）。

**不负责**：单文件 2500 行的门禁本体（`A0-SIZE-001`）——**本批是它的前置**，见最后一节。
不重写大纲遍历，不动 `maxBuffer` 补丁，**不升协议版本号**（理由见缺陷 6）。

### 涉及文件

| 文件 | 本批动作 |
| --- | --- |
| `core/rust/crates/xiao-doc-coverage-rust/src/lib.rs` | 缺陷 1、3、4 的主战场 |
| `docs/UseDocs/tooling/cli/doc-coverage-rust.md` | 缺陷 5 |
| `tools/doc-coverage/src/{rust-adapter.ts,types.ts}` | 透传缺陷 6 的 `outline` 开关；字段语义变化波及校验/注释时跟进 |
| `tools/doc-coverage/README.md`、`tools/doc-coverage/src/README.md` | 缺陷 5 的顺带检查 |

---

## 缺陷 1（严重）：`Declaration.end_line` 恒等于 `line`，是死字段

### 症状

协议 v2 的头号新增字段 `end_line`，对**全部真实声明**都等于 `line`，不携带任何信息。
门禁正要用它判断块大小——**在假数据上盖门禁就是补丁叠补丁**。

### 证据（全仓实测，141 个 Rust 文件 / 3850 条声明）

| 项 | 数量 |
| --- | --- |
| `line == end_line` 的声明 | **3647** |
| `line != end_line` 的声明 | 203 |
| 其中：每文件一条的 `crate` 伪声明 | 134 |
| 其中：`pub use` 重导出 | 69 |
| 真正的 `mod` 声明（112 条）里 `line != end_line` 的 | **0** |

也就是说，12 个 `push()` 调用点里只有 2 个（`crate` 伪声明、`visit_item_use`）恰好传了整项跨度，
其余全是废数。

### 根因

12 个调用点改的是（`lib.rs:181/193/205/217/229/241/253/265/277/297/310/320`）：

```rust
item.sig.ident.span().start().line   →   item.sig.ident.span()
```

**`X` 始终是标识符**（`item.sig.ident` / `item.ident` / `identifier`）。
标识符的跨度终点与起点**同行**，所以 `lib.rs:348` 的 `span.end().line` 必然等于 `lib.rs:343` 的
`span.start().line`。

### 同一个提交里的自相矛盾

`core/rust/crates/xiao-runtime/tests/b06_runtime.rs` 的 `span()`，**两条通道给出不同答案**：

```
大纲通道（node()，传的是 item.span()）：    line 19 .. end 21   ← 正确
声明通道（push()，传的是 ident.span()）：   line 19 .. end 19   ← 假
```

### 文档与实现相反

- `lib.rs:42-44` 字段文档：「取的是**整个项**的跨度，不是标识符的跨度；`line` 同样是项起始行。」
- `lib.rs:331-332` 的 `push()` 文档：「`span` 必须是**整个项**的跨度……用标识符跨度会少算属性和 `pub`，
  定位也不对。」

**`push()` 被自己的文档钉死在它全部 12 个调用者都没做到的做法上。**

### 修法

**不要**简单把调用点换成 `item.span()`——整项跨度会把 `///` 属性算进去（`///` 就是 `#[doc]`），
把 `line` 推到文档注释那一行，**正是 `0e9a42b` 声称要修掉的问题**。

正确做法：承认这是**两个概念**，照同文件里 `node()`（`lib.rs:447-458`）已有的设计办——

- `line` = **声明自身**起点（`header` 跨度）→ **保持现状**，它是对的（实测 `span` 报 19，与验收例子一致）
- `end_line` = **整项**终点（`full` 跨度）→ 从 `item.span()` 取

即 `push()` 改为接收两个跨度（`header: Span, full: Span`），调用点传：

| visitor | header | full |
| --- | --- | --- |
| `visit_item_fn` | `item.sig.ident.span()` | `item.span()` |
| `visit_item_struct` / `_enum` / `_trait` / `_mod` / `_const` / `_static` / `_type` / `_union` | `item.ident.span()` | `item.span()` |
| `visit_field` | `identifier.span()` | `field.span()` |
| `visit_impl_item_fn` / `visit_trait_item_fn` | `item.sig.ident.span()` | `item.span()` |
| `visit_item_use` | `item.use_token.span` | `item.span()` |

`visit_item_use` 现在只传了整项跨度，**要补上 header**，否则带属性的 `use` 会把 `line` 指到属性行。

`crate` 伪声明（`lib.rs:139-144`）不受影响：它的 `line` 是 1、`end_line` 是文件行数，本来就是对的。

改完后**逐条重写** `end_line` 与 `push()` 的 Rustdoc，让描述与实现一致。

### 验收

- 全仓实测 `line != end_line` 的声明数**显著大于 203**，且不再局限于 `crate` 与 `pub use`。
- `b06_runtime.rs` 的 `span` 在声明通道报 `19..21`，与同文件大纲通道一致。
- 新增一条断言「多行项的 `end_line > line`」的单元测试。

---

## 缺陷 2（记录）：提交正文的三处事实陈述不成立

`0e9a42b` 的正文有三处与代码/历史不符。**不改写历史**，但接手者必须知道，
免得把它们当既有事实继续引用：

| 正文原话 | 实际情况 |
| --- | --- |
| 「`line` 改为**声明自身**的行——原来是标识符的行」 | `line` 的语义**没有变**，仍是标识符跨度的起点（`lib.rs:343`） |
| 「实测例子：`b06_runtime.rs` 的 `span` 现在报 line 19 而不是 18」 | 父提交 `0e9a42b^:lib.rs:128` 就是 `item.sig.ident.span().start().line`，**本来就是 19**；该文件第 19 行正是 `fn span()` |
| 「这样 4 行的函数不会把上面的文档注释算进自己的行数」 | 实现恰好相反，见缺陷 3 |

**后续提交的正文只写核实过的事实。**引用行号或断言行为差异前，先在父提交上核对一遍。

---

## 缺陷 3（中）：`lines` 与 `line`/`end_line` 三者推不出彼此

`node()`（`lib.rs:453-455`）三个字段用了两种口径：

```rust
line: header.start().line,                                      // 排除属性/文档注释
end_line: full.end().line,                                      // 含属性
lines: full.end().line.saturating_sub(full.start().line) + 1,   // 含属性/文档注释
```

实测三例（同一份输出）：

```
L19..21  n=4   ← 第 4 行是第 18 行的 /// 文档注释；21-19+1 = 3
L39..39  n=2   ← 单行 static 报 2 行
L54..56  n=4   ← 56-54+1 = 3
```

消费者按 `line + lines - 1` 推算区域会算到 22，而真实区域止于 21。**三段读数互相矛盾**，
而字段文档 `lib.rs:70-71` 只写「覆盖行数，含子节点」，没说含不含上方文档注释。

### 修法

**统一到一个口径**，推荐：`line` 保持声明自身行（与验收例子一致，也是文档覆盖率报错要的定位），
`lines` 改为 `end_line - line + 1`（saturating），语义是**该声明自身占的行数，不含上方文档注释**。
并在 `OutlineNode.lines` 与 `Declaration.end_line` 的 Rustdoc 里写明这个定义。

**不要**保留「含属性」的计数却把 `line` 留在声明行——那正是现在这个两头不占的状态。
若确实需要属性行数，另开字段，不要复用 `lines`。

### 验收

- 全仓实测每条大纲节点满足 `line + lines - 1 == end_line`。
- 用带 `///` 的多行项夹具写一条单元测试，锁住该恒等式。

---

## 缺陷 4（中）：`declaration_head` 的文档与实现不符

`lib.rs:563-571` 的文档写「取到第一个 `{` / `;` / **`=`** 为止」，实现是：

```rust
.split(['{', ';'])      // ← 没有 '='
```

同批输出里就摆着证据：

```
源码：   static DROP_CALLS: AtomicUsize = AtomicUsize::new(0);
定义句： 'static : AtomicUsize = AtomicUsize::new(0)'
```

`=` 与其后内容全都在，且删名后留下悬空的 `static :`。

### 修法（二选一，并在代码里写明选了哪个）

- **(a) 让文档说实话**（最小改动）：文档改成「取到第一个 `{` / `;` 为止」。
  `=` 之后的内容对 `static` / `const` / `type` 是有信息的，保留反而更好。
- **(b) 让实现符合文档**：把 `=` 也加进终止符。代价是 `struct Foo<T = u32> {` 会被截成 `struct Foo<T`。

**建议 (a)。**无论选哪个，都**要补单元测试**把 `fn` / `static` / `struct` / 多行签名四种形状锁住——
这个函数目前**一条测试都没有**。

顺带（可选、低优先级）：删名后清掉悬空分隔符（`static : AtomicUsize` → `static: AtomicUsize`）。
它只是显示串，改了要有测试。

### 一个已知的理论风险（本仓未触发，不要为它改设计）

`head.find(name)` 取的是**子串首次出现**，不是「名字这个记号」。若同一行的属性里先出现同名子串
（如 `#[doc = "render"] pub fn render()`），会删错一处。我在本仓 141 个文件里**没有找到真实触发实例**，
故**不列为缺陷**。若顺手改成按记号边界匹配也可以，但不要为它引入新依赖或复杂解析。

---

## 缺陷 5（中）：协议升到 v2，成文契约没跟着改

`docs/UseDocs/tooling/cli/doc-coverage-rust.md:24` 在修复前写着（**原文照录，是本节缺陷的证据，不要改写或删除**）：

> 请求和响应都带有 `protocol_version`，当前版本为 `1`。响应固定包含 `declarations` 与 `errors` 数组……

- 版本号错：实际是 `2`（`lib.rs:13`）。
- `outlines` 数组**完全没提**，而它是 v2 的主要新增。

一个把版本号写错的协议文档比没有文档更坏——下一个人会信它。按
[00B. UseDocs 同步政策](00b-usedocs-policy.md)，协议变更必须同批更新契约文档，**这条属本批必修**。

### 修法

更新该页「协议边界」小节：版本号改 2、补齐 `outlines` 数组、简述 `OutlineNode` 字段
（`kind` / `name` / `line` / `end_line` / `lines` / `signature` / `source_line` / `children`），
并说明**覆盖率判定只读 `declarations`，大纲是独立通道**。
顺带检查 `tools/doc-coverage/README.md` 与 `tools/doc-coverage/src/README.md` 有无同样的版本号陈述。

### 验收

**只扫契约面，不扫全 `docs/`**：

```bash
grep -rn '当前版本为 `1`' docs/UseDocs/ tools/*/README.md tools/*/src/README.md
```

结果为 0，且该页能查到 `outlines` 与 `OutlineNode`。

> **判据必须限定在契约面，这条踩过坑。**「扫全 `docs/` 且结果为 0」与「本节保留原文引用」
> **互斥**：00D 自己就在 `docs/` 下，一保留引文，那条 grep 就永远不可能为 0。
> 实测 `8a916e3` 时两处命中，其中一处正是本文 222 行。
> 一个自败的判据会逼着后来者删证据去满足它——本仓已经发生过一次。
> 缺陷档案保留缺陷原文是**必须的**，所以判据的范围要跟着定，不能反过来。

---

## 缺陷 6（中）：大纲被绑死在覆盖率调用上，绿态也付全量代价

> **用户已裁决：取 B——加请求开关。** 本节是可执行的实现要求，不是选项清单。

### 实测

全仓扫描（141 个 Rust 文件）的 v2 响应：**1,560,394 字节、5,318 个大纲节点**。
而唯一的消费者 `tools/doc-coverage/src/checker.ts:49-50` **只读 `declarations`**：

```ts
const rust = scanRustFiles({ root, files: rustFiles, adapterPath: options.rustAdapter });
declarations.push(...rust.declarations);
```

这正是 `0e9a42b` 里 `maxBuffer: 256 * 1024 * 1024` 那个补丁的成因（1.56 MB > Node 默认 1 MiB）。
**补丁本身是对的**（理由也写得对，保留），但它掩盖了代价而不是消除代价：
每次覆盖率检查都背着整仓大纲，而原本的设想是**只在确实超标时才提取大纲**（正常绿态不 spawn cargo）。

### 修法：请求体加 `outline` 开关，缺省不产出大纲

**请求加一个布尔字段，缺省 `false`**——不请求就不算，这才是消除代价而不是掩盖代价：

```rust
/// 是否在响应中返回结构大纲。
///
/// 缺省为 `false`：大纲的体积远大于声明（实测整仓 1.56 MB），
/// 只有需要判断文件结构时才值得付这个代价。
#[serde(default)]
pub outline: bool,
```

**三条硬要求：**

1. **不升 `protocol_version`。**这是同一个协议的可选请求参数，**响应形状完全不变**：
   `outlines` 数组**永远存在**，未请求时是 `[]`。`validateRustAdapterResponse`
   （`rust-adapter.ts:126`）要求它必须是数组，所以**可以清空、不可以省掉**——
   省掉字段会让校验直接失败。不升版本也就避免了又一次两侧版本联动。
2. **字段名是 `outline`（布尔，单数），别和响应的 `outlines`（数组，复数）混**。
   请求体里加的是开关，响应里仍是数组。
3. **TS 侧 `scanRustFiles` 加同名可选参数**（如 `outline?: boolean`），透传进请求体；
   现有的 4 条提前返回路径继续带 `outlines: []`（`rust-adapter.ts:34/51/69/84`），不受影响。

**调用约定**（写进 UseDoc，也是门禁的实现依据）：

| 调用方 | `outline` | 理由 |
| --- | --- | --- |
| `checker.ts` 的覆盖率检查（`checker.ts:49`） | **不传**（即 `false`） | 只读 `declarations`，不需要大纲 |
| 门禁发现超标文件后取大纲 | `true` | 只为确实超标的文件请求 |

**`maxBuffer: 256 * 1024 * 1024` 保留**：它是关掉开关也仍然需要安全带的地方
（将来单文件请求大纲时体积依然可观），不要因为它不再被触发就删掉。

### 验收

- 全仓覆盖率调用（141 个 Rust 文件、不传 `outline`）的响应**不再接近 Node 默认 1 MiB**，
  且其中 `outlines` 为 `[]`、`declarations` 与改动前逐条一致。
- 传 `outline: true` 时，大纲内容与缺陷 1/3 修正后的结果一致。
- 两侧各有一条测试：**不传时 `outlines` 为空数组而不是缺字段**（缺字段必须被
  `validateRustAdapterResponse` 拒绝——那条校验保持不动，可作反向证据）。
- `docs/UseDocs/tooling/cli/doc-coverage-rust.md` 里能查到 `outline` 的缺省值与上面这张调用约定表。

---

## 硬性约束

门禁、区分度验证、单一来源原则、解耦约束**沿用 09R2D 文档第二章**（不重复）。本批额外注意：

1. **协议两侧必须同批改**：`lib.rs` 的 Rust 结构体与 visitor、
   `tools/doc-coverage/src/rust-adapter.ts` 的映射与 `validateRustAdapterResponse`、
   `src/types.ts` 的协议类型、两侧测试。**只改一侧会让仓库停在断状态**——
   `0e9a42b` 之前就发生过一次：Rust 侧升到 v2 而 TS 仍发 v1，覆盖率工具整个挂掉。
2. **`end_line` 语义变真后不要顺手收紧校验**：`rust-adapter.ts:137` 只校验它是 `number`，
   语义修正后依然够用。
3. **`missing_docs` 已接线**：`cargo clippy --workspace --all-targets -- -D warnings`
   会因新增公开项缺文档直接失败。新增公开项当场写 Rustdoc。
4. **不要用 `#[allow]` 掩盖警告**，也不要把断言放宽来让门禁变绿。
5. **新增 DevDocs 页面必须在 `docs/DevDocs/README.md` 有人链**，否则 `A0-DOCS-001`
   （`a0.docs.orphan_page`）会让门禁失败。

---

## 提交切分

按可独立验证的单元分三次，**每次提交后六项门禁全绿**：

1. **`lib.rs` 语义修正**（缺陷 1 + 3 + 4）：`push()` 双跨度、`lines` 口径统一、
   `declaration_head` 文档与测试。**这是核心**，后两次都是收尾。
2. **大纲开关**（缺陷 6）：请求体加 `outline`、TS 侧透传、两侧测试。
3. **协议契约同步**（缺陷 5 + 6 的 UseDoc 段落）：`doc-coverage-rust.md` 与工具 README。

**若中途必须停**：第 1 次提交本身完整可验证；第 2 次之后调用方已经可以按需取大纲。

---

## 验收

命令见 09R2D 文档 2.4 节。本批的关键验收**不是「测试通过」**：

1. `line != end_line` 的声明数**显著大于 203**，且不再局限于 `crate` 与 `pub use`。
2. `b06_runtime.rs` 的 `span` 在**声明通道**报 `19..21`，与同文件**大纲通道**一致——两条通道不再打架。
3. 每条大纲节点满足 `line + lines - 1 == end_line`。
4. `declaration_head` 的文档描述与实现一致，且有测试锁住四种形状。
5. **大纲按需**：不传 `outline` 的全仓覆盖率响应里 `outlines` 为 `[]`；
   传 `outline: true` 时大纲完整。响应**始终**带 `outlines` 字段（清空可以，删掉不行）。
   实测口径：141 个 Rust 文件 **1,560,394 → 605,535 字节**。
   **注意 605,535 字节 ≈ 592 KB，仍是 1 MiB 默认上限的 58%**——那是 v1 本来的体积
   （每声明重复整个文件路径，约 157 字节/条），不是「远离上限」。
   故 `maxBuffer` 必须保留，且**仓库规模翻倍时这 592 KB 会再次越过 1 MiB**。
6. 契约面内 `grep -rn '当前版本为 \`1\`' docs/UseDocs/ tools/*/README.md tools/*/src/README.md` 为 0，
   且该页能查到 `outlines`、`OutlineNode` 与 `outline` 开关。**判据范围限定在契约面的原因见缺陷 5**。
7. **六项门禁全绿**：`bun run check`、`bun run check:coverage`、`bun test`、
   `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、
   `cargo fmt --all -- --check`。

---

## 不要重复做的事

- **不要重写大纲遍历**：`outline_item` / `node` / `outline_fields` / `outline_variants` 是对的
  （实测嵌套到字段与枚举变体、行号指向声明行），本批不动它们的结构。
- **不要把 `impl` 块塞进 `declarations`**：覆盖率只读 `declarations` 是**有意的**，
  合并会平白改变一个已验证的 100% 数字。
- **不要改写 `0e9a42b` 的历史**：正文里那三处陈述无法回填，只约束后续提交（缺陷 2）。
- **不要动 `maxBuffer` 补丁**：256 MiB 的调整是对的，保留——即使加了开关，将来单文件请求大纲时它仍是安全带。
- **不要升 `protocol_version`**：加的是可选请求参数、响应形状不变，升版本只会平白触发一次两侧联动。
- **不要顺手实现 2500 行门禁**：本批是它的前置，见下。

---

## 本批之后的依赖：单文件 2500 行门禁（`A0-SIZE-001`）

本批**不是终点**。计划中的下一阶段是「单文件超过 2500 行即报错，并附该文件的树形大纲」，
规则码 `A0-SIZE-001`，`severity: "error"`。

已核实：**写 `error` 就足以硬失败**——`tools/repo-check/src/report.ts:101` 的 `mergeResults`
与 `cli.ts:87-92` 的退出码都只看 `severity`，不用改 reporter。

**它直接依赖本批的缺陷 1 与缺陷 3**：门禁要用 `end_line` 判断块大小、要把 `line` / `lines`
渲染给接手者，**在假数据上盖门禁就是补丁叠补丁**。所以先修本批。

已核实的前提（接手门禁时可直接用）：当前只有 2 个文件超标——`encode.rs`（3847 行）、
`parser.rs`（3040 行），按任何口径都超标；`rust-analyzer` 在本机只是 rustup 垫片、组件未安装，
大纲必须走仓内的 `syn` 适配器。

门禁本体、TS 侧结构大纲、豁免机制（文件旁的 `<文件名>的硬耦合需要的说明.md` 降级为 `warning`）
的完整设计与验收标准见 [00E. 单文件行数门禁交接](00e-file-size-gate.md)。

---

## 相关页面

- [A0. 工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md)
- [00B. UseDocs 同步政策](00b-usedocs-policy.md)
- [00C. 文档 lint 接线实现交接](00c-doc-lint-wiring.md)
- [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md)
- [文档覆盖率检查](../UseDocs/tooling/cli/doc-coverage.md)
- [Rust AST 适配器](../UseDocs/tooling/cli/doc-coverage-rust.md)

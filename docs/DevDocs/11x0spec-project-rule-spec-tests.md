# 11X0-SPEC. `X0-SPEC-001` 规格测试债

> **收口记录（2026-09-24）**：本债项已完成可执行范围的实现。`06-modules` 的两个历史
> 夹具已接入真实模块分析入口，并按执行后暴露的确定性诊断顺序修正；新增 `07-error-control`、
> `08-ir`、`11-config` 夹具和对应 harness，全部同步 `docs/module-registry.json`。同时新增
> `tests/spec` 的登记+真实加载者门禁，修正 `04-types` 与 `05-containers` 的 README 登记偏差。
> 阶段 10 经论证不新增规格夹具：其剩余规则属于双模式一致性或已有 `09-bytecode` 覆盖，
> 不重复制造第四套 schema；11-config 固定配置语义，CLI 的保注释/原子写回继续由既有
> `editor.test.ts` 单元层覆盖；11X0 协议夹具原已双向执行，无需改动。

> **债项原文**（`12-tests-and-milestones.md:717-720`）：X0 第 3 条「从阶段 01 到本阶段
> 已经实现的所有"已确定"规则都有自动化规格测试」另行登记为 `X0-SPEC-001` 规格测试债，
> **不属于平台复现批次**。
>
> **验收判据只有一条**（`11x0p1:352-354`）：
> **「真正的门槛不是'建目录'而是'有执行入口'」**——`tests/spec/06-modules/`
> 就是现成反例：目录和夹具都在，**但没有任何测试代码读取它**。
> 若只建 07/08/10/11 的 JSON 而不接测试，等于复制同一笔债。

## 一、Agent 交接上下文

### 接手前提

1. [12. 测试与开发里程碑](12-tests-and-milestones.md) **`:7-37`（六层测试分层）与
   `:717-720`（本债项登记）** —— §四的边界判据全部来自这里。
2. [11X0-P. 跨平台复现](11x0-platform-reproduction.md) **§8.2**（`:386-400`）
   —— 债项的**来源与范围裁定**。注意 `:420` 明令「**不做 X0 第 3 条以外的规格测试扩张**」。
3. [11X0-P1. 跨平台复现收口](11x0p1-platform-reproduction-closure.md) **§9.1**（`:350-356`）
   —— **本债项最重要的约束**：「门槛不是建目录而是有执行入口」。
4. [00A. 工程框架与目录布局](00a-project-layout.md) **`:155-157`**
   —— `tests/spec` 的宪章：**「语法、类型、配置和格式的正反例」**。
   注意**「配置」在里面**——这是 11 阶段那一笔债的直接依据。

### 现状盘点（2026-09-24 实测）

```text
夹具总量   54 个 JSON、2533 行、76302 字节
执行入口   54/54 被加载（100%）
收口结果   06-modules 已接线；07/08/11 新增夹具均有真实加载者；无未执行夹具
格式       三代互不兼容的 schema 并存（见 §五）
```

**十一个 spec 目录的执行入口**（判据表，逐项带 `文件:行号`）：

| 目录 | 夹具 | 执行入口 | 状态 |
| --- | --- | --- | --- |
| `01-lexical` | 16 | `xiao-syntax/tests/lexical_snapshots.rs:60,69,…195` | ✅ 16/16 |
| `02-parser` | 5 | `xiao-syntax/tests/parser_snapshots.rs:215,224,…251` | ✅ 5/5 |
| `03-expression` | 2 | `xiao-syntax/tests/p1_expression.rs:442,451` | ✅ 2/2 |
| `04-types` | 1 | `xiao-syntax/tests/p2_snapshots.rs:34` | ✅ 1/1 |
| `05-containers` | 10 | `xiao-types/tests/c0c1_snapshots.rs:122,…158` 等四份 | ✅ 10/10 |
| `06-modules` | 2 | `xiao-modules/tests/d0_modules.rs:454` | ✅ 2/2 |
| `07-error-control` | 2 | `xiao-types/tests/control_flow_snapshots.rs:72` | ✅ 2/2 |
| `08-ir` | 1（12 个 case，含七类选择器） | `xiao-ir/tests/structure_snapshots.rs:222` | ✅ 1/1 |
| `09-bytecode` | 8 | `xiao-vm/tests/r2_vectors.rs:250,…334` | ✅ 8/8（79 向量） |
| `11-config` | 2 | `xiao-config/tests/spec_snapshots.rs:60` | ✅ 2/2 |
| `11x0-protocol` | 5 | Rust `x0_a_protocol.rs:10,…22` + TS `protocol.test.ts:8,…69` | ✅ 5/5 **双向** |

**范围**（债项原文）：补 **07、08、10、11/11X0** 四个阶段的规格测试。
**但 §四会说明：这四个阶段的"可夹具化"程度差异极大，不能一刀切。**

---

## 二、判据：什么算「完成」

**不是「目录存在」，是「有测试代码读取它」。**

具体到可核验的形式：**每个新夹具都能指出「哪个文件的哪一行加载了它」**，
且该 harness **在夹具被删掉时会失败**（沿用 09R2D 的
「撤掉实现 → 用例必须失败 → 还原 → 通过」）。

⚠️ **一条门禁缺口**：`tools/repo-check/src/docs.ts:301` 只做 `existsSync(absolute)`——
**检查登记路径是否存在，不检查它是否被任何测试加载**。
所以 `06-modules` 在**四次门禁改版后依然存活**。
**本债项的第 4 份会继续复制，除非补一条门禁**（§六.4）。

---

## 三、`06-modules`：现成反例，也是**最好的先例**

### 3.1 它是遗漏，不是有意留待

证据链（三条）：

1. `git log -- tests/spec/06-modules` 只有 `4ec8a60 feat: implement 05 local modules and imports`。
   该提交**在同一笔里**同时新增了 `README.md` + `valid.json` + `errors.json`
   **和**两个测试文件 `xiao-syntax/tests/d0_imports.rs`、`xiao-modules/tests/d0_modules.rs`——
   **它们从来就是一批做的，只是没接线**；
2. `docs/module-registry.json:19,24` 把 `tests/spec/06-modules` 登记进两个 crate 的
   `tests` 数组，**登记表把它当测试资产**；
3. **那两个测试文件不读 JSON**：`d0_imports.rs:26` 用
   `SourceFile::from_text("import net.http, net.http as http\n")`，
   `d0_modules.rs:19-57` 用 `TempProject` 临时目录——
   两个文件里 `grep "json\|include_str\|tests/spec"` **零命中**。

### 3.2 ⭐ 先例：接线本身会**立刻产出真价值**

`05-containers` 有过**同型缺陷**，修的过程留在了 `git` 里（提交 `59a0317`）：

> 四个容器/选择器快照在 module-registry 里登记为有效契约，却**从未被任何测试加载**，
> 快照与实现的分歧因此**长期被静默掩盖**。补上 `c0c1_snapshots.rs` 后**立刻暴露两处**。
>
> - `c0-errors` 的 `unsupported-range` **已被 C1 取代**——原期望不可能再成立。
>   改写为 `unsupported-dict-range`……「**保留一条永不成立的期望只会让契约继续腐烂**」；
> - `c1-errors` 的 `random-seed-invalid` 源码含两个非法调用，实现正确地报了两条诊断，
>   **是快照当初只记了一条**。

而且修的过程里**顺带暴露了一处实现缺陷**：同一诊断编号在同一源码位置被重复上报。

**所以本债项不是形式主义**——「有夹具无入口」会让契约腐烂且**被静默掩盖**。
`06-modules` 很可能也藏着分歧，**接线会把它挤出来**。

---

## 四、⚠️ 四个阶段的**可行性差异极大**（本债项最需要先说清的事）

调研的结论是：**07/08/10/11 不能按"各建一个目录"一刀切**，因为它们"已确定的规则"
落在**不同的测试层**。照抄会写出**假装是规格测试、实际属于别层**的东西。

| 阶段 | 真正能进 `tests/spec/` 的 | 增量价值 | 主要障碍 |
| --- | --- | --- | --- |
| **07** | `try`/`catch`/`finally`/`raise` 的语法与静态边界、展开顺序、`suppressed` 不覆盖主错误 | 中 | ⚠️ **`07:303` 的最终顺序未冻结**（见 §七.2） |
| **08** | AST/IR 快照（`08:132` 要求七类选择器各有独立快照）、类型路径声明 vs 读取索引、`IrValidator` 的 `X08-IR-001/002` | **高** | **工作量最大**：要把 `u0_ir.rs` 的内联期望**重构成夹具驱动** |
| **10** | 固定宽度语义（`int`/`float` 64 位、`sint` 32 位）、溢出进统一错误路径、`const` 只读存储 | ⚠️ **最低** | **大部分已确定规则属于「双模式一致性」层**（见下） |
| **11** | `config.xiao` 的声明式子集、`xiao config` 结构化写入、`[language].locale` 优先级 | **高** | 需新 harness（命令级夹具是新形状） |
| **11X0** | 协议**已完成**；`xiao test` 的发现/排序/退出码语义**可选** | 低 | — |

### 4.1 ⚠️ 阶段 10 的陷阱：它的规则**大多不属于规格测试层**

`10` 的验收项里最像"规格测试"的是 `10:54`（同一 IR 同时跑字节码与原生对比）和
`10:71`（资源释放、容器路径、错误类别与字节码模式一致）——**但那两条的本质是
「双模式一致性」**，`12:27-29` 把它**单独定义为一层**，
已有 `tests/differential/` 与 `tests/benchmarks/reports/windows-native-semantic-differential.json`。

**把它写成 `tests/spec/` 夹具会**：① 与差分层重复；② 违反
`09-bytecode/README.md:6-10` 的「**禁止为某一型修改期望**」。

**10 真正能进 spec 的只有与机型无关的语言语义**（`:27`/`:31`/`:41`），
**而固定宽度与溢出已被 `09-bytecode/errors.json` 部分覆盖**。

**所以**：`10` **动手前必须先论证必要性**，否则会产出"看起来补了、实际是重复"的夹具。

### 4.2 阶段 08 的真实工作量：是 **refactor** 不是新增

`xiao-ir/tests/u0_ir.rs`（7 个 `#[test]`）已经在做 `to_json`/`from_json` 往返，
**但期望值是内联的 Rust 字符串**，没有 `tests/spec/08-*` 目录。
**所以 08 的补法是：先把内联期望抽成夹具，再让 harness 读它。**

这与 `06-modules` 的形态**相反**（06 是"夹具在、无人读"；08 是"harness 在、无夹具目录"）。

### 4.3 建议的优先级

按**「可控性 × 增量价值」**排序：

1. **`06-modules` 接线**——**零夹具新增，纯接 harness**，且 §3.2 的先例说明它会**立刻暴露分歧**。**这是本债项的第一笔，也是见效最快的一笔。**
2. **`11` 的 `config.xiao` 声明式子集**——`00a:155` 明确把「配置」写进 `tests/spec` 宪章，
   且规则已完整冻结。现状是 `xiao-config/tests/d05_config.rs` **全内联**。
3. **`08` 的 AST/IR 快照**——`08:132` 要求最明确，但要重构 `u0_ir.rs`。
4. **`07` 的错误控制流**——规则冻结，但要把三处内联测试（`f04_functions.rs`×2、`a06_lifetime.rs`）改成夹具驱动。
5. **`10`**——**先论证再动手**（§4.1）。

**不要按 07→08→10→11 的顺序做**——那是阶段编号顺序，不是价值或可控性顺序。

---

## 五、格式：**别发明第四套**

现有的 `tests/spec/` 有**三代互不兼容的 schema**：

| 代 | 代表 | 形状 | 适用 |
| --- | --- | --- | --- |
| **一** | `01-lexical`、`02-parser` | `source` + `tokens`/`statements` + `diagnostics`（带 `code`+`message_id`+`start`/`end` 字节偏移） | **前端结构快照** |
| **二** | `03-expression`、`05-containers`、`06-modules` | `{stage, status, cases: [{name, source, expect, diagnostics}]}`，`expect` 是 `success`/`error` | **正反例 + 稳定诊断编号** |
| **三** | `09-bytecode` | `status: verified-runtime`，每条锁 `outcome`/`error_code`/`value`/`releases`/`max_call_depth` | **执行结果向量** |

**新增 07/08/10/11 的夹具时，必须选一套既有 schema 并在新 README 里写明**——
不要发明第四套。按规则性质选：

- **07 的语法/静态边界** → 第二代（正反例 + 诊断编号）；
- **08 的 AST/IR 快照** → 第一代（结构快照）或第三代的变体；
- **11 的配置** → 第二代（`xiao-config` 的诊断编号已稳定）；
- **10**（若做）→ 第三代。

⚠️ **`12:21-25` 的「每条已确定语法至少三件」在现实中并未 1:1:1 配对**——
`04-types/declarations.json` 把 4 个合法 + 1 个非法混在一个文件里，
`09-bytecode` 是"向量"而非"正反例对"。**照既有做法写，别为了形式硬凑。**

---

## 六、硬约束

### 6.1 唯一判据是「有执行入口」（§二）

**每条新夹具都要能指出「哪个文件哪一行读它」**，且**撤掉实现时用例必须失败**。

### 6.2 ⚠️ 顺手点名两处**既有的 README 登记偏差**

它们与本债项**同型**（README 声称的 harness 实际不读夹具），建议同批更正：

- `tests/spec/05-containers/README.md:6-10` 把**三个**文件列为"对应实现测试"：
  `xiao-syntax/tests/c0_containers.rs`、`xiao-syntax/tests/c0_snapshots.rs`
  与 `xiao-types/tests/c0_containers.rs`——**而它们都不读 JSON**
  （`c0_snapshots.rs` 里 `grep "include_str|tests/spec|.json"` **零命中**，
  全用内联 `SourceFile::from_text`）；
- `tests/spec/04-types/README.md:3-4` 称"类型语义快照由 `xiao-types` 的测试入口验证"，
  **但全仓 `declarations.json` 只被 `xiao-syntax/tests/p2_snapshots.rs:34` 加载**，
  `xiao-types/` 下零命中。

**这两处不改，README 就继续在说谎**——而那正是「静默掩盖」的起点。

### 6.3 各层的边界（**别把别层的规则搬进 spec**）

| 若规则是… | 归哪层 | 本债项里的具体例子 |
| --- | --- | --- |
| 需要**跑两个后端对比**（stdout/退出码/错误类别/容器顺序/`drop`） | **双模式一致性**（`tests/differential/`） | `10:54`、`10:71` |
| 涉及**依赖方向、循环依赖、公共接口边界、README 登记** | **架构与耦合** | `08:36`（语义单一来源）、`11:9-11` |
| 涉及**换行、路径、固定宽度、环境注入、原生链接、交互终端** | **跨平台**（按 `10D` 写 `#[ignore]`） | `11:245-247`、`12:700` |
| 涉及**"这是 Rust 实现的"** | **实现语言边界** | `11:9`、`10:15` |
| 只覆盖**单个模块行为** | **单元测试**（crate 内联） | `07:115-132` 的错误对象字段表、`08a:43` 的 `IrValidator` |
| 是**性能/体积** | **benchmarks** | `10:76-81` |

### 6.4 已补门禁：登记路径之外还要有真实加载者

`tools/repo-check/src/docs.ts` 的 `checkSpecFixtureExecution` 现在对每个
`tests/spec/*/` 子目录同时检查：是否被 `docs/module-registry.json` 的某个 `tests` 数组
登记，以及登记的 Rust/TypeScript 测试源码是否出现该目录的真实加载路径。只登记路径或
只保留 README/JSON 都会触发 `A0-DOCS-003`，不会再复制 `06-modules` 的无人读取债项。

### 6.5 其它

- **不新增第三方 Rust crate**；规格 harness 复用 workspace 已锁定的 `serde`/`serde_json`，`check:lock` 仍会拦截未登记的锁文件漂移。
- **`docs/module-registry.json` 的 `tests` 数组**逐文件登记，新增夹具必须同步。
- 新增 harness 文件要进 `module-registry.json` 的对应 crate `tests` 数组。

---

## 七、最可能翻车的地方

1. **只建目录不接 harness**（§二）——**本债项的头号翻车点**，
   `06-modules` 就是活标本，而且**门禁抓不到**。
2. **把 `10` 的"与字节码一致"写成 spec 夹具**（§4.1）——落到双模式层，
   与 `tests/differential/` 重复，且违反「禁止为某一型修改期望」。
3. **⚠️ 为 `07:303` 未冻结的顺序写夹具**——`07:66` 说顺序"固定为
   `finally → drop → 匹配 catch / 继续传播`"，但 `07:165` 又说
   「**最终的 `finally` 与 `drop` 精确顺序必须由生命周期测试冻结**」。
   **这是"首选方案已定、最终顺序待冻结"**——为它写规格就是**未定义即冻结**。
   **只写已冻结的部分**（语法、静态边界、`FatalError` 不可捕获、具体→一般的 handler 顺序）。
4. **发明第四套 schema**（§五）。
5. **按阶段编号顺序做**（§4.3）——应从 `06-modules` 接线起步，不是从 07 开始。
6. **忘了 README 要写格式约定**——每个 spec 目录的 README 都必须写清 schema 与执行入口
   （照 `05-containers/README.md:42-58` 的「执行入口」一节）。

---

## 八、验收

1. **每个新夹具都有可指认的加载点**（`文件:行号`），且**撤掉实现该用例失败**；
2. **`06-modules` 的两个夹具已被读取**，且**接线后暴露的分歧已记录**（照 `59a0317` 的形态）；
3. **新目录的 README 写明了所选 schema 与执行入口**，且**没有第四套 schema**；
4. **`05-containers` 与 `04-types` 的两处 README 登记偏差已更正**（§6.2）；
5. **`10` 若未做，在文档里写明"为什么不做"**（§4.1 的论证）；
6. **未为 `07:303` 的未冻结顺序写夹具**（§七.3）；
7. **`docs/module-registry.json` 同步**；
8. **门禁全绿**，含 `check:lock`、`bunx tsc`。

---

## 九、不负责与不要重复做的事

### 9.1 ⚠️ `tests/unit/` 与 `tests/integration/` **不在本债项**

`11x0p:394` 把它们记为「**只有 README**」，但那是**测量事实**，
与 `:397-400` 的**债项登记范围**（只含 07/08/10/11/11X0 的规格测试）是**两回事**；
`11x0p:420` 还明令「不做 X0 第 3 条以外的规格测试扩张」。

**它们空着的原因是实践上测试全落在代码旁边**：
Rust 在 crate 内 `#[cfg(test)]` 或 `<crate>/tests/`（20 个 crate 各有目录），
TS 在源码同级 `*.test.ts`（12 个）。**这套约定已稳定运行，不要动。**

**必须在文档里显式声明 out of scope**，否则接手者读到 `11x0p:394` 会误以为在范围内。

### 9.2 其余

- **不补 07-C/07-D、10 的 N0-C/D、11A/11B/11C**——它们是后置的，不在本债项。
- **不做 `[debug]` 字段的规格测试**——`07:305` 与 `11:213` 互相指向，**两侧都没冻结**。
- **不做 X0 第 8 条的 POSIX 端到端**——`11x0p1 §9.2` 已排除，属独立工作。
- **不动 `11x0-protocol`**——它已有 5 夹具 + 双侧 harness，**不欠债**。

---

## 相关页面

- [11X0-P. 跨平台复现](11x0-platform-reproduction.md) §8.2 —— 债项来源与范围裁定
- [11X0-P1. 跨平台复现收口](11x0p1-platform-reproduction-closure.md) §9.1 —— **「门槛是有执行入口」的出处**
- [12. 测试与开发里程碑](12-tests-and-milestones.md) `:7-37` / `:717-720` —— 六层分层与债项登记
- [00A. 工程框架与目录布局](00a-project-layout.md) `:155-157` —— `tests/spec` 的宪章（含「配置」）
- [05-containers 夹具说明](../../tests/spec/05-containers/README.md) `:42-58` —— **接线暴露分歧的先例**
- [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 「撤掉实现 → 用例必须失败」的验收范式
- [10D. 环境依赖测试规范](10d-environment-gated-test-spec.md) —— 跨平台层的 `#[ignore]` 规格

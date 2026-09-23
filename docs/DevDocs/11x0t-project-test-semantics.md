# 11X0-T. 项目测试语义与结果协议

> **本批是 X0 退出条件第 1 条的最后一块。** `xiao test` 从 X0-B 起就只完成**入口登记**，
> 执行时返回 `X11-CLI-TEST-001`。`11x0b §2.1` 当时面临一个仓内没有规格的问题，
> 三条候选 A/B/C 中选择 **C（只登记）**，并立下规矩：
>
> > **不允许**悄悄实现一个名字对但语义含糊的命令。
>
> **X0-T 就是那个「后续批次」。** 所以本批的第一件事不是写代码，是**裁定语义**。

## 一、Agent 交接上下文

### 接手前提

1. [11X0-B. TypeScript CLI 骨架](11x0b-cli-shell.md) **§2.1**（`:66-84`）
   —— **本批的问题来源**。三条候选与「选择 C」的裁定在这里；`:190` 的第 4 条翻车点
   （「`xiao test` 语义含糊地实现了」）是本批要避免的。
2. [11X0-E. `xiao build` 与主机工具链发现](11x0e-build-and-toolchain.md) **§5.3**（`:206-220`）
   —— **归属裁定**：X0-T 负责「测试文件发现、确定性执行顺序、隔离/超时、源码测试入口和
   机器结果协议」。
3. [12. 测试与开发里程碑](12-tests-and-milestones.md) **`:7-37`（测试分层）与 `:702-704`（X0-T 登记）**
   —— **本批交付物必须与六层测试分层对齐**，尤其第 6 层「实现语言边界测试」的
   「**通过结构化请求/结果验证两层接口，不把本地化文本当作契约**」。
4. [11X0. 跨平台工具链](11x0-cli-protocol-and-toolchain.md) **§2.4**（`:155-175`）
   —— **协议单一来源（方案 C）**。本批若加协议字段，必须遵守这套机制。
5. [11X0-A §1.3](11x0-cli-protocol-and-toolchain.md) —— **`print` 不存在**，
   `RunOutcome.value` 恒为 `None`。**这条直接决定本批判据的形状**（见 §2.2）。
6. [09-B0-E. VM 中途取消检查点](09b0e-vm-cancellation-checkpoint.md) —— **与本批同期的另一笔**，
   **它决定「隔离/超时」这条出口条件能承诺到什么程度**（见 §2.3）。
7. [10D. 环境依赖测试规范](10d-environment-gated-test-spec.md) —— 新测试若依赖环境，
   按 `#[ignore]` 规格写。

### 现状盘点（2026-09-23 实测）

```text
CLI 侧    cli/ts/src/commands/parser.ts:57-60   test 分支（无 - 前缀校验）
          cli/ts/src/commands/index.ts:54-56    直接返回 X11-CLI-TEST-001
          cli/ts/src/main.test.ts:9-18          锁死「返回 64 + X11-CLI-TEST-001」
协议侧    xiao-driver/src/protocol/request.rs:213-292   ProtocolRequest（Hello/Run/Build/Cancel/Shutdown）
          xiao-driver/src/protocol/message.rs:14       PROTOCOL_VERSION: u16 = 1
          xiao-driver/src/protocol/service.rs:34-106   dispatch
测试发现  全仓无任何「发现 Xiao 测试文件」的实现
语言层    xiao-syntax/src/token.rs 里没有 test 保留字；project-layout.md 没有 tests/ 目录约定
```

**三个空白**（本批要填）：

- **没有测试发现**——CLI 侧仅有的目录遍历是配置发现（`config/editor.ts`）与核心/工具链发现，
  都与测试无关。
- **没有确定性顺序规格**——全仓唯一的先例是
  `xiao-modules/src/discovery.rs:86`（`entries.sort_by_key(file_name)`），
  由 `project-layout.md` 末段的「模块发现结果按逻辑名称排序，供后续依赖图和诊断稳定复现」背书。
- **没有测试书写形态**——见 §2.1。

### 本批交付与不负责

**交付**：`xiao test` 的**语义裁定**、测试文件发现与确定性执行顺序、机器可读结果协议、
`UseDocs` 与全部登记点的同步。

**不负责**：**VM 中途取消检查点**（归 [09-B0-E](09b0e-vm-cancellation-checkpoint.md)）；
**内置函数与断言**（归 20 阶段）；**别名层清理**（见 §七）。

---

## 二、必须先裁定的三件事

### 2.1 Xiao 项目的测试**怎么写**（仓内没有规格）

**事实**：

- `xiao-syntax/src/token.rs` 里**没有 `test` 保留字**（唯一的模块导入关键字是 `Import`）——
  `test` 在今天只是普通标识符；
- `docs/UseDocs/language/modules/project-layout.md` 只定义 `.xiao` → 模块映射、
  `config.xiao` 边界与扫描排除，**没有 `tests/` 目录约定**；
- **唯一的线索**在 [`11-cli-config-and-platform.md:105`](11-cli-config-and-platform.md)：
  > 原规划中的 `xiao -VM config_path.xiao test.xiao` 先作为候选形式，**最终选项名待 CLI 阶段确认**。

**这条线索对应的正是交付项里的「源码测试入口」，但形态完全未定。**

**必须先回答**：`xiao test` 是**项目测试运行器**（跑用户写的测试）还是**规格测试入口**
（跑工具链自带的用例）？——这就是 `11x0b §2.1` 的 A/B 之争，当时被推迟，现在必须裁决。

**若选「项目测试运行器」**，还要再定**测试文件的识别方式**，候选至少包括：

| 方式 | 形状 | 代价 |
| --- | --- | --- |
| **约定目录** | 项目里 `tests/**/*.xiao` | 无需改语言、无需改配置；但"测试"与"普通程序"没有语言层区分 |
| **`config.xiao` 声明** | 配置里列出测试入口 | 显式、可审计；但依赖 D0/E0 的配置消费链 |
| **语言层声明** | 新关键字或 `[test]` 表 | 语义最清晰；**但这是语言特性，改动面最大** |

**本批必须选一条并写明**。`11x0b:78-84` 的规矩在这里同样适用：
**不允许实现一个名字对但语义含糊的命令**，也不允许"先随便做，以后再说"。

### 2.2 通过/失败的判据**只能**建立在「退出码 + 结构化诊断」上

**这是硬约束，不是选择**：

- `print("hello")` 今天报 `X06-RUNTIME-012`（内置函数不存在，`11X0 §1.3` 决策三）；
- 脚本模式的 `RunOutcome.value` **恒为 `None`**；
- 也就是说**用户程序跑完不产生任何可观察输出**。

**推论**：测试**不能**靠"比对程序标准输出"判定成败。

这与 `12-tests:35-37` 的既有纪律一致：结果协议**走结构化字段**
（`code`/`message_id`/`exit_code`），**不把人类可读文案当契约**，
也**不把 Rust 内部布局当契约**。

⚠️ **一个推论上的陷阱**：既然判据是退出码，那么"一个测试文件"能表达的成败粒度就受限于
一次程序运行的终局（B0-D 的五个值）。**若需要多用例粒度，就必须在语言或协议层新造机制**
——那属于 §2.1 的形态选择，别在本批里用"一个文件跑一次、按文件名报告"含糊带过。

### 2.3 「隔离/超时」的边界，以及与 09-B0-E 的接口

出口条件里有「**隔离/超时**」这一条，但**它今天做不到承诺的那种程度**：

- **超时**：驱动器只做**阶段边界采样**（开始、前端完成、降低完成、VM 调用前后，
  见 [09-B0-E §3](09b0e-vm-cancellation-checkpoint.md)）。**VM 指令循环里没有检查点**，
  所以**超时无法中断 VM 里的死循环**——VM 会跑完，跑完后才发现已超时。
- **隔离**：协议层每个 run/build 请求在**独立线程**中执行（`service.rs:495` 的
  `thread::spawn`），panic 被 `catch_unwind` 转成 `X11-PROTOCOL-003`。
  但**没有子进程隔离**，全局状态（VM 单例、文件系统）**不隔离**。

**二选一，并写明**：

- **(a) 本批只承诺阶段边界层面的隔离/超时，并在 UseDocs 写明边界**——
  **推荐**。它诚实、可交付，且不阻塞任何事。
- **(b) 等 09-B0-E 完成后才承诺中途取消**——代价是本批无谓地等待，
  而 09-B0-E 的出口条件里**并没有**包含 `xiao test` 的语义。

**无论选哪条，`xiao test --timeout` 这类承诺都必须与 [09-B0-E](09b0e-vm-cancellation-checkpoint.md)
的实际能力对齐**，否则就是超额承诺——那正是 `11x0b:190` 第 4 条翻车点的另一个形态。

---

## 三、硬约束

### 3.1 `cargo test` / `bun test` **不得**被包装成 `xiao test`

这条在仓内**重复登记了六处**（`12-tests:703-704`、`11-cli-config-and-platform.md:20-25`、
`11x0b:82-84`、`11x0:251`、`11x0e:218-219`、`shell.md:46-47`）。
**`xiao test` 必须是新写的东西，而不是这两者的壳。**

### 3.2 协议字段：**夹具先行**，两侧同批

`tests/spec/11x0-protocol/README.md:15-16` 与 `UseDocs/tooling/cli/protocol.md` 规定了顺序：

> **增加字段时先更新夹具，再更新两侧的显式类型和测试。**

配合 `11x0:174-175` 的「**迁移前不得同时保留第二套隐式规则**」——
**不得**新增一套只在某一侧存在的字段约定。

⚠️ Rust 侧用 `include_str!` 的**相对路径**引用夹具（`x0_a_protocol.rs:9-16`），
**移动或重命名夹具目录会直接编译失败**。

### 3.3 ⚠️ 两侧版本常量**没有跨语言断言**

`PROTOCOL_VERSION` 在 Rust（`protocol/message.rs:14`）与 TS（`protocol/messages.ts:4-5`）
**各写一份，没有任何测试把它们绑在一起**。唯一的外部绑定是
`x0_a_protocol.rs:47-51` 对 `hello-request.json` 里 `protocol_version` 的断言。

**后果：只改一侧而忘了另一侧，门禁不会红**——只有混合版本真正运行时才炸
（`validate_versions` 是**严格相等**，`validate.rs:14-25`）。

**本批若递增版本**（新增协议字段时很可能要），**必须两侧 + 夹具三处同批改**，
并**补一条跨语言断言**把这个洞堵上。这条本身就是一个值得单独提交的改进。

### 3.4 CLI 退出码 `64` 与协议退出码是**两套**

- `CLI_EXIT_CODES.usage = 64`（`diagnostics/render.ts:41-48`）的语义是
  **「命令本身没用」**——参数错误、文件读取失败，以及**当前的 `test` 未实现**。
- 协议结果的退出码是**另一套**：`render.ts:115-118` 的 `responseExitCode`
  直接取响应的 `exit_code`（B0-D 冻结的 `0..=4`）。

**所以**：X0-T 落地后，**测试失败不应继续用 64**。测试结果若走协议回传，
进程码必须来自 `response.exit_code`；**只有 CLI 自身的错误**（找不到项目、
无测试文件、参数非法）才用 `usage` / `infrastructure`。

⚠️ `CliCommandError` 的默认 `exitCode` 就是 `usage`（`index.ts:38`）——
**新增诊断若忘记传退出码，会静默变成 64**。

### 3.5 既有测试**故意**锁死了旧行为

`cli/ts/src/main.test.ts:9-18` 断言 `xiao --json test` 返回 **64** 且
`code === "X11-CLI-TEST-001"`。**本批一改实现它必红——这是「故意要改」的测试，不是回归。**

重写它时**要保留区分度**（沿用 09R2D 的「撤掉实现 → 用例必须失败 → 还原 → 通过」）：
新测试必须在**语义被撤掉时失败**，而不是只断言"命令能跑"。

### 3.6 架构回归测试**逐字**锁死协议模块名单

`protocol_architecture_tests.rs` 干三件事：

- `:25-37` 断言门面里存在 `mod frame; … mod service;`——**加 `protocol/test.rs` 必须同批更新**；
- `:93-106` 断言 `service.rs` **显式依赖**每个子模块；
- `:109-122` 断言**除 `service.rs` 外**不得出现 `std::thread` / `Arc<`。

**新增子模块若不同步这些断言，它会「逃出」依赖图检查。**

### 3.7 别把新逻辑塞进 `service.rs`

`A0-SIZE-001` 是**每文件 2500 物理行**。`protocol/build.rs` 已 601 行、`service.rs` 536 行，
而 `11x0f §现状盘点`（`:42-45`）明说**协议层是单调增长的**（新字段、新命令、新端点）。
**本批应新开子模块，而不是加宽 `service.rs`**。

### 3.8 确定性顺序：援引既有先例，别新造规则

全仓唯一可援引的先例是 `xiao-modules/src/discovery.rs:86` 的
`entries.sort_by_key(file_name)` / `:173-174` 的逻辑名排序，
由 `project-layout.md` 的「模块发现结果按逻辑名称排序」背书。

**测试文件的执行顺序应当与它同源**（同一种排序口径），而不是另立一条——
否则就是本仓登记过 7 次的**单一来源病**的又一次复发。

### 3.9 其他

- **不新增 Rust 依赖**；若新增，`check:lock` 会拦（`tests/benchmarks` 是独立 crate，
  动它必须提它的 `Cargo.lock`）。
- **`docs/module-registry.json`** 的 `rust.xiao-driver.tests`（`:40`）与
  `ts.xiao-cli.tests`（`:42`，逐文件列 `.test.ts`）要同步。
- **TS 侧 `export` 项文档覆盖率 100%**；UseDocs frontmatter 六字段齐全，
  且 `ts.xiao-cli` 的 UseDocs 必须保持 `status: verified`。
- ⚠️ `parser.ts:57-60` 的 `test` 分支**没有 `-` 前缀校验**（对比 `parseBuild` 的
  `argument.startsWith("-")`）：`xiao test --filter` 会**静默**把 `--filter` 当成项目路径。
  新选项必须在 `test` 分支内解析。

---

## 四、分步提交（**不要一笔做完**）

| 步 | 内容 | 为什么这个顺序 |
| --- | --- | --- |
| **1** | **语义裁定**：写进 `11-cli-config-and-platform.md`，含形态选择与理由 | `11x0b` 的规矩：先有语义，再有实现 |
| **2** | 测试文件发现 + 确定性顺序（可能纯 Rust 侧） | 可独立测试；顺序口径照 §3.8 |
| **3** | 协议侧的新命令与结果形状（**夹具先行**，见 §3.2） | 两侧同批，含 §3.3 的跨语言断言 |
| **4** | CLI 侧接线：`parseTest`、`executeTest`、客户端方法 | 照 `build` 的模板（§五） |
| **5** | 结果渲染与退出码（§3.4） | 与协议回传形状绑定 |
| **6** | 文档与登记收尾：六处承诺点 + UseDocs 新页面 + `module-registry.json` | 见 §六 |

**每一步的验收**：既有测试除 `main.test.ts`（故意要改）外不改、门禁全绿。

---

## 五、CLI 侧接线的现成模板

**`build` 是最完整的模板**（`11x0b:222` 要求新增命令照它的形状）：

```text
parser.ts  parseBuild（:94-151）
           ↓
index.ts   executeBuild（:94-137）：
           readFile + TextDecoder(fatal) → findProjectConfig
           → discoverToolchainWithMetadata → new ProtocolClient({...})
           → renderProtocolResponse(result.response, options)
```

**加一个新命令要动的文件清单**（照 `build`）：

1. `commands/parser.ts` —— `ParsedCommand` 联合（`:14-21`）、`parseArguments` 分支（`:55-67`）、
   `helpText()`（`:71-84`）；
2. `commands/index.ts` —— `executeCommand` 分支（`:48-62`）+ 新的 `executeTest`；
3. `protocol/client.ts` —— ⚠️ `call()` 的签名是 `RunRequest | BuildRequest`（`:181`），
   **必须扩签名或加新方法**；会话是写死的 hello → 单请求 → shutdown（`:207-231`）；
4. `protocol/messages.ts` —— 请求/响应联合类型；
5. `diagnostics/render.ts` —— 若新结果形状需要专属渲染分支（参考 `:80-94` 的 build 分支）；
6. 新增 `.test.ts`；
7. 三处 README（`commands/`、`protocol/`、`cli/ts/`）—— 目录 README 必须与实现同步。

---

## 六、验收

**关键验收不是「测试通过」**：

1. **语义已裁定且可追溯**——`11-cli-config-and-platform.md` 里写明了 `xiao test` 跑什么、
   测试怎么写、判据是什么（§2.1），**且不是「语义含糊但名字对」**（`11x0b:190` 第 4 条）；
2. **区分度**：撤掉发现/执行/结果协议中的任一环，对应用例必须失败；
3. **确定性顺序有断言**——同一次输入两次运行结果一致，且顺序口径与 §3.8 的先例同源；
4. **判据不依赖程序输出**（§2.2）——测试用例**没有**任何"比对 stdout"的断言；
5. **超时/隔离的承诺与实现一致**（§2.3）——UseDocs 写明的边界与实际能力吻合，
   **没有超额承诺**；
6. **协议改动走完夹具先行**（§3.2），且两侧 + 夹具同批；
7. **`main.test.ts` 已重写且保留区分度**（§3.5）；
8. **六处承诺点全部同步**：`shell.md:41-42/:46-47`、`UseDocs/tooling/cli/README.md:22-23`、
   `build.md:96-97`、`11-cli-config-and-platform.md:24-25`、`11x0:39/:251/:330`、
   `12-tests:702-704`、`11x0e:218`、`DevDocs/README.md:99`、`commands/README.md:6`；
9. **新增 `UseDocs/tooling/cli/test.md`**（照 `build.md` 的六段形状：
   输入/输出/退出码/失败恢复/平台差异），并从 `README.md` 与 `shell.md` 链接；
10. **门禁全绿**，含 `check:lock`、`bunx tsc`、`cargo check --manifest-path tests/benchmarks/Cargo.toml`。

---

## 七、不负责与不要重复做的事

- **不做 VM 中途取消检查点**——归 [09-B0-E](09b0e-vm-cancellation-checkpoint.md)，**本批不碰 VM 热循环**。
- **不做内置函数 / 断言 / `print`**——归 20 阶段（`11X0 §1.3` 决策三）。
  **本批的判据建立在退出码与结构化诊断上，不等待 20 阶段。**
- **不把 `cargo test` / `bun test` 包装成 `xiao test`**（§3.1）。
- **不新增一套协议字段约定**（§3.2）。
- **不重跑或改动 09R3 的基准**。
- **不做别名层清理**——`11x0e §5.4` 曾把它也写成"由 B0-E 验收"，**已裁定拆开**
  （见 [09-B0-E §七.1](09b0e-vm-cancellation-checkpoint.md)）。它与本批无关。
- **不要顺手修 `parser.ts:57-60` 缺 `-` 前缀校验**——那是既有现象，
  若本批要为 `test` 加选项则顺势处理，否则**另开提交**。

---

## 相关页面

- [11X0-B. TypeScript CLI 骨架](11x0b-cli-shell.md) §2.1 —— **本批的问题来源与「不允许语义含糊」的规矩**
- [11X0-E. `xiao build` 与主机工具链发现](11x0e-build-and-toolchain.md) §5.3 —— **归属裁定**
- [12. 测试与开发里程碑](12-tests-and-milestones.md) `:7-37` / `:702-704` —— 测试分层与 X0-T 登记
- [11X0. 跨平台工具链](11x0-cli-protocol-and-toolchain.md) §2.4 —— 协议单一来源（方案 C）
- [09-B0-E. VM 中途取消检查点](09b0e-vm-cancellation-checkpoint.md) —— **同期另一笔**，决定「隔离/超时」的边界
- [11-cli-config-and-platform.md](11-cli-config-and-platform.md) —— `xiao test` 裁定落地处
- [00A. 工程框架与目录布局](00a-project-layout.md) —— 目录边界与登记要求

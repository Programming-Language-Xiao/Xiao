# 20E-归档接线. `.xar` 的标准库兼容与能力目标约束

> **背景**：路线稿 §七 把 20E 定为「**归档接线**」：`.xar` 的标准库兼容检查、
> 资源能力目标约束和负例，前置是 **17 的索引与启动验证**。
> 17A–17D 已完成：唯一索引、启动前完整验证、语言上下文和平台/调试接线都在；
> 20C 冻结了 `StandardLibraryIdentity`，20D 冻结了能力位 0–6。
>
> **但归档索引里今天没有标准库要求，也没有能力位**：`xiao-xar`/`xiao-artifacts`
> 对 `stdlib`/`identity`/`capability` 零引用。20E 要把这两类要求装进索引，
> 并在**任何模块初始化之前**校验。

状态：**本轮自审修复已验收并通过审核（2026-10-18）**：载荷与索引语言一致性、启动预检顺序、读写调试/入口约束及协议诊断启动顺序已修复并通过完整门禁与三平台回归，见 §十二；审核结论与三条低优先级见 §十三。原批次证据保留在 §十一；18B 的实际可达闭包和 CLI 构建接线仍独立处理。

## 一、为什么是这一批

1. **17 的前置已经满足**：[17](17-xar-archive.md) 的索引（17A/17B）与启动验证（17C/17C-FIX/17D）
   都已交付；20E 不需要重做归档格式；
2. **两个身份块已经存在但没接线**：20C 的 `StandardLibraryIdentity`
   （`xiao-driver/src/compatibility.rs:194`）和 20D 的能力位（`xiao-intrinsics` 的 bit 0–6）；
3. **不接线的后果**：归档可以在一个接口版本不匹配的标准库上启动，或在缺少所需能力的
   上下文里先执行一部分模块再失败——这正是 20E 要消掉的两种静默。

## 二、冻结已定死的（**输入，不许重新论证**）

| 项 | 冻结内容 | 出处 |
| --- | --- | --- |
| 归档格式 | 唯一 `META-INF/xiao/index.pb`；字段 1–6 保持原义，新字段从 7 起；旧读取器跳过未知字段 | [17A](17a-archive-format-and-codec.md) §2.3 |
| 启动验证 | 先验证索引/对象/ABI/平台，再解析入口；**校验失败不部分执行** | [17C](17c-entry-and-runner.md) §2.3 |
| 标准库身份 | `language_version`、`stdlib_interface_version`、`runtime_abi_major/minor`、`target`；可读、可比较 | [20C](20c-official-xiao-library.md) §3.2 |
| 能力位 | bit 0–6 = `filesystem`/`clock`/`process`/`network`/`native_extension`/`stdout`/`stdin`；其余位拒绝 | [20D-冻结](20d-freeze-resource-contract.md) §2.7 |
| 能力码 | 能力缺失 → `X06-RUNTIME-026` | 同上 |
| 发布边界 | 归档**不携带**官方 Runtime/标准库；只记录模块/ABI 要求；签名/信任不在本阶段 | 路线稿 §七、[17](17-xar-archive.md) §一 |

## 三、本批冻结的契约（四道裁定之后）

### 3.1 索引新增四个字段（字段号 13–16，追加）

| 字段 | 类型 | 语义 |
| --- | --- | --- |
| `stdlib_interface_version` | `u32`（varint） | 构建时标准库公开接口主版本；与 `current_standard_library_identity()` **精确相等** |
| `language_version` | string | 构建时语言语义版本；**精确相等** |
| `stdlib_modules` | repeated string | 归档实际依赖的官方模块规范身份（如 `stdlib.os`）；**不复制对象** |
| `required_capabilities` | `u64`（varint） | 归档运行所需能力位；只能使用冻结的 bit 0–6 |

- 既有字段 1–12 **不改语义、不改字段号**；`ArchiveEntry` 六个字段不动；
- 新字段按 protobuf 语义追加，旧读取器可跳过——**但这正是问题**：旧启动器会跳过新字段、
  不做检查。因此**新归档必须同时设置一个新的 required-feature 位**（见 §3.4），
  让旧读取器因「未知必需特性」拒绝，而不是静默放行。

### 3.2 兼容规则

- `stdlib_interface_version` 与 `language_version` **精确相等**；0.1.0 没有迁移窗口；
- Runtime ABI 沿用现有 `runtime_abi_min`/`runtime_abi_max`；
- 平台约束沿用现有 `platform`/`entry.target` 规则；
- 缺标准库模块 → `X17-XAR-008`（依赖未满足），消息带模块名；
- 版本不匹配 → `X17-XAR-007`（版本不兼容），消息带期望值与实际值。

### 3.3 能力规则

- `required_capabilities` 只能包含冻结位；未知位 → **`X17-XAR-011`**（本批新增，索引无效）；
- 启动时用**本次请求的能力上下文**与归档要求比对；缺位 → **`X06-RUNTIME-026`**，
  消息带缺失能力名；
- 检查在**任何模块初始化之前**；失败时**不得**产生模块加载/函数进入事件；
- 构建侧从 **intrinsic 契约**推导所需位，不引入手写配置（首批）。

### 3.4 旧读取器/旧归档门禁

- 新增一个**归档专用** required-feature 位（例如 bit 0 = 「标准库/能力要求存在」），
  只写入 `ArchiveIndex`，**不改** `GlobalIndex` 的 feature 掩码；
- 旧启动器（17A）期望 feature 为 0，遇到新位会报「未知必需能力」并拒绝；
- 新启动器要求该位；缺少该位的旧归档按 `X17-XAR-007` 拒绝，不做静默兼容。

## 四、实施前核出的现状

### 4.1 索引字段

`ArchiveIndex`（`xiao-artifacts/src/lib.rs:1206`）现有 12 个字段：
`schema_major/minor`、`entry`、`entries`、`dependency_lock_digest`、
`runtime_abi_min/max`、`platform`、`debug_activation`、`language_locale`。
编码字段 1–12 已占用；**没有**标准库版本、模块列表或能力位。

### 4.2 启动验证顺序

`run_archive_with_control`（`xiao-xar/src/runner.rs:213`）当前顺序是：
解码 → 语言 → 事件容量 → Runtime ABI → 平台 → 每条 entry 的 target → 入口对象 → 读对象 → VM。
20E 的新检查必须插在**入口对象读取之前**，并保证失败时不进入 VM。

### 4.3 标准库身份与能力上下文

- `StandardLibraryIdentity` 已有五个字段，但 `xiao-xar`/`xiao-artifacts` 零引用；
- `XarRunOptions`（`xiao-xar/src/runner.rs:15`）有 `runtime_abi`/`platform`/`language_locale`，
  **没有** `capabilities`；构造点在 `xiao-driver/src/protocol/run.rs:440`；
- 20D 的请求已经携带不可变能力掩码，20E 只需把它传进 `XarRunOptions`。

### 4.4 构建侧边界

- 18B（`build -xar`）仍在进行中；CLI 的 `build` 对 `.xar` 输入目前直接拒绝；
- 按裁定四，20E **先在库层**落地索引字段、校验和负例，用 `XarBuilder` 构造测试归档；
  真实构建路径填字段由 18B 接入，不阻塞本批。

### 4.5 i18n 缺口（本批顺手补）

`XarRunError::message_id` 使用的四个键
（`x17.xar.archive_missing_object`/`archive_version_incompatible`/
`archive_dependency_unsatisfied`/`archive_validation_failed`）
在 `xiao-i18n` 表里**没有登记**；目前只登记了 `x17.xar.language_fallback`。
20E 新增的错误路径会放大这个缺口 ⇒ 本批补齐这四个键 + 新 `X17-XAR-011` 的键，
并在中英目录各提供一条消息。

## 五、本批要做的事

1. **索引模型**（依赖：无）
   - `ArchiveIndex` 加四个字段；`encode`/`decode`/`validate_archive_index` 同步；
   - 字段号 13–16；`stdlib_modules` 编码前按字节序去重排序（确定性）；
   - 新归档写 required-feature 位；`GlobalIndex` 不变；
   - `XarBuilder` 增加设置入口；未设置时编码失败（不能产出没有兼容元数据的归档）；
2. **标准库兼容检查**（依赖 1）
   - `run_archive` 在 Runtime ABI 检查前后（建议紧邻）比较
     `stdlib_interface_version`/`language_version` 与 `current_standard_library_identity()`；
   - 逐个检查 `stdlib_modules` 能被已安装标准库解析（只解析，不执行源码）；
   - 版本不匹配 → `X17-XAR-007`；模块缺失 → `X17-XAR-008`；
3. **能力检查**（依赖 1）
   - `XarRunOptions` 加 `capabilities: u64`，默认 `KNOWN_CAPABILITY_MASK`；
     `protocol/run.rs` 从请求掩码传入；
   - 索引解码/校验拒绝未知位 → `X17-XAR-011`；
   - 启动前 `required & !context != 0` → `X06-RUNTIME-026`，消息列出缺失能力名；
4. **失败顺序与不部分执行**（依赖 2、3）
   - 新检查全部在入口对象读取之前；
   - 负例必须断言没有 `ModuleLoaded`/`FunctionEntered` 事件；
5. **i18n 与诊断**（依赖 2、3）
   - 补齐 §4.5 的四个键 + `X17-XAR-011` 的中英文消息；
   - 新增错误码常量并登记到 `xiao-xar` 的公开常量；
6. **构建侧契约**（依赖 1；实现归 18B）
   - 记录字段来源：标准库模块 = 入口可达的 `stdlib.*` 导入闭包；
     能力位 = 入口可达的所有 intrinsic 在 `IntrinsicDecl.capability` 上的并集；
   - 不在本批做 CLI `build -xar`；18B 对接时只填字段，不改本批检查；
7. **回归**（依赖 2–6）
   - 正例：匹配的标准库/能力上下文运行成功；
   - 负例：接口版本不匹配、语言版本不匹配、缺模块、缺能力位、未知能力位、
     缺字段的旧归档、旧读取器拒绝新 feature；
   - 确定性：同一索引两次编码逐字节相同；新字段往返稳定；
   - 不复制标准库：归档 entries 里没有官方模块对象；
8. **文档与登记**（依赖全部）
   - 17 规格 §待定决策回填「索引字段号/版本已由 17A 冻结」；
   - `docs/module-registry.json` 补 20E 条目（保持与既有阶段一致的 `draft`/证据口径）；
   - 用户文档补「归档需要匹配的标准库接口版本与能力位」。

## 六、不许

1. **不许**把官方 Runtime/标准库复制进归档；模块只记为要求；
2. **不许**用手写配置代替 intrinsic 契约的能力推导（首批）；
3. **不许**在执行后再检查能力——那会部分执行；
4. **不许**让旧归档静默通过：缺新字段/新 feature 的归档必须按版本不兼容拒绝；
5. **不许**改 `ArchiveEntry` 六个字段或索引字段 1–12 的语义；
6. **不许**在本批新增签名/信任/撤销策略；
7. **不许**扩大原生/VM 的模块调用支持范围（18B 的构建接线另行处理）；
8. **不许**只测「能力缺失」而不测「未知位」「旧归档」和「无执行事件」。

## 七、四道裁决（已落定，均甲）

| # | 问题 | 裁定 |
| --- | --- | --- |
| 1 | 标准库要求怎么进索引 | **甲：版本字段 + 模块列表**（`stdlib_interface_version`、`language_version`、`stdlib_modules`） |
| 2 | 兼容严格度 | **甲：全部精确相等**；0.1.0 无迁移窗口 |
| 3 | 能力失败身份 | **甲：缺位 `X06-RUNTIME-026`；未知位 `X17-XAR-011`** |
| 4 | 与 18B 的边界 | **甲：库层先落地**，`build -xar` 的字段填充由 18B 接入 |

**来源**：星崽直接裁定（AskUserQuestion，2026-10-18）；四条均取甲。

## 八、落点

```text
docs/DevDocs/20e-archive-wiring.md                     本文
core/rust/crates/xiao-artifacts/src/lib.rs             ArchiveIndex 四字段 + feature 位 + 编解码/校验
core/rust/crates/xiao-xar/src/lib.rs                   X17-XAR-011 常量与 XarBuilder 入口
core/rust/crates/xiao-xar/src/runner.rs                XarRunOptions.capabilities + 启动检查
core/rust/crates/xiao-driver/src/protocol/run.rs       把请求能力掩码传入 XarRunOptions
core/rust/crates/xiao-i18n/src/lib.rs                  四个既有 X17 键 + 011 的中英消息
core/rust/crates/xiao-xar/tests/**                     正例、负例、确定性、无执行事件
docs/DevDocs/17-xar-archive.md                         §待定决策回填
docs/DevDocs/README.md                                 阶段索引加本文一行
```

18B 的 `frontend`/CLI 落点不在本文范围内；它只负责填字段。

## 九、验收

1. **索引**：四个新字段可往返、两次编码逐字节相同；`stdlib_modules` 排序去重；
   新 feature 位让旧读取器拒绝；
2. **标准库**：接口版本/语言版本精确匹配才启动；缺模块报 `X17-XAR-008`；
   版本不符报 `X17-XAR-007`；都不执行任何模块；
3. **能力**：缺位报 `X06-RUNTIME-026`；未知位在索引阶段报 `X17-XAR-011`；
   检查在入口对象读取之前，负例断言无 `ModuleLoaded`/`FunctionEntered`；
4. **边界**：归档不含官方模块对象；不改既有字段语义；
5. **i18n**：四个既有 X17 键 + 011 键在中英目录均可渲染；
6. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、
   `bunx tsc --noEmit` 全绿；提交标题带类型前缀、正文非空。

## 十、裁定结果（已执行）

见 §七 的四行表格；四条均取甲，来源为星崽直接裁定（AskUserQuestion，2026-10-18）。

## 十一、实施与验收记录（2026-10-18）

### 11.1 实际落点与边界

| 交付 | 实际落点 | 行为 |
| --- | --- | --- |
| 索引模型与 wire | `xiao-artifacts/src/lib.rs` | 字段 13–16；模块排序去重；版本与能力字段必填；bit 7/63 等未知能力位拒绝；字段 1–12 和 `ArchiveEntry` 保持原义 |
| 旧格式门禁 | 同上 | 归档专用 feature bit 0；新读取器拒绝缺 feature 的旧归档；冻结旧读取规则遇新位拒绝；`GlobalIndex` 掩码仍为 0 |
| 唯一标准库身份 | `xiao-standard-library/src/lib.rs`、`xiao-driver/src/compatibility.rs` | 身份定义移至标准库元数据层，驱动器保留原重导出，避免归档与驱动器循环依赖；不复制版本定义 |
| 库层构建入口 | `XarBuilder::with_requirements` | 接收身份、入口可达的官方模块及 intrinsic ID 闭包；能力从权威 intrinsic 契约生成；不读取新用户配置、不复制官方对象 |
| 启动前校验 | `xiao-xar/src/runner.rs` | 接口/语言版本精确比较，官方模块只解析不初始化；缺模块 `008`、未知能力 `011`、缺位 `026`；均在入口对象定位与 VM 建立之前拒绝 |
| 请求到实际 VM | `xiao-driver/src/protocol/{request.rs,run.rs}`、`xiao-vm/src/{run.rs,semantics/exec.rs}` | `RunOptions.capabilities` 默认七位全开，归档预检与实际 VM 使用同一掩码；旧序列化默认字段保持兼容；旧 `.xiaoc` 入口保留全开包装 |
| TypeScript 与消息 | `cli/ts/src/protocol/{messages.ts,client.ts}`、`xiao-i18n/src/lib.rs` | 客户端可传归档请求能力；补四个既有 X17 键、011 键及保留缺失名称的归档能力键，中英目录均验证 |

上表 Rust crate 路径均以 `core/rust/crates/` 为前缀。
字段 15 是 protobuf repeated：没有官方模块要求时允许省略；字段 16 即使为零也显式编码，
避免把未声明能力要求的旧归档当成零能力新归档。
归档索引禁止携带官方 `stdlib` 命名空间的模块对象；实际模块导入闭包及可达 intrinsic 闭包
由 18B 提供，本批没有接入 CLI `build -xar` 或扩大归档 VM/原生模块调用范围。

### 11.2 已完成的定向验证

- `20e_archive_wiring` **7 passed / 0 failed**：真实前端产物、构造器契约推导、版本/模块拒绝、
  七位能力拒绝、无执行事件、请求掩码进入 VM、协议与中英目录；
- 格式层回归包含字段往返/确定性/模块排序去重、必填字段、旧归档/旧读取规则、未知位、
  接口版本溢出及重新计算 ZIP 长度/CRC 后的实际畸形索引拒绝；
- 既有 `d17c_archive` **9 passed / 0 failed**，标准成功输出、审计、事件容量、诊断失败拒绝等保持；
- 体积回归 **2 passed / 0 failed**：`.xiaoc` 仍为 1256 字节；`.xar` 由 1151 变为 **1163**，
  12 字节增量来自新增字段 13/14/16，官方模块列表为空时字段 15 按 repeated 语义省略。

没有把缺少能力的运行时补救当作启动前检查的证据：拒绝用例要求 `run_archive` 直接返回错误，
并断言任何 `ModuleLoaded`/`FunctionEntered`/输出事件都未发生。
另有故意少报要求的夹具验证 VM 仍受请求能力限制，防止执行入口恢复默认全开。

### 11.3 完整验收

本机完整工作区为 **1216 passed / 0 failed / 41 ignored**；
`tools/gates/run.sh` exit 0，包含格式、Clippy、TypeScript、文档/覆盖率、锁文件、基准、
驱动器与 Bun。Bun 为 **295 pass / 5 skip / 0 fail**。
归档两层测试已移入各自 `src/tests.rs`，VM 复用既有内部构造入口；三个文件的规模守卫
均通过，未调整 2500 行上限。拆分后的格式回归与 20E 端到端测试已再次通过。

实现提交 `98a974f17b7860efaff4239f16e0fa35fd527d45`，
测试拆分与三平台接线提交 `b0269f5047d3f10f6e143d1753641fdf949bf41a`。
当前提交的 [工作区门禁 `39114319897`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39114319897)
已成功（作业 `117636875806`）。

[维护回归 `39114319971`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39114319971)
六个作业全部成功。下载三个已完成平台作业的完整日志，逐项确认新增 20E 步骤均实际执行
全部七项测试，均为 **7 passed / 0 failed / 0 ignored**，没有用 ignored 汇总代替执行证据。

| 平台 | 已完成的受控作业 | 20E 日志位置 |
| --- | --- | --- |
| Windows | [117636995831](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39114319971/job/117636995831) | 第 2362 行进入 `20e_archive_wiring`，第 2373 行为 7 passed |
| Linux | [117636995865](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39114319971/job/117636995865) | 第 2335 行进入 `20e_archive_wiring`，第 2346 行为 7 passed |
| macOS | [117636995961](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39114319971/job/117636995961) | 第 2201 行进入 `20e_archive_wiring`，第 2213 行为 7 passed |

七个用例在三平台均确认相同的版本/模块/能力错误身份、无执行事件、请求上下文及成功输出。
格式层的旧归档/旧读取规则、未知位、缺字段、确定性和物理畸形索引回归还由完整工作区
与安全作业执行。首轮提交的门禁 `39113925915` 因文件规模守卫失败，已通过测试模块拆分修复；
首轮维护回归 `39113925923` 被最终提交取代并取消，两者不作为最终验收依据。

因此 §九 六条验收均满足。本批交付库层检查和构造契约；18B 后续仅填入实际可达要求，
不需改动本批检查规则。

## 十二、自审修复（2026-10-18）

### 12.1 复现与修复

| 问题 | 修复前的复现 | 修复 |
| --- | --- | --- |
| 索引伪装载荷语言 | 索引写 `0.1.0`、载荷实际为 `0.2.0`，编码和读取均接受 | 两条方向共用载荷校验；索引和 `.xiaoc` 语言必须相等，不符报 `X17-XAR-007`；合法的外部语言归档仍可编码，由运行时精确兼容检查拒绝 |
| 新要求预检晚于载荷解码 | 同时声明不兼容接口/缺失能力并放入损坏入口，运行器先报载荷 `009`，未先检查 `007`/`026` | 私有索引状态只校验容器和索引；运行上下文通过后才解压、解码和验证全部载荷；公开可读归档仍必须先通过完整验证 |
| 读取端调试/入口约束缺失 | 写入器拒绝的调试激活位不符、内嵌完整调试/源码分区、入口映射缺失和零长度入口，读取端却接受 | 读写共用语言/调试载荷契约及入口索引校验，保留既有独立调试对象边界 |
| 协议检查晚于诊断启动 | 移除协议预检后，专门回归的 ABI 负例报 `X11-DIAGNOSTIC-START-001`，遮蔽应先返回的 `X17-XAR-007` | 协议和库层共用 `validate_archive_context`，版本、能力、ABI、平台、每条 entry.target 及可执行入口种类均先于载荷和诊断进程；索引层错误审计只计 1 个已验证成员 |

新增 `inspect_archive_index` 只返回索引和索引层审计，不暴露对象字节。
直接运行、完整审计、协议语言选择及初次读取都按相同顺序处理；`decode_xar` 的公开完整
验证保证保持。平台审计同时考虑各条 `entry.target`，不再只看归档顶层平台。
没有修改索引字段号、`ArchiveEntry`、GlobalIndex feature、Runtime ABI、intrinsic 或模块调用支持范围。
18B 的实际可达闭包填充继续按原边界处理。

### 12.2 回归证据

四个格式负例先实际失败：载荷语言伪装、读取端调试绕过、预检晚于载荷、入口映射/长度绕过。
修复后 `xiao-xar` 单元回归 **22 passed / 0 failed**，另有确定性模糊测试 **2 passed**；
`20e_archive_wiring` 由 7 项增为 **10 passed / 0 failed**，既有 `d17c_archive` **9 passed**。
新协议用例覆盖损坏载荷与版本/能力错误并存，要求先返回上下文错误且审计只计索引；
ABI/顶层平台/entry.target/入口种类错误在隔离子进程中以不可用诊断组件验证，确保不会启动真实窗口。
隔离子进程由同一个测试父用例驱动，不增加 ignored 项。

协议预检的移除验证实际退出 **101**：ABI 负例错误地进入诊断启动，返回
`X11-DIAGNOSTIC-START-001`，专门断言失败。源文件随后按原始字节恢复，变异不提交。

### 12.3 本轮完整验收

本轮最终代码的完整工作区为 **1223 passed / 0 failed / 41 ignored**；
完整 `tools/gates/run.sh` exit 0，覆盖格式、Clippy、TypeScript、文档/覆盖率、锁文件、
基准、驱动器与 Bun。Bun 为 **295 pass / 5 skip / 0 fail**。

修复提交为 `1b8c8468bbc335a6f9b1a5d362daf60c4e7b4272`、
`6df18a9ca52b50bc91de2dd799bd2fb03bc555fe`。
最终提交的 [工作区门禁 `39118679277`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39118679277)
已成功（作业 `117649619099`）。

[维护回归 `39118679289`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39118679289)
六个作业全部成功。已下载三个完成作业的完整日志，逐一核实全部 10 项 20E 用例实际执行，
均为 **10 passed / 0 failed / 0 ignored**，包括三个新增端到端回归。

| 平台 | 最终受控作业 | 20E 完整日志位置 |
| --- | --- | --- |
| Windows | [117649912748](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39118679289/job/117649912748) | 第 2346 行进入测试文件，第 2360 行汇总 10 passed |
| Linux | [117649912755](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39118679289/job/117649912755) | 第 2336 行进入测试文件，第 2350 行汇总 10 passed |
| macOS | [117649912589](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39118679289/job/117649912589) | 第 2201 行进入测试文件，第 2215 行汇总 10 passed |

三平台均核实载荷语言伪装被拒绝、元数据错误先于损坏载荷错误、ABI/平台/入口种类错误先于
诊断启动，且索引层失败审计只计已验证成员。原有版本/模块/能力负例、合法资源要求及
中英文消息同样通过；完整工作区还实际执行了四个格式审核回归和既有模糊测试。
因此本轮自审修复验收完成；没有扩展 18B 的填充/构建范围或改变既定支持边界。
§十一 的旧提交和 CI 保留为历史，不作为本轮修复验收依据。

## 十三、审核附录（2026-10-18）

审核对象：`98a974f`、`b0269f5`、`d37fe07`、`1b8c846`、`6df18a9`、`fe92a37`。
方式是**自己跑**：读 diff、逐项核对索引/预检/协议/载荷契约，重跑定向与全量门禁，
并对一处自审修复做变异验证。

### 13.1 通过的项

| 项 | 证据 |
| --- | --- |
| CI | 最终 `39118679277`/`39118679289` 绿；初始 `39114319897`/`39114319971` 绿；macOS 日志第 2215 行确认 **10 passed**；首次失败 `39113925915` 确认为 `A0-SIZE-001`（三个文件超 2500 行），已由测试拆分修复 |
| 索引 | 字段 13–16、归档 feature=1、`GlobalIndex`=0、字段 16 显式零值、未知位报 011、旧读取器按 feature 拒绝 |
| 身份 | 标准库身份下沉到单一来源；驱动器只重导出 |
| 构造 | 能力并集从 intrinsic 契约推导；未知/墓碑 ID 报错；模块排序去重 |
| 预检 | 直接运行与协议共用；版本 → 模块 → 能力 → ABI → 平台 → `entry.target` → 入口种类，全部在载荷解码与诊断启动之前 |
| 载荷契约 | 编码与读取共用语言/调试检查；禁止内嵌完整调试/源码分区 |
| 入口 | 缺失入口、零长度入口、非 `Xiaoc` 入口统一拒绝 |
| 请求 → VM | `RunOptions.capabilities` 默认全开且旧请求兼容；VM 使用同一掩码；旧 `.xiaoc` 入口保留全开 |
| 审计 | 索引层失败只计 1 个已验证成员；完整审计计实际 members |
| 本地 | workspace **1223 passed / 0 failed / 41 ignored**；`run.sh` exit 0；定向 10/9/22/2；体积回归 2 |
| 变异 | 临时关掉载荷语言检查后，`review_builder_rejects_relabelled_payload_language` 立即红；已还原、工作树干净 |

### 13.2 O-1（低）：请求掩码的错码归因

`xiao-xar/src/runner.rs:394` 把 `options.capabilities` 的未知位也映射为
`X17-XAR-011`（“归档要求无效”）。协议层已在 `protocol/run.rs:106` 把请求掩码
单独报为请求错误，因此 CLI 路径正确；但库层直接调用 `run_archive` 时，
归档本身没有问题却会得到 011。建议把请求掩码与归档掩码分开校验。
**已登记为 18B / 下一批前置。**

后续处理（2026-10-18）：已由 18B 收口提交 `d3ebb0a` 修复，库层未知请求位返回
`XarRunError::Validation/009`，归档未知位仍为 `011`。直接调用回归已实际执行，
见 [18B 收口](18b-build-xar-and-closeout.md) §十二。

### 13.3 O-2（低）：能力名表重复

`xiao-xar/src/runner.rs:429` 手写了 bit 0–6 的七个名字，与 20D 的能力常量/枚举
构成第二份映射。建议在 `xiao-intrinsics` 提供共享的 `capability_name(bit)`，
避免新增位时漂移。

### 13.4 O-3（低）：每次启动都分析标准库

`validate_requirements` 每次归档运行都调用
`xiao_modules::analyze_project(xiao_standard_library::source_root())`。
它是解析/类型检查、不执行，所以语义正确；但一是每次启动都付出分析开销，
二是标准库中任何**无关模块**分析失败都会让所有要求模块被报成
`X17-XAR-008`（缺少模块），诊断指向错误方向。建议至少把“分析失败”映射成
独立错误，或缓存模块清单。

### 13.5 边界与结论

- 18B 的实际可达闭包与 CLI `build -xar` 没有被提前实现（符合裁定四）；
- 没有新增签名/信任策略，没有扩大模块调用范围；
- **无阻断项**；三条低优先级已登记，其中 O-1 进下一批前置。

## 相关页面

- [17. `.xar` 字节码归档与启动](17-xar-archive.md) §二——**直接前置**：索引、启动验证、运行器
- [17A](17a-archive-format-and-codec.md) §2.3——字段 1–6 与「新字段从 7 起」的冻结
- [17C](17c-entry-and-runner.md) §2.3——执行前完整验证与「不得部分执行」
- [20C 官方库](20c-official-xiao-library.md) §3.2——`StandardLibraryIdentity` 五个字段
- [20D-冻结](20d-freeze-resource-contract.md) §2.7——能力位与 `X06-RUNTIME-026`
- [18B 后端调用与工作流](18b-backend-calls-and-workflow.md)——`build -xar` 的字段填充归属

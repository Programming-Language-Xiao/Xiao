# 20O-B3-2. `args` / `cli` 实施规划

> **状态（2026-11-25）**　B3-2A 开工基线固定于 `8d1c09f0`。args 53～54、纯 Xiao 分组、运行协议和分发启动器已进入实施。Windows VM、C ABI 空快照及分发入口已有本机结果。args 原生降低和应用启动快照待生成器裁定，三平台验收与收口待执行。教程和 registry 为 `planned`。[B3-1 收口](assets/20o-b3/entry-20261124-b3-1/README.md) · [B3-2A 基线](assets/20o-b3/entry-20261125-b3-2/README.md) · [开工裁定](assets/20o-b3/entry-20261125-b3-2/decisions.json)
> **版本**　当前为 ABI 1.20、生成器 23、标准库接口 1。args 使用 53～54。cli 为纯 Xiao。
> **Windows 产物政策（2026-11-22）**　自 B3 起，Windows 公共/发布产物必须产品级 COFF `Complete`，声明组件=观察组件；B3-2 不得用同链接、导出表-only 或 `UnverifiedCoffExports` 代替。
> 入口：[B3-1 收口](assets/20o-b3/entry-20261124-b3-1/README.md)、[B3-0F 交接](assets/20o-b3/entry-20261124-b3-0f/README.md)、[B3-0B 契约](assets/20o-b3/entry-20261123-b3-0b/contract-draft.md)、[B3 宿主库规划](20o-b3-host-libraries-plan.md)。
> B3-2 不实现 fs/env/log，不升级教程/registry verified，不启动 B3-3/B4。

## 一、入口与范围

| 项 | B3-2 接收状态 |
| --- | --- |
| B3-1 | `b3_1_passed=true`、`b3_2_entry_ready=true`；产品快照 `8d1c09f0`，ABI 1.20、生成器 23、标准库接口 1 |
| B3-0B | args 的 `program_name`/`argv`、`process` 位、预算和错误已冻结；cli 的 `split_terminator` 为纯 Xiao |
| 当前 intrinsic 表 | 53～54 活动，55～63 为 `reserved` |
| 目标模块 | `args`、`cli` |
| 本批目标 | 激活 args 53～54，接通 `RunOptions.arguments`/原生快照；实现纯 Xiao CLI 分组；完成 VM/原生/C ABI/教程/三平台证据 |
| 本批不做 | fs/env/log 实现、55～63 激活、registry/教程 verified、B3-3/B4、旧 721、历史债务核销 |
| Windows COFF | args/cli 的 Windows 产物必须产品级 `Complete`；完整公共矩阵留 B3-7 |
| 当前状态 | `b3_2_started=true`、`b3_2_passed=false`、`b3_3_entry_ready=false`、`b3_product_started=true` |

B3-2 是 **args/cli 实施批**，不是六模块并行批。args 需要同时接通运行协议、VM/TAC、LLVM/C ABI 和原生启动快照；cli 是纯 Xiao 模块，不新增 intrinsic 或 C 符号。B3-2 完成后只声明 args/cli 内部可用，不升级教程或 registry verified。

### 1.1 要做

1. 固定 B3-1 产品快照 `8d1c09f0`、锁文件、Runtime、工具链和 B1 28 守卫；
2. 将 intrinsic 53～54 从 `reserved` 激活，登记 `os_args_program_name`、`os_args_argv`、`process` 能力、效果和 C 符号；
3. 实现 `RunOptions.arguments` 的 `{program_name, argv}` 快照和 hello `"arguments"` 能力；
4. 接通 VM/TAC、LLVM、C ABI 和原生启动快照；保证空项、重复项、NUL、非法 Unicode 和预算失败；
5. 实现纯 Xiao `std.cli.split_terminator`，按首个 `--` 分组、移除首个分隔符、保留后续项；
6. 让 args/cli 的 B3-0E 教程在 VM 和受控原生 O0/O3 逐字通过；
7. 记录三平台、工具链、Runtime、源码、CI 和原始日志；
8. 形成 B3-2 证据和 B3-3 入口；registry/教程保持 `planned`。

### 1.2 明确不做

- 不激活 55～63，不实现 fs/env/log；
- 不修改 ABI 1.20、生成器 23、接口 1 或已冻结的 time/args/cli API；
- 不把 CLI 做成 intrinsic、宿主库或第二套参数解析入口；
- 不升级 registry/教程 verified，不启动 B3-3/B4；
- 不跑旧 721，不核销 22C/22D、19/11/22 或 B1 历史债务；
- 不用同链接、导出表-only 或 `UnverifiedCoffExports` 替代 Windows 产品级 COFF `Complete`。

### 1.3 结论字段

```text
b3_2_passed =
  b3_2a_entry_and_baseline_verified
  and b3_2b_args_intrinsics_implemented
  and b3_2c_cli_pure_xiao_implemented
  and b3_2d_protocol_vm_native_consistent
  and b3_2e_args_cli_tutorials_passed
  and b3_2f_evidence_audit_passed

b3_3_entry_ready = b3_2_passed and decisions_approved and no_product_scope_creep
```

`b3_2_passed=true` 只表示 args/cli 内部可用和证据通过，不表示 fs/env/log 完成、registry/教程 verified 或 B3 公共验收完成。

## 二、开工闸门

1. B3-1 收口审计、自反例、time 三平台和 COFF `Complete` 通过；`b3_1_passed=true`、`b3_2_entry_ready=true`；
2. 固定产品 `8d1c09f0`、两个 Cargo.lock、Runtime、工具链和 B1 28 守卫可复核；
3. B3-2A 固定产品中的 53～63 为 `reserved`。本批激活 53～54，沿用 1～52 和 ID 100；
4. B3-0B 的 args/cli 契约、预算、错误和教程可读；
5. Rust 1.96.0、三平台目标、VM、受控原生 O0/O3 和 Windows 产品验证器可用；
6. 独立 target/Runtime/缓存、唯一任务根，CLI/env/log 输出隔离；
7. `RunOptions.arguments`、hello `"arguments"` 和旧核心拒绝规则已冻结；
8. 教程/registry 仍为 `planned`，B3-2 不升级 verified。

## 三、已批准开工裁定

| 编号 | 冻结内容 | 来源 |
| --- | --- | --- |
| B3-2-ID | 只激活 53～54；55～63 保持 reserved；不新增 intrinsic、ABI 或生成器编号；新 lowering 若改变生成文本另行批准 | 星崽批准，B3-2 编号激活问题卡 |
| B3-2-PROTO | `RunOptions.arguments` 使用 `{program_name, argv}`；hello 增加 `"arguments"`；旧核心未声明时 `X11-CLI-CORE-004`、退出码 2；协议/Core 仍为 1；原生启动器注入已校验快照 | 星崽批准，B3-2 运行协议问题卡 |
| B3-2-CLI | `stdlib.cli.split_terminator([str]) -> [[str]]` 为纯 Xiao；`std`/`stdlib` 同一语义；无 intrinsic/C 符号 | 星崽批准，B3-2 CLI 与夹具问题卡 |
| B3-2-FIXTURES | args/cli 从 B3-0E 按原字节复制到 `tests/fixtures/docs/{args,cli}/tutorial.*`；args 有 `tutorial.input.json`，cli 使用源内字面量；只登记 args/cli 为 `planned` | 星崽批准，B3-2 CLI 与夹具问题卡 |
| B3-2-EVIDENCE | 三平台 args/cli VM/原生 O0/O3；Windows args/cli 产物取得产品级 COFF `Complete`；完整矩阵留 B3-7 | 星崽批准，B3-2 验收范围问题卡 |

五项答复均已登记来源、时点和适用范围。上游已批准字段和本批开工答案见开工台账。[开工台账](assets/20o-b3/entry-20261125-b3-2/decisions.json)

## 四、分批执行

### B3-2A：入口与基线

1. 固定 B3-1 产品 `8d1c09f0`、ABI 1.20/生成器 23/接口 1、两个 Cargo.lock、B1 28 守卫；
2. 复核 intrinsic 53～54 reserved、55～63 reserved、1～52 active、ID 100 tombstone；
3. 固定 args/cli 教程、夹具、输入、黄金和 B3-0E 精确入口；
4. 建立 `entry-20261125-b3-2/` 证据目录和唯一任务根；
5. 记录 Windows/Linux/macOS 工具、目标和 Runtime；缺平台记 `unverifiable`；
6. B1 守卫的 4 项实际 Cargo 输出和 24 项派生负例引用 B3-1 固定回执。Windows 新任务根的 28 项本机模拟与 0 项实际 Cargo 输出单独登记。

退出：B3-1 基线和 B3-2 入口可复核；未改产品。

### B3-2B：args intrinsic、能力与 Runtime 快照

1. 把 53～54 从 reserved 切为 active，登记名称、arity、参数、返回、效果、能力和 C 符号；
2. 实现 `program_name() -> str` 和 `argv() -> [str]` 的只读运行快照；
3. 实现 `process` 位检查；能力缺失返回 026，类型错误 002；
4. 快照限制：argv 至多 1,024 项，程序名/单项各 4,096 UTF-8 字节，合计 65,536 字节；
5. 空串、空项和重复项原样；NUL/非法 Unicode/超预算在执行前拒绝，错误 004/005；
6. 返回数组为调用者拥有，修改返回值不影响快照；
7. 子任务共享同一快照，不读宿主 argv；嵌入省略为空快照。

退出：args Runtime 快照、能力和边界通过；未接协议/VM/原生。

### B3-2C：纯 Xiao `cli`

1. 创建 `stdlib/cli.xiao`，实现 `split_terminator([str]) -> [[str]]`；
2. 首个 `--` 前为第一组、之后为第二组；移除首个分隔符，后续 `--` 原样保留；
3. 无分隔符时第一组为全部输入，第二组为空；空输入返回两个空数组；
4. 输入至多 1,024 项、每项 4,096 字节、合计 65,536 字节；超限报 `CLI_LIMIT_EXCEEDED`；
5. 使用现有数组能力，不新增 intrinsic/C 符号；返回两组新数组，保留项字节不变；
6. 通过 `std`/`stdlib` 导入路径探测，保持 registry/UseDocs `planned`；
7. 不实现完整选项解析、默认值或帮助生成。

退出：CLI 纯 Xiao 分组、预算和双路径通过；未执行产品教程。

### B3-2D：协议、VM/TAC、LLVM 与 C ABI

本机已验证 `RunOptions.arguments`、hello 能力、源码与删源后的 `.xiaoc` / `.xar` 运行，以及两个 C 入口的空快照和输出槽。分发包的 Rust 启动器标记原始参数无法解码的位置，Bun CLI 按冻结顺序接纳整份应用参数。实际目标采用调用时拼写。

应用原生入口需要发射 args C 调用并在用户代码运行前设置快照。cli 的数组入口检查也需要发射对应调用。按 B3-2-ID，这些 LLVM 生成文本变更及生成器 23 → 24 待单独裁定，裁定范围包含启动接线、版本和缓存复验。当前原生后端拒绝 `stdlib.cli` 的构建，VM 入口已执行类型检查。

1. 在 `RunOptions` 增加可选 `arguments` 字段，形状 `{program_name, argv}`；省略为空快照；
2. 在 hello 响应增加 `"arguments"` 能力；旧核心未声明时客户端执行前拒绝；
3. VM/TAC 识别 53～54，保留 1～52 原义；`process` 能力检查在触碰快照前完成；
4. C ABI 实现 `xiao_runtime_args_program_name(out, capabilities)` 和 `xiao_runtime_args_argv(out, capabilities)`；
5. LLVM 动态/静态 lowering 使用同一快照；直接原生入口注入已校验 argv，嵌入/归档省略为空；
6. 保持 0/1/2/3/6/8 状态、输出槽和借读/拥有规则；
7. 记录协议/Core 版本、入口符号和原生上下文；
8. 本机基线的 clang 默认目标为 `x86_64-w64-windows-gnu`。Windows args/cli 原生 O0/O3 构建须显式传入 `-target x86_64-pc-windows-msvc`，使用同目标 Runtime，并按产物归档实际目标及完整编译、链接命令。

退出：args 的协议、VM/TAC、LLVM/C ABI 和原生快照一致。

### B3-2E：教程、边界与三平台

1. 迁入 args/cli 的 `tutorial.xiao`、`tutorial.stdout`，args 附 `tutorial.input.json`；
2. VM 与原生 O0/O3 覆盖 `std`/`stdlib` 双路径，逐字比对 UTF-8/LF 黄金；
3. args 覆盖空项、重复项、程序名、空快照、非法 Unicode/NUL、单项/总预算和能力缺失；
4. cli 覆盖首项、中部、尾项、无分隔符、开头/末尾分隔符、空输入和预算；
5. 三平台实跑；缺平台记 `unverifiable`；
6. 按 §B3-2D 的 MSVC 目标回执逐份核对 Windows args/cli 产物，取得产品级 COFF `Complete`，绑定 map 和产物身份；
7. registry/教程保持 `planned`。

退出：args/cli 教程、边界和三平台通过；产品 COFF 等级如实登记。

### B3-2F：证据、收口与 B3-3 入口

1. 生成 B3-2 证据：intrinsic、Runtime、协议、VM/TAC、LLVM/C ABI、CLI 源码、教程、平台和 CI；
2. 只读审计核对固定 Git 对象、B3-1 产品、B3-0E 教程、53～54 状态和实际产品源码；
3. 自反例覆盖伪造 args 激活、错误预算、非法 Unicode 放行、CLI 分组错误、伪造教程执行和伪造 COFF 等级。归档逐项执行回执及审计前后摘要，引用 B3-2A 的独立自反例回执；
4. 生成 B3-3 入口：args/cli 已验证、fs 待实现、COFF 待验证项；
5. B3-3/B4 未开始；旧 721 和历史债务仍后置；
6. 定稿后单次 docs 门禁。

退出：B3-2 证据完整；B3-3 入口明确；没有 B3-3/B4、registry/教程 verified 或历史债务变化。

## 五、验收表

| 层 | 必测 |
| --- | --- |
| intrinsic | 53～54 active；55～63 reserved；1～52、ID 100 不变 |
| args | 程序名/argv 快照、空项/重复项、非法 Unicode/NUL、预算、`process`、002/026/004/005 |
| cli | 首个/后续 `--`、无分隔符、空输入、预算、双路径、纯 Xiao 无 intrinsic |
| 协议 | `RunOptions.arguments`、hello `"arguments"`、旧核心拒绝、嵌入空快照 |
| VM/TAC | 53～54 分派、能力检查、返回数组所有权、错误身份 |
| LLVM/C ABI | 两个 args 符号、输出槽、状态码、原生快照注入 |
| 教程 | `tests/fixtures/docs/{args,cli}/tutorial.*`、std/stdlib、VM/原生黄金、退出码 0 |
| 平台 | Windows x86_64 MSVC、Linux x86_64 GNU、macOS arm64；Windows COFF `Complete` |
| 状态 | registry/UseDocs/教程 `planned`；无 B3-3/B4 |
| 证据 | 源码/Runtime/工具/目标/任务根/CI run/job/attempt |

## 六、退出条件

1. B3-1 产品基线和 B3-2 入口固定；53～54 激活、55～63 仍 reserved；
2. args 的协议、Runtime、VM/TAC、LLVM/C ABI 和原生快照一致；
3. cli 为纯 Xiao，无新 intrinsic/C 符号，分组和预算符合契约；
4. 空项、重复项、非法 Unicode/NUL、预算和能力错误通过；
5. args/cli 教程在 VM 和受控原生 O0/O3 逐字通过；缺平台如实登记；
6. Windows args/cli 产物达到产品级 COFF `Complete`；完整矩阵留 B3-7；
7. 证据审计和自反例通过；没有 registry/教程 verified、B3-3/B4 或历史债务变化；
8. `b3_2_passed=true`、`b3_3_entry_ready=true`。

## 七、风险与禁止

- 提前激活 55～63 或把 reserved 入口暴露为可调用；
- 让 args 读取宿主 argv 而不是运行快照；
- Windows 二次拆引号、非法 Unicode 有损替换或吞掉 NUL；
- 把 cli 做成 intrinsic、宿主库或第二套参数解析入口；
- 只做 VM 或只做原生，导致协议/能力不一致；
- 用同链接、导出表-only 或 `UnverifiedCoffExports` 冒充 Windows `Complete`；
- 在 B3-2 顺手实现 fs/env/log 或升级 registry/教程。

## 八、B3-3 交接

B3-3 接收 args/cli 已验证、fs 待实现和 COFF 待验证项。B3-2 不得修改 fs/env/log 的冻结 API；发现 args/cli 与 B3-0B 冲突回 B3-0 修订并重新冻结。

交接必须包含：

- args 的 53～54、C 符号、能力、协议和原生快照；
- cli 的纯 Xiao 源码、双路径和预算；
- 错误、资源、平台和 COFF 实际等级；
- args/cli 教程、源码、黄金和实际执行日志；
- B3-3 入口和明确延期项。

## 九、落点

```text
docs/DevDocs/20o-b3-2-args-cli-implementation-plan.md   本规划
docs/DevDocs/assets/20o-b3/entry-20261125-b3-2/          执行时建立的证据
docs/DevDocs/20o-b3-1-time-implementation-plan.md        前批实施
core/rust/crates/xiao-intrinsics/intrinsics.json         args 53～54 激活
core/rust/crates/xiao-runtime/src/                       args Runtime/C ABI
core/rust/crates/xiao-vm/src/semantics/                  args VM/TAC
core/rust/crates/xiao-codegen-llvm/src/                  args 原生 lowering
core/rust/crates/xiao-driver/src/protocol/                arguments 协议
stdlib/cli.xiao                                          纯 Xiao 分组
tests/fixtures/docs/{args,cli}/                          教程最终夹具
docs/module-registry.json                                只读核对 planned，不升级
```

本页登记 B3-2 实施范围和进度。B3-3 入口由本批最终审计产生。

## 十、新 Agent 先读

1. [B3-0B 契约](assets/20o-b3/entry-20261123-b3-0b/contract-draft.md)；
2. [B3-1 收口](assets/20o-b3/entry-20261124-b3-1/README.md)；
3. [B3-0F 交接](assets/20o-b3/entry-20261124-b3-0f/README.md)；
4. [B3-0D 版本/C ABI](assets/20o-b3/entry-20261124-b3-0d/README.md)；
5. [B3 宿主库规划](20o-b3-host-libraries-plan.md)；
6. [20D 能力上下文](20d-capability-context.md)。

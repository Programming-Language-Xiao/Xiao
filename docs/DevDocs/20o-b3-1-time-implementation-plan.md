# 20O-B3-1. `time` 实施规划

> **状态（2026-11-24）**　B3-0F 已完成联合冻结审计，`b3_0_passed=true`、`b3_1_entry_ready=true`。time 进入开工裁定，`b3_1_started=false`、`b3_1_passed=false`、`b3_2_entry_ready=false`、`b3_product_started=false`。[B3-0F 交接](assets/20o-b3/entry-20261124-b3-0f/README.md)
> **输入契约**　15 个原语为 49～63，time 为 49～52；能力位 `clock` 为 1，`stdout` 为 5；目标为 ABI 1.20、生成器 23、标准库接口 1；C ABI 采用末尾 `capabilities: u64` 的冻结形式。
> **Windows 产物政策（2026-11-22）**　自 B3 起，Windows 公共/发布产物必须产品级 COFF `Complete`，声明组件=观察组件；B3-1 只实现 time，不得把 B3-0E 候选或 `UnverifiedCoffExports` 写成产品验收。
> 入口：[B3-0E 收口](assets/20o-b3/entry-20261124-b3-0e/README.md)、[B3-0D 收口](assets/20o-b3/entry-20261124-b3-0d/README.md)、[B3-0 实施规划](20o-b3-0-probe-freeze-plan.md)、[B3 宿主库规划](20o-b3-host-libraries-plan.md)。
> B3-1 不改 args/cli/fs/env/log 的冻结 API；发现冲突回 B3-0 修订并重新冻结。

## 一、入口与范围

| 项 | B3-1 接收状态 |
| --- | --- |
| B3-0E | `b3_0e_passed=true`、`b3_0f_entry_ready=true`；六份教程和精确入口候选已冻结，教程仍为 `planned` |
| B3-0F | 联合冻结表和只读审计已收口，`b3_1_entry_ready=true`。产品开工按 §3 裁定 |
| 基线 | 固定产品 `e79fab3c`、ABI 1.19、生成器 22、标准库接口 1；两个 Cargo.lock 和 B1 28 守卫可复核 |
| 目标模块 | 仅 `time`：`utc_now`、`monotonic`、`format_utc`、`sleep` |
| 目标版本 | ABI 1.20、生成器 23、标准库接口 1；49～52 激活，53～63 的激活策略见首轮裁定 |
| 本批不做 | args/cli/fs/env/log 实现、教程/registry verified、B3-2、B4-B9、旧 721、历史债务核销 |
| Windows COFF | B3-1 产品产物必须最终达到 `Complete`；B3-1 只处理 time 产物，完整三平台验收在 B3-7 |
| 当前状态 | `b3_1_started=false`、`b3_1_passed=false`、`b3_2_entry_ready=false`、`b3_0_passed=true` |

B3-1 实施 `time` 的四个入口。B3-0F 提供已冻结的六模块契约与验收入口，B3-1 的产品开工以本页 §3 为准。

### 1.1 要做

1. 完成 B3-0F 的冻结证据、审计和 B3-1 入口签收；
2. 按 B3-0D 冻结表登记 time 的 intrinsic 49～52、C 符号、效果和能力；
3. 实现 `utc_now`、`monotonic`、`format_utc`、`sleep` 的 Runtime、VM/TAC 和原生 lowering；
4. 实现 4 个 C ABI 入口，保持借用输入、输出槽、状态码和 `capabilities` 位置；
5. 实现 `clock` 能力检查、错误身份、格式化边界、sleep 上限、取消/超时和安全点；
6. 让 B3-0E 的 time 教程在 VM 与受控原生 O0/O3 逐字通过；
7. 记录三平台、工具链、Runtime、源码和原始日志身份；
8. 形成 B3-1 证据和 B3-2 入口；registry/教程保持 `planned`。

### 1.2 明确不做

- 不在 B3-0F 通过前写产品代码；
- 不实现 args/cli/fs/env/log 的行为或激活 53～63（除非首轮裁定选择预登记 reserved）；
- 不修改 1～48、ID 100 或已验证的 ABI 语义；
- 不把教程/registry 升级为 verified，不启动 B3-2/B4；
- 不跑旧 721，不核销 22C/22D、19/11/22 或 B1 历史债务；
- 不用同链接、导出表-only 或 `UnverifiedCoffExports` 替代 Windows 产品级 COFF `Complete`。

### 1.3 结论字段

```text
b3_1_passed =
  b3_1a_entry_and_baseline_verified
  and b3_1b_time_intrinsics_implemented
  and b3_1c_vm_tac_native_consistent
  and b3_1d_c_abi_verified
  and b3_1e_time_tutorial_and_boundaries_passed
  and b3_1f_evidence_audit_passed

b3_2_entry_ready = b3_1_passed and decisions_approved and no_product_scope_creep
```

`b3_1_passed=true` 只表示 time 内部可用和证据通过，不表示六模块完成、registry/教程 verified 或 B3 公共验收完成。

## 二、开工闸门

1. B3-0E 收口审计、自反例和 B3-0F 入口可读；B3-0F 的冻结表、证据审计和 B3-1 入口签收通过；
2. 固定产品 `e79fab3c`、两个 Cargo.lock、B1 28 守卫和当前 ABI/生成器/接口可复核；
3. B3-0D 的 49～52 编号、C 符号、能力位、C ABI 签名、ABI 1.20/生成器 23 和 COFF 政策可读；
4. B3-0E 的 time 教程、源码、黄金和输入可读，当前状态为 `planned`；
5. Rust 1.96.0、三平台目标、VM 入口和受控原生 O0/O3 工具链可用；缺平台只能记 `unverifiable`；
6. 独立 target/Runtime/缓存和唯一任务根；
7. Windows 产物验证器可执行；B3-1 产物必须按 `complete_required` 评价，未达 `Complete` 只能记失败；
8. 无产品改动先于 B3-0F 签收；registry/教程仍为 `planned`。

## 三、首轮裁定

| 编号 | 候选 | 必须由星崽定夺 |
| --- | --- | --- |
| B3-1-ID | 使用 ABI 1.20、生成器 23 和标准库接口 1，激活 49～52。53～63 的编号保存在 B3-0D 冻结表，后续版本按各批新增 ABI 与 lowering 变化再裁定 | 与 B3-1-RESERVE 择一 |
| B3-1-RESERVE | 激活 49～52，另为 53～63 在产品表增加不可调用的 `reserved` 状态。产品表现有状态是 `active` 与 `tombstone` | 与 B3-1-ID 择一，后续版本仍按实际变化裁定 |
| B3-1-CLOCK | 宿主实际精度、墙钟回拨、单调钟起点和回退钳制沿 B3-0B1a/B3-0B4b | 已有冻结答复 |
| B3-1-SLEEP | 上限 60 秒，零值先查运行控制，安全点按取消、截止、完成排序。错误身份和性能阈值沿 B3-0B1b/B3-0B4b/B3-0B4c | 已有冻结答复 |
| B3-1-EVIDENCE | B3-1 执行三平台 time 用例和受控原生 O0/O3。Windows time 产物逐份取得产品验证器 `Complete`，B3-7 执行完整公共产物矩阵 | 裁定 B3-1 的证据范围 |
| B3-1-FIXTURES | B3-1E 迁入 `tests/fixtures/docs/time/` 需要同步目录 README 与 registry 路径登记。已有同路径候选的布局失败回执 | 裁定是否在 B3-1 增加 `planned` 状态的最小路径登记，或把正式夹具迁入留给 B3-7 |

首轮裁定登记来源、时点和适用范围。CLOCK 与 SLEEP 沿用 B3-0B 的时钟和运行控制契约。正式夹具登记取决于布局门禁规则与本批迁入范围。[time 冻结契约](assets/20o-b3/entry-20261123-b3-0b/contract-draft.md) · [布局失败回执](assets/20o-b3/entry-20261123-b3-0b/layout-first.json)

## 四、分批执行

### B3-1A：B3-0F 入口与基线

1. 完成 B3-0F 的固定证据、只读审计、自反例和 B3-1 入口；
2. 固定 B3-0E 教程/夹具、B3-0D 编号/版本、B3-0A 三平台回执和 B1 28 守卫；
3. 固定产品 `e79fab3c`、两个 Cargo.lock、Runtime/工具/目标；
4. 记录当前 ABI 1.19/生成器 22/接口 1，确认 B3-1 的产品目标；
5. 建立 `entry-20261124-b3-1/` 证据目录和唯一任务根；
6. B3-0F 未通过前不写产品。

退出：B3-0F 收口和 B3-1 入口可复核；产品树仍为基线。

### B3-1B：intrinsic、能力与 Runtime time 核心

1. 按裁定登记 49～52；如选择 reserved，则登记 53～63 但不可调用；
2. 在 intrinsic 表登记名称、参数、返回、效果、能力、C 符号和错误；
3. 实现 `utc_now`、`monotonic`、`format_utc`、`sleep` 的 Runtime 核心；
4. 实现 `clock` 能力检查和 `stdout` 无关的 time 边界；
5. 实现单位 `ms`/`ns`、UTC 格式域、毫秒/纳秒小数位、负时间和范围检查；
6. 实现 sleep 上限、零值、负值、取消/截止、安全点和清理；
7. 记录 Runtime 源码、构建身份和单元测试。

退出：time Runtime 核心通过；未接 VM/原生/C ABI。

### B3-1C：VM、TAC 与字节码

1. 让 TAC/字节码识别 49～52 并保留 1～48 原义；
2. VM 分派在触碰宿主前检查 `clock` 能力；
3. 实现 UTC/单调钟/格式化的值返回和拥有字符串；
4. 实现 sleep 的取消/超时/返回 none；
5. 让错误 002/026/012/009、`TIME_SLEEP_LIMIT_EXCEEDED` 与 span/catch/finally/release 一致；
6. 单元测试覆盖能力缺失、非法单位、负时长、格式域、溢出和取消。

退出：VM/TAC time 行为与冻结契约一致。

### B3-1D：LLVM、C ABI 与原生入口

1. 实现 4 个 C ABI 签名和 `xiao_runtime_time_*` 符号；
2. 保持借读输入、输出槽、末尾 capabilities 和 0/1/2/3/6/8 状态；
3. LLVM 动态/静态 lowering 调用对应 Runtime 入口，不绕过能力检查；
4. 原生 O0/O3 的输出、退出码和错误与 VM 一致；
5. Windows time 产物使用 B3-0D 允许的来源完成产品验证器正向实测，记录 `Complete`、组件等式和产物身份绑定。B3-7 接收完整公共产物矩阵；
6. 记录工具链、Runtime 和产物身份。

退出：C ABI、LLVM 和原生的 time 入口一致。

### B3-1E：教程、边界与三平台

1. 按 B3-1-FIXTURES 裁定确定 time 夹具落点，逐字执行 B3-0E 的冻结示例；
2. VM 与原生 O0/O3 覆盖 `std.time`/`stdlib.time` 两个导入前缀；
3. 覆盖固定墙钟、单调关系、UTC 格式边界、sleep 0/1 ns/1 ms/60 s/越界、取消与截止；
4. 记录三平台实际结果；缺平台记 `unverifiable`；
5. 只对 time 升级教程/registry 的范围需另行裁定；B3-1 默认保持 `planned`；
6. 不把候选解析当作产品执行。

退出：time 教程和边界测试通过；registry/教程状态未越界。

### B3-1F：证据、收口与 B3-2 入口

1. 生成 B3-1 证据清单：intrinsic、Runtime、VM/TAC、LLVM/C ABI、教程、边界、平台、工具链和原始日志；
2. 只读审计核对固定 Git 对象、B3-0 冻结表、B3-0E 教程和实际产品源码；
3. 自反例覆盖伪造 time 状态、缺能力检查、错误身份替换、伪造教程执行和伪造 COFF 等级；
4. 生成 B3-2 入口：time 已验证、args/cli 待实现、COFF 待验证项；
5. B3-2/B4 未开始；旧 721 和历史债务仍后置；
6. 定稿后单次 docs 门禁。

退出：B3-1 证据完整；B3-2 入口明确；没有 B3-2/B4、registry/教程 verified 或历史债务变化。

## 五、验收表

| 层 | 必测 |
| --- | --- |
| intrinsic | 49～52 名称/ID/C 符号/参数/效果/能力；1～48 和 ID 100 不变 |
| Runtime | 墙钟、单调钟、UTC 格式化、sleep、单位、范围、错误、清理 |
| 能力 | `clock` 缺失 → 026；位 0～7 不变；time 不误用 `environment` |
| 取消 | sleep 0/负/上限、取消与到期、根/子任务错误、安全点、finally/release |
| VM/TAC | 49～52 分派、输出所有权、错误身份、无宿主绕过 |
| LLVM | 动态/静态入口、O0/O3、能力检查、输出/退出码 |
| C ABI | 4 个符号、借用/输出槽、状态码、无别名、异常/Fatal 边界 |
| 教程 | `tests/fixtures/docs/time/tutorial.*`、std/stdlib、VM/原生黄金、退出码 0 |
| 平台 | Windows x86_64 MSVC、Linux x86_64 GNU、macOS arm64；Windows COFF `complete_required` |
| 证据 | 源码/Runtime/工具/目标/任务根/原始日志/CI run/job/attempt |
| 边界 | 无其他模块实现、无 registry/教程 verified、无 B3-2/B4、无旧 721 |

## 六、退出条件

1. B3-0F 入口签收通过，B3-1 基线固定；
2. time intrinsic 49～52 与 1～48/ID 100 身份一致；
3. Runtime、VM/TAC、LLVM、C ABI 的 time 行为一致；
4. 单位、格式、范围、sleep、取消/超时、错误和资源边界通过；
5. time 教程在 VM 和受控原生 O0/O3 逐字通过；缺平台如实登记；
6. 证据审计和自反例通过；没有 registry/教程 verified、B3-2/B4 或历史债务变化；
7. `b3_1_passed=true`、`b3_2_entry_ready=true`；Windows 产品 COFF 若未达 `Complete`，不得宣称 B3 产品验收完成。

## 七、风险与禁止

- B3-0F 未签收就写产品；
- 把 49～63 全部暴露为可调用而只实现 time；
- 只在 VM 实现，原生绕过能力检查；
- sleep 无上限、不可取消或吞掉取消/截止；
- 用真实用户时区、终端或环境实现 time；
- 把教程解析/黄金检查写成产品执行；
- 把 `UnverifiedCoffExports`、同链接或导出表-only 写成 Windows `Complete`；
- 在 B3-1 顺手实现 args/cli/fs/env/log 或升级 registry/教程。

## 八、B3-2 交接

B3-2 接收 time 已冻结、已验证和未完成项。B3-1 不得修改其他五模块的冻结 API；发现 time 与 B3-0B/C 冲突回 B3-0 修订并重新冻结。

交接必须包含：

- time 的 intrinsic、C 符号、Runtime/VM/LLVM/C ABI 身份；
- time 错误、能力、取消、资源和平台结果；
- time 教程、源码、黄金和实际执行日志；
- Windows COFF 的实际等级与缺项；
- B3-2 入口和明确延期项。

## 九、落点

```text
docs/DevDocs/20o-b3-1-time-implementation-plan.md   本规划
docs/DevDocs/assets/20o-b3/entry-20261124-b3-1/      执行时建立的证据
docs/DevDocs/20o-b3-0-probe-freeze-plan.md           B3-0 总规划
docs/DevDocs/20o-b3-host-libraries-plan.md           B3 宿主库规划
core/rust/crates/xiao-intrinsics/intrinsics.json     time 编号和契约
core/rust/crates/xiao-runtime-abi/src/lib.rs         ABI 1.20 增量
core/rust/crates/xiao-runtime/src/                   time Runtime/C ABI
core/rust/crates/xiao-vm/src/semantics/exec.rs       VM 分派
core/rust/crates/xiao-codegen-llvm/src/              LLVM 原生入口
tests/fixtures/docs/time/                            time 教程最终夹具
docs/module-registry.json                            time 夹具路径登记待 B3-1-FIXTURES 裁定
```

本页只做 B3-1 规划；B3-2 及后续仍未开始。

## 十、新 Agent 先读

1. [B3-0 探测/冻结规划](20o-b3-0-probe-freeze-plan.md)；
2. [B3-0E 收口](assets/20o-b3/entry-20261124-b3-0e/README.md)；
3. [B3-0D 收口](assets/20o-b3/entry-20261124-b3-0d/README.md)；
4. [B3-0B 契约](assets/20o-b3/entry-20261123-b3-0b/contract-draft.md)；
5. [B3 宿主库规划](20o-b3-host-libraries-plan.md)；
6. [20D 能力上下文](20d-capability-context.md)。
7. [B3-0F 联合冻结交接](assets/20o-b3/entry-20261124-b3-0f/README.md)。

# 20O-B3-3. `fs` 实施规划

> **状态（2026-11-27）**　排他重命名修复 `2460e75f` 通过三平台专项。并发释放修复后的产品 `3506455e` 也通过 fs 排他重命名与三平台教程复验，Windows 六份产物达到产品级 COFF `Complete`。`b3_3_passed=true`、`b3_4_entry_ready=true`、`b3_4_started=true`。[原收口](assets/20o-b3/entry-20261125-b3-3-final/README.md) · [修复复验](assets/20o-b3/entry-20261126-b3-3-rename-fix/README.md) · [新产品复验](assets/22d-fix2/entry-20261126/README.md)
> **产品身份**　fs 使用 55～59。原生接线按补充裁定使用 ABI 1.20、生成器 25、标准库接口 1。[生成器裁定](assets/20o-b3/entry-20261125-b3-3/generator-decision.json)
> **历史身份（2026-11-27 补记）**　B3-4C 已按 B3-4-GEN 升生成器 27。本页 `3506455e`/生成器 26（及原 B3-3 的 25）仍只代表 B3-3 历史证据；B3-4D/E 只声明 env，B3-7 在最终产品重跑六模块矩阵后再更新 fs 当前结论。
> **Windows 产物政策（2026-11-22）**　自 B3 起，Windows 公共/发布产物必须产品级 COFF `Complete`，声明组件=观察组件；B3-3 只处理 fs 产物，完整公共矩阵留 B3-7。
> 入口：[B3-2 收口](assets/20o-b3/entry-20261125-b3-2-final/README.md)、[B3-0C 契约](assets/20o-b3/entry-20261123-b3-0c/contract-draft.md)、[B3-0F 交接](assets/20o-b3/entry-20261124-b3-0f/README.md)、[B3 宿主库规划](20o-b3-host-libraries-plan.md)。
> B3-3 不实现 env/log，不升级教程/registry verified，不启动 B3-4/B4。

## 一、入口与范围

| 项 | B3-3 接收状态 |
| --- | --- |
| B3-2 | `b3_2_passed=true`、`b3_3_entry_ready=true`；产品快照 `879ce625`，ABI 1.20、生成器 24、标准库接口 1 |
| B3-0C | fs 的五个公开入口、路径、原子写、删除、重命名、元数据、错误、预算和平台差异已冻结 |
| 入口 intrinsic 表 | 55～63 为 `reserved`。B3-3 的激活范围为 55～59 |
| 目标模块 | `fs` 扩展 |
| 本批目标 | 激活 fs 55～59，实现普通宿主模式、原子写、枚举、删除、重命名、元数据和 C ABI；完成 VM/原生/教程/三平台证据 |
| 本批不做 | env/log 实现、受限/沙箱模式、60～63 激活、registry/教程 verified、B3-4/B4、旧 721、历史债务 |
| Windows COFF | fs 的 Windows 产物必须产品级 `Complete`；完整矩阵留 B3-7 |
| 当前状态 | `b3_3_started=true`、`b3_3_passed=true`、`b3_4_entry_ready=true`、`b3_4_started=true`；B3-4 见 [env 实施规划](20o-b3-4-env-implementation-plan.md) |

B3-3 是 **fs 实施批**，不是六模块并行批。普通宿主模式是首批唯一模式；受限/沙箱模式在 B3-0C 已明确延期，不得在 B3-3 临时扩大能力。B3-3 完成后只声明 fs 内部可用，不升级教程或 registry verified。

### 1.1 要做

1. 固定 B3-2 产品 `879ce625`、锁文件、Runtime、工具、三平台回执和 B1 28 守卫；
2. 把 intrinsic 55～59 从 `reserved` 激活，登记 `os_fs_*`、`filesystem` 能力、效果和 C 符号；
3. 实现 `write_text`、`list_dir`、`remove`、`rename`、`metadata` 的 Runtime 核心；
4. 接通 VM/TAC、LLVM、C ABI 和原生入口；
5. 冻结并验证路径 4,096 字节、写入 8 MiB、枚举 1,024/4,096/65,536 字节、10 秒操作期限；
6. 让 B3-0E 的 fs 教程在 VM 和受控原生 O0/O3 逐字通过；
7. 记录三平台、工具、Runtime、源码、CI 和原始日志；
8. 形成 B3-3 证据和 B3-4 入口；registry/教程保持 `planned`。

### 1.2 明确不做

- 不激活 60～63，不实现 env/log；
- 不提供受限/沙箱模式；不扩大宿主权限或路径能力；
- 不修改 ABI 1.20、标准库接口 1 或已冻结的 time/args/cli/fs API；
- 不把 `std.fs` 做成第二套 `std.os` 句柄 API；
- 不升级 registry/教程 verified，不启动 B3-4/B4；
- 不跑旧 721，不核销 22C/22D、19/11/22 或 B1 历史债务；
- 不用同链接、导出表-only 或 `UnverifiedCoffExports` 替代 Windows 产品级 COFF `Complete`。

### 1.3 结论字段

```text
b3_3_passed =
  b3_3a_entry_and_baseline_verified
  and b3_3b_fs_intrinsics_implemented
  and b3_3c_vm_tac_llvm_c_abi_native_consistent
  and b3_3d_fs_tutorial_and_boundaries_passed
  and b3_3e_evidence_audit_passed

b3_4_entry_ready = b3_3_passed and decisions_approved and no_product_scope_creep
```

`b3_3_passed=true` 只表示 fs 普通宿主模式内部可用和证据通过，不表示受限模式、env/log、registry/教程 verified 或 B3 公共验收完成。

## 二、开工闸门

1. B3-2 收口审计、自反例、三平台和 COFF `Complete` 通过；`b3_2_passed=true`、`b3_3_entry_ready=true`；
2. 固定产品 `879ce625`、两个 Cargo.lock、Runtime、工具链和 B1 28 守卫可复核；
3. 开工快照中的 55～63 为 `reserved`。本批核对 1～54 和 ID 100 的固定身份；
4. B3-0C 的 fs 签名、路径、原子写、错误、预算和平台表可读；
5. Rust 1.96.0、三平台目标、VM、受控原生 O0/O3 和 Windows 产品验证器可用；
6. 独立 target/Runtime/缓存、唯一任务根；fs 只操作任务专属临时目录；
7. Windows args/cli 的生成器 24 记录已登记。改变生成文本的 fs 原生接线依 B3-3-ID 申请生成器 25；
8. 教程/registry 仍为 `planned`，B3-3 不升级 verified。

## 三、已批准开工裁定

| 编号 | 冻结内容 | 来源 |
| --- | --- | --- |
| B3-3-ID | 只激活 55～59；60～63 保持 reserved；ABI 1.20、标准库接口 1 不变；生成器 25 只在 fs lowering 改变生成文本时申请 | 星崽批准，B3-3 编号与版本问题卡 |
| B3-3-FS | 首批只做普通宿主模式；相对路径按 cwd、绝对路径/`..`/中间链接交宿主；禁止覆盖时 `FS_EXISTS`；同目录原子写不可保证时 `FS_UNSUPPORTED`；受限模式继续延期 | 星崽批准，B3-3 fs 边界问题卡 |
| B3-3-CABI | 五个 `xiao_runtime_fs_*` 符号；`write/remove/rename` 无输出槽，`list_dir/metadata` 有 `out`；末尾 `capabilities:u64`；状态 0/1/2/3/6/8 | 星崽批准，B3-3 C ABI 问题卡 |
| B3-3-FIXTURES | fs 教程迁入 `tests/fixtures/docs/fs/tutorial.*`；无输入 JSON；任务入口创建空 `demo`，结束检查为空；只登记 fs 为 `planned` | 星崽批准，B3-3 夹具与验收问题卡 |
| B3-3-EVIDENCE | 三平台 fs VM/受控原生 O0/O3；Windows fs 产物产品级 COFF `Complete`；完整矩阵留 B3-7 | 星崽批准，B3-3 夹具与验收问题卡 |

五项答复均已登记来源、时点和适用范围；B3-3A 建立证据目录时归档为机器可读台账。未答项不得写成已冻结。

LLVM 接入五个 fs 调用改变生成文本，星崽补充批准生成器 24→25。24 产物依据既有兼容规则可由 25 校验器读入，25 产物由上限为 24 的校验器拒绝。以同一 Runtime 和 LLVM 工具核对缓存指纹。[补充裁定](assets/20o-b3/entry-20261125-b3-3/generator-decision.json)

## 四、分批执行

### B3-3A：入口与基线

1. 固定 B3-2 产品 `879ce625`、ABI 1.20/生成器 24/接口 1、两个 Cargo.lock、B1 28 守卫；
2. 复核 intrinsic 55～59 reserved、60～63 reserved、1～54 active、ID 100 tombstone；
3. 从 B3-0E 固定 fs 教程源码与黄金字节，记录原始摘要。入口基线使用生成器 24，最终验收使用获准的生成器 25；
4. 建立 `entry-20261125-b3-3/` 证据目录和唯一任务根；
5. 记录三平台工具、目标和 Runtime；缺平台记 `unverifiable`。

退出：B3-2 基线和 B3-3 入口可复核；未改产品。

### B3-3B：fs intrinsic、能力与 Runtime 核心

1. 将 55～59 从 reserved 切为 active，登记名称、arity、参数、返回、效果、能力、错误和 C 符号；
2. 实现 `write_text`：同目录暂存、原子提交、显式覆盖、禁止覆盖的提交结果判定、失败清理；
3. 实现 `list_dir`：只枚举直接子项、原始 UTF-8 字节排序、严格解码、超限整份作废；
4. 实现 `remove`：文件、空目录或末段链接，非递归；不跟随末段链接；
5. 实现 `rename`：目录项移动、显式覆盖、跨卷/不支持语义报错；
6. 实现 `metadata`：`kind` 1/2/3/4、`size`、`modified_ms`；
7. 实现 `filesystem` 能力检查、路径/文本/枚举预算、10 秒操作期限和取消安全点；
8. 保持 `FS_EXISTS`、`FS_UNSUPPORTED`、`FS_IO_FAILED`、`FS_LIMIT_EXCEEDED`、`FS_TIMEOUT` 身份。
9. 严格遵循冻结错误顺序：类型 `002`、能力 `026`、参数 `012`、预算、运行控制安全点、宿主操作；能力检查必须先于任何宿主文件系统访问，不能提前于类型检查。

退出：fs Runtime 核心、能力、错误和边界通过；未接 VM/原生/C ABI。

### B3-3C：VM/TAC、LLVM 与 C ABI

1. VM/TAC 识别 55～59，保留 1～54 原义；
2. VM 在触碰宿主前检查 `filesystem`；
3. LLVM 动态/静态 lowering 使用同一 Runtime 入口，不绕过能力检查；
4. 实现五个 C ABI 符号、借读输入、输出槽和末尾 `capabilities:u64`；
5. 保持 0/1/2/3/6/8 状态、所有权和无别名规则；
6. C ABI 失败时保留已初始化输出槽，成功时替换；`ResourceError` 映射到状态 6，类型、形状和句柄错误按冻结状态映射；
7. 原生 O0/O3 与 VM 的路径/错误/取消一致；
8. Windows 原生必须显式使用 `x86_64-pc-windows-msvc` 和同目标 Runtime；COFF 以产品验证器 `Complete` 为准。

退出：fs 的 VM/TAC、LLVM/C ABI 和原生入口一致。

### B3-3D：教程、边界与三平台

1. 迁入 fs 的 `tutorial.xiao`、`tutorial.stdout`；无额外输入；
2. 任务入口创建空 `demo` 目录，结束检查 `demo` 为空；
3. VM 与原生 O0/O3 覆盖 `std.fs`/`stdlib.fs` 双路径，源码与黄金字节绑定 B3-0E，最终运行回执绑定生成器 25；
4. 覆盖禁止覆盖时的已有文件、目录和悬空链接；并发无覆盖写入由多个独立调用争用同一目标时必须恰有一个成功、其余返回 `FS_EXISTS`；覆盖写入遇符号链接时替换链接目录项而不改写其目标；
5. 用故障注入验证暂存准备失败时旧目标保持、临时文件清理、清理失败保留附带原因；通过边界测试证明进入原子提交后返回实际提交结果；
6. 覆盖枚举 UTF-8 字节排序、1,024 项/4,096 字节单项/65,536 字节合计边界、超限整份作废；在允许非 UTF-8 文件名的平台验证严格解码失败且不交付部分数组；
7. 覆盖删除非递归和末段链接本身、重命名末段链接本身与显式覆盖、跨卷拒绝、元数据四类 kind 和尺寸/时间转换溢出；
8. 用受控宿主适配验证调用前、分块/批次间、提交/交付前的取消与期限安全点；阻塞宿主调用不能被中途打断时，验证调用返回后的错误身份和提交结果遵循冻结契约，不把 10 秒安全点描述为可中断系统调用的硬实时保证；
9. 三平台实跑；缺平台记 `unverifiable`；
10. Windows fs 产物取得产品级 COFF `Complete`，绑定 map 和产物身份；
11. registry/教程保持 `planned`。

退出：fs 教程、边界和三平台通过；产品 COFF 等级如实登记。

### B3-3E：证据、收口与 B3-4 入口

1. 生成 B3-3 证据：intrinsic、Runtime、VM/TAC、LLVM/C ABI、教程、平台、工具链和 CI；
2. 只读审计核对固定 Git 对象、B3-2 产品、B3-0E 教程、55～59 状态和实际源码；
3. 自反例覆盖伪造 fs 激活、未检查 `filesystem`、错误覆盖语义、伪造教程执行和伪造 COFF 等级；
4. 生成 B3-4 入口：fs 已验证、env 待实现、COFF 待验证项；
5. B3-4/B4 未开始；旧 721 和历史债务仍后置；
6. 定稿后单次 docs 门禁。

退出：B3-3 证据完整；B3-4 入口明确；没有 B3-4/B4、registry/教程 verified 或历史债务变化。

## 五、验收表

| 层 | 必测 |
| --- | --- |
| intrinsic | 55～59 active；60～63 reserved；1～54、ID 100 不变 |
| fs | os/fs 分工、路径、`..`、绝对路径、符号链接、覆盖、原子写、清理、非递归删除、重命名、元数据 |
| 错误 | 002/026/012/009、`FS_EXISTS`、`FS_UNSUPPORTED`、`FS_IO_FAILED`、`FS_LIMIT_EXCEEDED`、`FS_TIMEOUT` |
| 资源 | 4,096 字节路径、8 MiB 文本、1,024 枚举项、4,096 单项、65,536 总字节、10 秒期限 |
| VM/TAC | 55～59 分派、冻结错误顺序、能力检查、输出所有权、取消与期限安全点 |
| LLVM/C ABI | 五个符号、失败保留/成功替换输出槽、状态码、原生入口、无别名 |
| 教程 | `tests/fixtures/docs/fs/tutorial.*`、B3-0E 源码/黄金摘要、std/stdlib、VM/原生 O0/O3，执行身份为生成器 25，`demo` 为空 |
| 平台 | Windows x86_64 MSVC、Linux x86_64 GNU、macOS arm64；Windows COFF `Complete` |
| 状态 | registry/UseDocs/教程 `planned`；无 B3-4/B4 |
| 证据 | 源码/Runtime/工具/目标/任务根/CI run/job/attempt |

## 六、退出条件

1. B3-2 产品基线和 B3-3 入口固定；55～59 激活、60～63 仍 reserved；
2. fs 五个入口、普通宿主模式、路径/原子写/清理/错误/预算冻结并实现；
3. VM/TAC、LLVM/C ABI、原生 O0/O3 一致；
4. 取消、操作期限、失败关闭和清理通过；
5. fs 教程在 VM 和受控原生 O0/O3 逐字通过；缺平台如实登记；
6. Windows fs 产物达到产品级 COFF `Complete`；完整矩阵留 B3-7；
7. 证据审计和自反例通过；没有 registry/教程 verified、B3-4/B4 或历史债务变化；
8. `b3_3_passed=true`、`b3_4_entry_ready=true`。

## 七、风险与禁止

- 把受限/沙箱模式塞进 B3-3 或默认扩大宿主权限；
- `write_text` 非原子、覆盖语义不清或失败后遗留临时文件；
- `list_dir` 递归或超限部分交付。目录路径按宿主规则解析链接；
- `remove` 递归删除或跟随末段链接；
- `rename` 跨卷/覆盖语义不明确却返回成功；
- 只做 VM 或只做原生，导致能力和错误不一致；
- 用同链接、导出表-only 或 `UnverifiedCoffExports` 冒充 Windows `Complete`；
- 在 B3-3 顺手实现 env/log 或升级 registry/教程。

## 八、B3-4 交接

B3-4 接收 fs 已验证、env 待实现和 COFF 待验证项。B3-3 不得修改 env/log 的冻结 API；发现 fs 与 B3-0C 冲突回 B3-0 修订并重新冻结。

交接必须包含：

- fs 的 55～59、C 符号、能力、错误、预算和平台结果；
- fs 教程、源码、黄金和实际执行日志；
- Windows fs 产物的实际 COFF 等级与缺项；
- B3-4 入口和明确延期项（含受限模式未排期）。

## 九、落点

```text
docs/DevDocs/20o-b3-3-fs-implementation-plan.md   本规划
docs/DevDocs/assets/20o-b3/entry-20261125-b3-3/   执行时建立的证据
docs/DevDocs/assets/20o-b3/entry-20261125-b3-3-final/  三平台收口证据
docs/DevDocs/20o-b3-2-args-cli-implementation-plan.md 前批实施
core/rust/crates/xiao-intrinsics/intrinsics.json  fs 55～59 激活
core/rust/crates/xiao-runtime/src/                fs Runtime/C ABI
core/rust/crates/xiao-vm/src/semantics/           fs VM/TAC
core/rust/crates/xiao-codegen-llvm/src/           fs 原生 lowering
tests/fixtures/docs/fs/                           fs 教程最终夹具
docs/module-registry.json                         fs 登记为 planned
```

B3-4 从通过三平台回验的 `3506455e` 接续，使用 ABI 1.20、生成器 26、标准库接口 1。[新产品复验](assets/22d-fix2/entry-20261126/README.md)

## 十、新 Agent 先读

1. [B3-0C 契约](assets/20o-b3/entry-20261123-b3-0c/contract-draft.md)；
2. [B3-2 收口](assets/20o-b3/entry-20261125-b3-2-final/README.md)；
3. [B3-0F 交接](assets/20o-b3/entry-20261124-b3-0f/README.md)；
4. [B3-0D 版本/C ABI](assets/20o-b3/entry-20261124-b3-0d/README.md)；
5. [B3 宿主库规划](20o-b3-host-libraries-plan.md)；
6. [20D 能力上下文](20d-capability-context.md)。

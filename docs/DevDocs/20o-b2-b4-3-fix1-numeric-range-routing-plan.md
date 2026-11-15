# 20O-B2-B4-3-FIX.1. numeric_range 路由回归修复与 B4-5 重跑规划

> **FIX.1 尚未开始；B4-5 已被 workspace 原生断言阻塞，B4-5C/D、B5～B7 与 B2-C 后置。**
> 触发：B4-5 完整 workspace 在 `n0_b_dynamic_native::generic_value_copy_registers_weak_dependency_only_when_called` 失败；递归用例 `recurse(depth - 1)` 的 `weak` 依赖期望 `true`、实际 `false`。
> 星崽 2026-11-15 裁定：**收窄 numeric_range 豁免到 float pow**，让整数/固定宽度程序恢复原动态路由与 `weak` 依赖，同时保留 B4-3 的 Float `**` 与 cmath 共享内核。
> 只修 codegen-llvm 路由与必要测试；不改 cmath 数学、模块加载器、ABI/版本/registry；不跑旧 721。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| 最后已知通过 | B3-4 workspace 日志（`7d8b85e4`）中该测试为 ok |
| 当前产品树 | B4-4 最终产品 `bb0b8196`，1064 个受保护文件，ABI1.18/生成器21/接口1 |
| 阻塞证据 | `entry-20261115-b4-5/closeout.json`：`blocked=true`、`workspace_gates=blocked_existing_n0_b_dynamic_assertion`、`language_available=false` |
| 失败位置 | `core/rust/crates/xiao-driver/tests/n0_b_dynamic_native.rs:352`，`runtime_components` 缺少 `weak` |
| 路由来源 | `4be30eaf` 的 `dynamic/predicate.rs`：所有仅含 `numeric_range` 检查的程序不再强制动态路径 |
| 覆盖缺口 | B4-3/B4-4 的 core-tests 只跑 codegen/runtime/vm/bytecode/intrinsics/xar，不含 `xiao-driver` 集成测试 |
| 裁定 | 2026-11-15 收窄豁免到 float pow；不保留全量 `numeric_range` 豁免 |
| 本批目标 | 恢复整数 `numeric_range` 程序的动态路由与 `weak`、n0_b 全绿、B4-3 矩阵不回退、B4-5 可重跑 |
| 未做 | B4-5C/D、B5、registry/教程、三平台、性能、旧721 |

FIX.1 是 **路由回归修复批**，不是 B4-5 收口批。B4-5 不修改产品源码的边界保持不变；修复完成并恢复 workspace 门槛后，B4-5 从其现有收口流程重新执行。

### 1.1 要做

1. 归因复跑：在 `7d8b85e4`、`c7a45e10`、`4be30eaf` 三个固定提交上，用各自独立任务根/target/Runtime 直接运行 `n0_b_dynamic_native::generic_value_copy_registers_weak_dependency_only_when_called`，记录 PID、时间、环境、源码前后摘要和日志哈希；确认 `4be30eaf` 是引入点；若结果不符，重定归因并停下。
2. 检查分类：解析该递归用例的 `depth - 1` 与 Float pow 用例各自产生的 `runtime_checks`，列出 kind/span/来源，区分整数或固定宽度的 `numeric_range` 与 Float pow 所需的检查。
3. 收窄 predicate：只对 Float pow 路径豁免 `numeric_range` 强制动态；整数/固定宽度 `numeric_range` 程序恢复原动态路由与 `weak` 注册。不得恢复 `llvm.pow.f64`，不得改变 `cmath_pow` 共享内核。
4. 定向回归：新增用例显式断言「整数 numeric_range-only 程序走动态并注册 weak」与「Float pow 静态路径仍调用 `cmath_pow`、无 `llvm.pow.f64`」两条路由；不得只改既有 n0_b 期望。
5. 回归：`n0_b_dynamic_native` 全绿；B4-3 四组 O0/O3 各 18,318 项与 1,075 组 pow 对账不回退；B1/B2 VM/原生、22c 29/29、B4-4 边界 4 项与双路径/归档回归通过。
6. 完整 workspace 门禁通过；随后把结果交回 B4-5，由 B4-5 重新运行其收口（B4-5C/D 才能继续）。
7. 证据：新建 `docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-3-fix1/`，原始日志按 SHA-256 内容寻址；B4-3/B4-4/B4-5 历史目录不改写；不升 ABI/生成器/接口。

### 1.2 明确不做

- 不删除、放宽、重命名或 `#[ignore]` 既有 `n0_b_dynamic_native` 断言；
- 不改 cmath 数学内核、模块加载器、官方模块允许表、导入/归档语义；
- 不升 ABI 1.19、生成器保持21、接口保持1，不改 registry/教程/锁；
- 不跑旧 721，不关闭 22C/22D、19/11/22 历史债务；
- 不在 B4-5 收口批里顺手修产品，也不把 FIX.1 写成三平台或发布验收。

## 二、开工闸门

1. B4-5 阻塞证据与失败日志可读；失败用例可复现；
2. `7d8b85e4`、`c7a45e10`、`4be30eaf` 固定 Git 对象可读；
3. 星崽 2026-11-15「收窄到 float pow」裁定已记录；
4. 唯一任务根、独立 target/Runtime/缓存已建立；
5. 当前 B4-3、B4-4、B4-5 只读审计可复核；
6. 修复面限定在 `dynamic/predicate.rs` 与必要测试，未冻结项不得先写产品后补契约。

## 三、分批执行

### FIX-0：归因复跑

1. 三个固定提交各跑一次单项测试，保存实际日志、PID、环境、源码/DevDocs 摘要与哈希；
2. 对比 `predicate.rs` 改动与失败路径，确认 `4be30eaf` 引入；
3. 若 `c7a45e10` 已失败或三处结果不符合预期，停止修复并另立归因记录。

退出：引入点有直接复跑证据，不再只靠推理链。

### FIX-1：收窄路由

1. 定义只针对 Float pow 的 `numeric_range` 豁免条件，输入来自 FIX-0 的检查分类；
2. 整数/固定宽度 `numeric_range` 程序恢复动态路径与 `weak`；Float pow 保持静态 `cmath_pow`；
3. 不改变 `uses_runtime`、组件集合与缓存指纹的其他既有行为；若必须改指纹，先停下另裁。

退出：两类程序路由明确且互不串线。

### FIX-2：定向回归与 B4-3 矩阵

1. 新增两条路由断言用例（整数 numeric_range-only、Float pow 静态共核）；
2. `n0_b_dynamic_native` 全部通过；22c 29/29、边界 4 项、B4-4 双路径/归档回归通过；
3. B4-3 四组 18,318 项与 1,075 组 pow 对账重跑或按固定快照复核，不得以旧日志冒充本次；
4. Clippy/fmt 通过。

退出：路由回归与既有矩阵同时通过。

### FIX-3：完整 workspace 与 B4-5 交接

1. 运行完整 workspace 门禁，保存实际 cargo 输出与失败/重试记录；
2. 交回 B4-5：B4-5 重新执行其 blocked 收口，通过后才可进入 B4-5C/D；
3. 保存源码、Runtime、工具链、目标、PID、日志与前后摘要；Linux/macOS/CI/性能未执行时如实登记。

退出：workspace 门槛恢复；B4-5 重新开始收口；B5 未开始。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 归因 | 三个固定提交的单项直接复跑、日志哈希、源码摘要 |
| 路由 | 整数 numeric_range-only 动态+weak；Float pow 静态+cmath_pow+无 llvm.pow.f64 |
| n0_b | 全部用例通过，原断言未改 |
| B4-3 矩阵 | 四组 18,318 项、1,075 组 pow |
| B4-4/22c | 双路径/归档/边界、22c 29/29 |
| workspace | 实际 cargo 输出、完整 workspace 门禁 |
| 边界 | 未改 ABI/版本/registry/加载器/721 |

## 五、退出条件

1. 归因有直接复跑证据；
2. Float pow 豁免收窄完成，整数 numeric_range 路由与 `weak` 恢复；
3. n0_b 全绿且断言未改；定向路由用例、B4-3 矩阵、22c/边界/双路径回归通过；
4. 完整 workspace 门禁通过；
5. B4-5 已交回并重新开始收口；B5 未开始；
6. 无 ABI/版本/registry/加载器改动，无旧721；
7. 三平台/性能仍未声称。

## 六、风险与禁止

- 继续保留全量 `numeric_range` 豁免，换一种方式绕开 n0_b；
- 改 n0_b 断言或 `#[ignore]` 让它变绿；
- 收窄时误伤 Float pow 的静态共核，重新引入 `llvm.pow.f64`；
- 顺手改 cmath 数学/加载器/归档或升版本；
- 用 B4-3/B4-4 旧日志冒充 FIX.1 本次执行；
- 在 B4-5 收口批里修产品。

## 七、交接

FIX.1 退出后由 B4-5 重新执行收口；B4-5 通过后才进入 B5（ABI 1.19、生成器沿用21）。B6、B7、B2-C 仍后置。

新 Agent 先读：

1. [B4-5 阻塞证据](assets/20o-b2-b/entry-20261115-b4-5/README.md)；
2. [B4-3 规划与证据](20o-b2-b4-3-llvm-plan.md)；
3. [B4-4 证据](assets/20o-b2-b/entry-20261115-b4-4/README.md)；
4. [B4-5 收口规划](20o-b2-b4-5-closeout-plan.md)；
5. [官方库总纲](20o-official-library-program.md)。

## 八、落点

```text
core/rust/crates/xiao-codegen-llvm/src/dynamic/predicate.rs   路由豁免收窄
core/rust/crates/xiao-driver/tests/                            定向路由回归或既有测试补充
docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-3-fix1/         FIX.1 证据
```

本页只做 FIX.1；B4-5 重跑、B5 与后续批次仍后置。

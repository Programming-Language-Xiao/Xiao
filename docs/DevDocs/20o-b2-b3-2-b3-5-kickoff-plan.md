# 20O-B2-B3-2/B3-5. cmath Runtime、C ABI、intrinsic 34～48 与包装实施规划

> **截至本规划批次时点，B3-2A 与 B3-3A/B 已完成 Windows 本机内部验收；B3-4源码包装现已独立收口，B3-5 与 B4～B7、B2-C 仍未完成。**
> 当前产品代码 `89d782e1`，活动 intrinsic 最大 48、ABI 1.18、生成器 20；实际证据见[实施记录](20o-b2-b3-3a-b-progress.md)。
> 本页只做 15 项 cmath 的 Runtime/C ABI、intrinsic 34～48 与 `stdlib/cmath.xiao` 包装；B4 才接语言全链路。
> `stdlib.cmath` 仍 planned；不升 ABI 1.19/21，不跑旧 721，不关闭历史债务。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B3-1 | 15 项私有内核实测 17,243 条参考，15 函数，max 1 ULP，0 不可验证 |
| 当前内部面 | intrinsic 1～48 活动、ID 100 墓碑；15 个cmath C符号已实现，包装未加入 |
| 本批目标 | 15 个 C 符号、intrinsic 34～48、共享 `pow` 接线、`stdlib/cmath.xiao` 15 包装 |
| 版本策略 | 本批内部激活 34～48，ABI/生成器仍 1.18/20；B5 才切 1.19/21 |
| 未做 | B4 类型/IR/TAC/VM/LLVM、B5 版本、B6 性能、B7 收口、B2-C 721/发布 |

B3-2/B3-5 是 `cmath` 的 Runtime/C ABI/声明/包装批，不是 B4 语言全链路；B4 才让 `std.cmath` 在 VM/LLVM 和归档中真正可调用。

### 1.1 要做

1. 先把 float `**` 切到与 `cmath.pow` 共用的内核，逐组处理 1,075 组差异并回归 B2-A；
2. 为 15 项 cmath 增加 C ABI 符号，按 F-5 的借用/out/状态表实现；
3. 在 `intrinsics.json` 增加 34～48，保持 1～33 与 ID 100 不变；
4. 新增 `stdlib/cmath.xiao` 的 15 个公开包装，只调用已确认原语；
5. 覆盖 15 项域/特殊值/NaN/Inf/有符号零/下溢/ULP 与 C ABI 状态；
6. 保存符号表、声明表、Runtime 摘要、测试计数和 B1/B2 回归证据；
7. 复核没有 registry、stdlib 教程、ABI/生成器或工作流提前升级；
8. 形成 B4 入口：类型/IR/TAC/VM/LLVM/序列化双路径。

### 1.2 明确不做

- 不实现 sfloat 版 cmath、默认参数、重载、方法别名或新自动转换；
- 不新增 34～48 之外的 intrinsic/C 符号，不改 1～33/ID 100；
- 不升 ABI 1.19/生成器 21，不改 `.xiaoc`/`.xar`/原生元数据版本；
- 不升级 registry/教程，不把 `stdlib.cmath` 写成 verified；
- 不运行旧 721 条 F10，不关闭 22C/22D、19/11/22；
- 不把 B3 Runtime/C ABI 证据当成 B4 语言全链路或 B2-C 发布。

## 二、开工闸门

B3-2 执行前必须确认：

1. B3-0/B3-1 审计通过，`88886dd5` 的两个允许路径未漂移；
2. 17,243 条参考、15 函数、max 1 ULP、0 不可验证仍可复现；
3. intrinsic 1～33 活动、34～48 不存在、ID 100 墓碑；
4. ABI 1.18/生成器 20/标准库接口 1/cmath planned 未变；
5. 1,075 组 pow 差异表（1,011 一致、40 值/错误边界、24 位模式）可读；
6. B1/B2 的 Runtime/C ABI/语言回归和 28/28 守卫可复现；
7. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、分批执行

### B3-2A：共享 `pow` 切换与 B2-A 回归

执行细化见[float `**` 与 `cmath.pow` 共享内核切换规划](20o-b2-b3-2a-pow-switch-plan.md)；B3-2A只切 float `**`。

1. 对 1,075 组 `pow` 差异逐组标注旧 float `**`、私有 cmath.pow、F-2/F-9 规则和公开路径可达性；
2. 1,011 组一致不改变行为；40 组值/错误边界与 24 组位模式必须逐项决定；
3. 把 `ops.rs` 的 float `**` 从宿主 `powf` 切到共享内核；不得只切 cmath 一侧；
4. 共享内核继续先拒绝 NaN/Inf，按 F-2 处理 0**0、负底整指数、-0 奇偶、0 负幂；
5. 运行 B2-A 算术/语言/重放回归；已验收行为若改变且无 F-2/F-9 依据，停止回冻结；
6. 保存切换前后同输入位模式、错误身份和 B2-A 测试日志；
7. 本批不修改 `cmath.pow` 的公开参数名/符号。

退出：float `**` 与 `cmath.pow` 共用同一内核；差异表全部有归属，B2-A 回归通过或无未登记变化。

### B3-3A：15 个 C ABI 符号

执行细化见[C ABI 与 intrinsic 34～48 规划](20o-b2-b3-3a-b-c-abi-intrinsics-plan.md)；B3-3A只做C入口，声明在B3-3B。

1. 新增 sqrt/floor/ceil/round/trunc/abs/min/max/pow/sin/cos/tan/exp/ln/log10 的 C 符号；
2. 符号名与 F-5 表一致：`xiao_runtime_cmath_*`；
3. 一元函数一个借用输入、二元 min/max/pow 两个借用输入，统一拥有 out；
4. 状态 0 成功释放替换 out；1 空指针、2 未知标签、3 坏句柄保持 out 且不碰挂起槽；
5. 状态 6 已知类型/域/非有限/预算/分配失败保留 out 并设置可恢复错误；8 沿 Fatal；
6. 输入强引用覆盖调用，输入与 out 不重叠；成功不清既有挂起错误；
7. 不新增宿主能力，不把 Pure 解释成可删除。

退出：15 个 C 符号与 Runtime 内核一一对应；状态/out 测试通过。

### B3-3B：intrinsic 34～48 声明

执行细化见[C ABI 与 intrinsic 34～48 规划](20o-b2-b3-3a-b-c-abi-intrinsics-plan.md)；B3-3B只做声明，类型/IR/VM/LLVM留B4。

1. 在 `intrinsics.json` 增加 34～48，保持 1～33 与 ID 100；
2. 每项声明：源级入口、参数名、`float` 返回、`Pure`、无能力、C 符号；
3. 一元/二元参数数量与 F-5 表一致；
4. 15 个声明与 C 符号、Runtime 入口一一对应，不留缺号/重复；
5. 本批只加声明和 Runtime/C ABI，不接类型/IR/VM/LLVM（B4）；
6. 保存声明表、34～48 集合、33 与 100 不变证据。

退出：34～48 在内部快照活动；活动最大 48；ABI 仍 1.18/20；B4 未开始。

### B3-4：`stdlib/cmath.xiao` 包装

执行细化见[stdlib/cmath.xiao 包装规划](20o-b2-b3-4-stdlib-wrapper-plan.md)与[收口规划](20o-b2-b3-4-closeout-plan.md)；B3-4只写包装源码，语言双路径留B4。

1. 新增 15 个公开包装，只调用已确认原语；
2. 不复制数学实现，不假装已有私有可见性；
3. 参数/返回均为 `float`，无默认值、重载、别名；
4. 不新增宿主能力；Pure 仍可能报错；
5. registry 保持 planned，不把包装写成 verified；
6. 保存源码、导出集合、导入/移除负例和双路径基线，但 VM/LLVM 全链路留 B4。

退出：15 个包装源码存在且只依赖已确认原语；registry/cmath 状态未升级。

### B3-5：证据与 B4 交接

1. 15 项域/特殊值/NaN/Inf/有符号零/下溢/ULP 回归；
2. 15 个 C ABI 空指针/未知标签/坏句柄/类型错/out 保留与替换；
3. intrinsic 34～48 声明、1～33/100 不变、活动最大 48；
4. 共享 pow 差异表、B2-A 回归、float `**`/cmath.pow 一致性；
5. `stdlib/cmath.xiao` 导出/移除/普通包隔离基线；
6. B1/B2 的 20 入口、资源、双精确、28 守卫与 33 单入口回归；
7. 保存源码、锁、Runtime、工具链、目标、PID、日志和前后摘要；
8. 形成 B4 入口：类型/IR/TAC/VM/LLVM/序列化双路径。

退出：B3 Runtime/C ABI/声明/包装内部证据完整；B4 未开始；cmath 仍 planned。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 共享 pow | 1,011 一致、40 值/错误边界、24 位模式逐项；B2-A 回归 |
| 域/特殊值 | NaN/Inf、负零、子正规、下溢、非有限、009 |
| 取整/超越 | 逐位/≤4 ULP，tan 不设 epsilon 极点 |
| C ABI | 15 符号、状态 0/1/2/3/6/8、out 保留/替换 |
| 声明 | 34～48、Pure/None、1～33/100 不变、最大 48 |
| 包装 | 15 个 `stdlib.cmath`、无默认/重载、只调原语 |
| 回归 | B1/B2 20/资源/双精确/28 守卫/33 单入口 |
| 边界 | ABI1.18/20、cmath planned、registry 未改、721 未跑 |

## 五、退出条件

1. float `**` 与 `cmath.pow` 共用内核，1,075 组差异全部有归属，B2-A 回归通过；
2. 15 个 C ABI 符号与 Runtime 内核一一对应，状态/out 符合 F-5；
3. intrinsic 34～48 声明完整，1～33/100 不变，活动最大 48；
4. `stdlib/cmath.xiao` 15 个包装存在且不依赖未确认符号；
5. B1/B2 回归通过；旧 721 未运行；历史债务未核销；
6. ABI 1.18/生成器 20/标准库接口 1 未变，registry/cmath 仍 planned；
7. 三平台/CI 仍未声称；B4 未开始；
8. 向量迁移到 `tests/fixtures/docs/cmath/` 的 B4 前置项仍明确保留。

## 六、风险与禁止

- 只切 `cmath.pow` 不切 float `**`，或反向只切一半；
- 把 40/24 组差异静默解释为容差，不逐项归属 F-2/F-9；
- 用宿主 libm 直接当 C ABI 结果，绕过私有内核；
- 失败时改写 out、丢失挂起错误或把状态 6 混成 1/2/3；
- 提前把 `stdlib.cmath` 升 verified、改 registry 或写最终教程；
- 在 B3 提前升 ABI 1.19/21 或生成跨版本产物；
- 用 B3-1 的参考成绩冒充 B4 语言全链路或 B2-C 发布。

## 七、交接

B3-2/B3-5 完成后进入 B4：类型/IR/TAC/VM/LLVM/序列化双路径，`std.cmath`/`stdlib.cmath` 导出、移除、归档和普通包隔离。B4 前必须把产品测试正式向量迁入 `tests/fixtures/docs/cmath/`。B5 再升 ABI 1.19/21，B6 做资源/性能，B7 收口后交 B2-C。

新 Agent 先读：

1. [B3 cmath 规划](20o-b2-b3-cmath-plan.md) §7.1；
2. [B3-0/B3-1 证据](assets/20o-b2-b/entry-20261108-b3/README.md)；
3. [B3 pow 差表](assets/20o-b2-b/entry-20261108-b3/b1/pow-diff.json)；
4. [F-2/F-9 行为与参考](20o-b2-freeze-draft.md#七cmath行为误差与二进制转换f-2f-8f-9已确认)；
5. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
6. [F-12 错误表](20o-b2-freeze-b3-error-table-plan.md)。

## 八、落点

```text
core/rust/crates/xiao-runtime/src/value/ops.rs                 float ** 切共享 pow
core/rust/crates/xiao-runtime/src/value/arithmetic/cmath.rs    15 项内核
core/rust/crates/xiao-runtime/src/abi_cmath.rs                 15 个 C 入口
core/rust/crates/xiao-runtime-abi/src/lib.rs                   15 个 C 声明
core/rust/crates/xiao-intrinsics/intrinsics.json               intrinsic 34～48
stdlib/cmath.xiao                                              15 个公开包装
docs/DevDocs/assets/20o-b2-b/entry-<date>-b3/                  B3-2/B3-5 证据
```

本页只规划 B3-2/B3-5；B4 语言全链路、B5 版本、B6 性能与 B2-C 仍后置。

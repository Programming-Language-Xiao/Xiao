# 20O-B2-B3. 15 项 `cmath`、intrinsic 34～48 与共享 `pow` 实施规划

> **B3-0/B3-1 已完成并经自审，现有证据仅为 Windows 本机；B3-2 起与 B4～B7、B2-C 仍未开始。**
> 入口为 B2 内部收口代码提交 `37b92c53` 及其证据；当前活动 intrinsic 最大 33，ABI 1.18、生成器 20、标准库接口 1。
> B3 只实现 15 项 `cmath`、intrinsic 34～48、C ABI 与共享 `pow`；不升 ABI 1.19/21，不升级 registry。
> `stdlib.cmath` 仍 planned，旧 721 条 F10 不运行，历史债务保持独立。
> 已完成部分见 [B3-0/B3-1 证据](assets/20o-b2-b/entry-20261108-b3/README.md)；后续接线须满足本页 §7.1 的四项交接门槛。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B2 | 全局 `lfloat_divide`/intrinsic 33 已在 `37b92c53` 内部收口；B2 审计通过 |
| B3-0/B3-1 | 基线与私有内核参考验收完成；代码 `88886dd5`，B3证据冻结于 `96f28de0`，仅 Windows 本机 |
| 公开版本 | ABI 1.18、生成器 20、活动 intrinsic 最大 33、标准库接口 1 |
| 本批目标 | 15 项 `std.cmath`、intrinsic 34～48、15 个 C 符号、`cmath.pow` 与 float `**` 共用内核 |
| 版本策略 | B3 只在内部快照激活 34～48；B5 才统一升 ABI 1.19/21 |
| 文档/registry | `stdlib.cmath` 仍 planned；B2-C 才按最终三平台证据升级 |
| 未做 | B4 语言全链路、B5 版本切换、B6 资源/性能、B7 收口、B2-C 721/发布 |

B3 是 `cmath` 的** Runtime/C ABI/参考向量批**，不是 B2-B 整批完成。B4 才接类型/IR/TAC/VM/LLVM 全链路；B5 才切版本；B2-C 才做最终公共发布。

### 1.1 要做

1. 按 F-9 建立 MPFR 4.2.2/GMP 6.3.0 离线参考向量与 4 ULP/位精确验收；
2. 完成 sqrt/floor/ceil/round/trunc/abs/min/max/pow/sin/cos/tan/exp/ln/log10 的 Runtime 行为；
3. 让 `cmath.pow` 与 float `**` 共用同一数学内核，消除两套域/末位规则；
4. 新增 15 个 C ABI 符号与 intrinsic 34～48 声明，保持 33 与 1～32 不变；
5. 新增 `stdlib/cmath.xiao` 的 15 个公开包装，只调用已确认原语；
6. 覆盖 NaN/Inf 拒绝、数学域 009、有符号零、下溢、非有限结果和 out 所有权；
7. 保存参考生成器、版本、输入/输出位模式、摘要和 Runtime/C ABI 证据；
8. 保持 B1/B2 的 20 入口、资源、双精确、28 守卫与 B2 单入口回归。

### 1.2 明确不做

- 不实现 `sfloat` 版 `cmath`、默认参数、重载、方法别名或新数值自动转换；
- 不新增 F-5 之外的 C 符号/intrinsic，不改变 1～33 或 ID 100 墓碑；
- 不升 ABI 1.19/生成器 21，不改 `.xiaoc`/`.xar`/原生元数据版本；
- 不把 `stdlib.cmath` 提前升 verified，不改 registry 或最终教程；
- 不运行旧 721 条 F10，不关闭 22C/22D、19/11/22；
- 不把 B3 Runtime/C ABI 证据当成 B4 语言全链路或 B2-C 发布。

## 二、开工闸门

B3 执行前必须确认：

1. B0/B1/B2 只读审计在当前仓库通过；
2. B2 代码提交 `37b92c53` 的允许路径与 intrinsic 33 声明未漂移；
3. 当前 intrinsic 1～33 活动、34～48 不活动、ID 100 墓碑未变；
4. ABI 1.18/生成器 20/标准库接口 1/cmath planned 未变；
5. B1/B2 的 Runtime/C ABI/语言回归可复现；
6. F-2/F-5/F-9/F-12 正文与 MPFR/GMP 参考协议一致；
7. 唯一任务根、独立 target/Runtime/缓存已建立，不复用 B2 产物。

## 三、分批执行

### B3-0：入口、快照与公开面基线

执行细化见[B3-0/B3-1 开工规划](20o-b2-b3-0-b3-1-kickoff-plan.md)；B3-0只做基线。

1. 固定 B2 代码 `37b92c53`、入口/证据身份和 ABI 1.18/生成器 20；
2. 复核 intrinsic 1～33 活动、34～48 不活动、ID 100 墓碑；
3. 保存 B1/B2 的 Runtime、C ABI、VM/原生精确入口和 28/28 守卫基线；
4. 记录当前 `xiao_runtime_*` 符号表和 `std.cmath` 不存在；
5. 建独立证据目录，不覆盖 B0/B1/B2。

退出：入口身份和公开面基线可复现；未写 cmath。

### B3-1：F-9 参考向量与私有内核升级

执行细化见[B3-0/B3-1 开工规划](20o-b2-b3-0-b3-1-kickoff-plan.md)；B3-1只做参考和私有内核，不激活34～48。

1. 用离线 MPFR 4.2.2/GMP 6.3.0 生成 binary64 输入/输出位模式；产品不新增 C/GMP 依赖；
2. 输入按 64 位模式精确装入，从 256 位起向下/向上求值，两端最近偶数转 binary64 一致才接受；
3. 不能唯一确定时逐次加倍到 4096 位；仍不能确定则标不可验证，不生成猜测黄金；
4. 参考向量登记生成器来源、版本、输入/输出位模式或稳定错误、摘要；
5. sqrt 0 ULP；floor/ceil/round/trunc/abs/min/max 逐位；普通超越与 pow 非零结果 ≤4 ULP；
6. 把 B1 的常量 ULP 初检替换为正式向量；保留 B1 历史结果，不改写；
7. 复核子正规、带符号零、域错误和 NaN/Inf 拒绝，不用数值容差放过错误符号/错误身份。

退出：15 项函数有独立、可复现的参考向量；没有宿主 libm 自证。

### B3-2：15 项 Runtime 内核

执行细化见[B3-2/B3-5 开工规划](20o-b2-b3-2-b3-5-kickoff-plan.md)与[B3-2A pow切换规划](20o-b2-b3-2a-pow-switch-plan.md)；B3-2A先切共享pow，C ABI与声明在B3-3A/B。

1. 逐项完成 sqrt/floor/ceil/round/trunc/abs/min/max/pow/sin/cos/tan/exp/ln/log10；
2. 全部入口先拒绝 NaN/Inf；域错误和非有限结果按 F-2 为 009；
3. `sqrt(-0)=-0`、`round(-0.5)=-0`、`min/max` 零符号、`ln(1)=+0` 等按 F-2；
4. `exp` 正常下溢允许子正规/有符号零；不把正常下溢当 009；
5. `pow` 处理 `0**0=1`、负底整指数、`-0` 奇偶幂、0 的负幂和负底非整数域错误；
6. 取整/abs/min/max 逐位精确；普通非零超越与 pow 结果 ≤4 ULP；
7. 建立 `cmath.pow` 与 float `**` 的单一共享内核；B3 一次性切换两个调用点，不留半接线；
8. 切换前后产出 float `**` 行为差表，逐项对照已确认 F-2/F-9；涉及尚未裁定的 B2-A 兼容变化时先回冻结。

退出：15 项 Runtime 内核通过正式参考；float `**` 与 `cmath.pow` 共用同一实现。

### B3-3：intrinsic 34～48 与 C ABI

执行细化见[B3-2/B3-5 开工规划](20o-b2-b3-2-b3-5-kickoff-plan.md)；B3-3A做C ABI，B3-3B做声明。

1. 在 `intrinsics.json` 增加 34～48，保持 1～33 和 ID 100 不变；
2. 每项声明：源级入口、参数名、返回 `float`、效果 `Pure`、能力 `None`、C 符号；
3. 15 个 C 符号与 F-5 表一一对应：sqrt/floor/ceil/round/trunc/abs/min/max/pow/sin/cos/tan/exp/ln/log10；
4. C ABI 统一借用输入、拥有 out、返回 i32 状态；
5. 状态 0 成功替换 out；1 空指针、2 未知标签、3 坏句柄不碰挂起槽；6 已知类型/域/非有限/预算/分配失败保留 out 并挂起错误；8 沿 Fatal；
6. 输入强引用覆盖调用，输入与 out 不重叠；成功不清既有挂起错误；
7. 不新增 34～48 之外的符号，不把 `cmath` 暴露成宿主能力。

退出：15 个声明/C 符号/Runtime 入口一一对应；34～48 在内部快照活动，ABI 仍 1.18/20。

### B3-4：`stdlib/cmath.xiao` 公开包装

执行细化见[B3-2/B3-5 开工规划](20o-b2-b3-2-b3-5-kickoff-plan.md)；只做包装源码，VM/LLVM双路径留B4。

1. 新增 `stdlib/cmath.xiao`，导出 15 个公开包装；
2. 每个包装只调用已确认原语，不复制数学实现；
3. 不假装已有私有可见性；底层原语按现有 `os_` 模式全局登记；
4. 包装参数/返回均为 `float`，无默认参数、重载或方法别名；
5. 不新增宿主能力；Pure 描述计算副作用，不排除正常错误传播；
6. registry 仍 planned，不写最终教程/示例为 verified。

退出：15 个公开包装存在且只依赖已确认原语；registry/cmath 状态未升级。

### B3-5：证据、回归与 B4 交接

1. 覆盖 15 项特殊值/域/NaN/Inf/有符号零/子正规/ULP；
2. 覆盖 15 个 C ABI 的空指针、未知标签、坏句柄、类型错、out 保留/替换；
3. 覆盖 intrinsic 34～48 声明自洽、1～33 与 100 不变；
4. 复核 float `**` 与 `cmath.pow` 行为差、共享内核和 B2-A 回归；
5. 执行 B1/B2 的 20 入口、资源、双精确、28 守卫与 33 单入口回归；
6. 保存源码、锁、Runtime、工具链、目标、PID、日志和前后摘要；
7. 形成 B4 入口：类型/IR/TAC/VM/LLVM/序列化全链路。

退出：B3 Runtime/C ABI/参考证据完整；B4 未开始；cmath 仍 planned。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 参考 | MPFR/GMP 版本、输入/输出位模式、ULP、不可验证标记 |
| 域/特殊值 | NaN/Inf、负零、子正规、下溢、非有限、009 |
| 取整 | floor/ceil/round/trunc/abs/min/max 逐位 |
| 超越 | sin/cos/tan/exp/ln/log10 4 ULP、tan 不设 epsilon 极点 |
| pow | 0**0、负底整指数、-0 奇偶、0 负幂、共享内核 |
| 声明 | intrinsic 34～48、Pure/None、1～33/100 不变 |
| C ABI | 15 符号、状态 0/1/2/3/6/8、out 保留/替换 |
| 包装 | 15 个 `stdlib.cmath` 公开包装、无私有假设 |
| 回归 | B1/B2 20/资源/双精确/28 守卫/33 单入口 |
| 边界 | ABI1.18/20、cmath planned、registry 未改、721 未跑 |

## 五、退出条件

1. 15 项 cmath 的 Runtime、C 符号、intrinsic 34～48 一一对应；
2. F-9 正式参考覆盖特殊值与 ULP，无宿自主证或猜测黄金；
3. NaN/Inf、域、有符号零、下溢、非有限结果符合 F-2；
4. `cmath.pow` 与 float `**` 共用内核，B2-A 回归无未登记变化；
5. 15 个 C ABI 状态/out 所有权符合 F-5；
6. `stdlib/cmath.xiao` 15 个包装存在且无默认/重载；
7. B1/B2 回归通过；旧 721 未运行；历史债务未核销；
8. `stdlib.cmath` 仍 planned，registry 未改；B4 未开始。

## 六、风险与禁止

- 用宿主 `std`/`libm` 的结果直接当正式 ULP 黄金；
- 用数值容差放过错误符号、错误域或错误错误身份；
- 为 `cmath.pow` 另写一套内核，导致与 float `**` 分叉；
- 在 B3 提前升 ABI 1.19/21 或生成跨版本产物；
- 把 `cmath` 声明成宿主能力，或删除“结果未使用”的 Allocates/Pure 调用；
- 提前升级 registry/教程，或把 B3 写成 B4/B2-C 完成；
- 用 B2 的 33 单入口或 B1 常量初检冒充 F-9 正式成绩。

## 七、交接

B3 完成后进入 B4：类型/IR/TAC/VM/LLVM/序列化全链路，`std.cmath` 与 `stdlib.cmath` 双路径、导出/移除/归档。B5 再升 ABI 1.19/生成器 21，B6 做资源/性能，B7 收口后交 B2-C。

### 7.1 B3-0/B3-1 已知边界与后续门槛

以下四项属于后续批次的执行要求。本次只同步交接文档；现有产品测试路径和历史证据身份保持原样。

| 项 | 当前事实 | 后续执行要求 |
| --- | --- | --- |
| 正式向量归属 | `cmath_tests.rs` 的 `include_str!` 仍直接读取 DevDocs 证据目录 | 在 B4 接入前将产品测试使用的正式向量迁入 `tests/fixtures/docs/cmath/`；后续批次单列路径、源码身份和测试入口。DevDocs 保留历史副本与摘要，校验夹具与副本逐字节一致，并从迁移后的源码重新编译测试，确认产品测试不再依赖 DevDocs 目录位置 |
| 本机 Runtime 身份 | 默认 `audit.py` 校验归档、构建窗口和记录身份；只有 `--live-runtime` 读取并校验两份本机 Runtime 存档。现有复跑报告已含该模式通过记录 | 后续声称本机 Runtime 字节身份已核实时，必须另跑 `--live-runtime` 并保存实际输出；默认归档审计通过不能替代该结论。新批次还须绑定本批源码、Runtime 和独立任务根，历史存档通过不替代新产物身份 |
| 共享 `pow` 行为变化 | [差表](assets/20o-b2-b/entry-20261108-b3/b1/pow-diff.json)有 1,075 组：1,011 组一致、40 组值/错误边界差异、24 组位模式差异 | B3-2/B3-3 切共享内核前逐组登记旧/新行为、对应 F-2/F-9 规则及公开路径可达性；全部差表用例保留。切换后回归 B2-A 已验收的算术、错误、语言和重放路径，VM 与原生 O0/O3 分记；超出已确认规则或出现尚未裁定的 B2-A 兼容变化时先回冻结 |
| 平台和 CI 证据 | 当前归档只有 Windows 本机证据，未收录本批 Linux/macOS 或远端 CI 实测 | B4 语言全链路、B5 版本矩阵、B6 资源/性能分别补齐对应的三平台与远端 CI 证据，记录源码、Runtime、作业及日志身份；缺项明确标不可验证。不能将当前 B3 写成三平台通过，也不能因此提前升级 `cmath` 教程或 registry |

历史 B3-0/B3-1 的默认复跑入口仍按其固定 Git 对象审计。向量迁移与后续共享内核接线使用新的源码身份和证据，不改写旧快照、差表或清单。B2-C 的完整 721 条 F10 与最终状态升级继续按原批次执行。

新 Agent 先读：

1. [B2-B 实施规划](20o-b2-b-implementation-plan.md)；
2. [B2 `lfloat_divide` 实施记录](20o-b2-b2-lfloat-divide-progress.md)；
3. [已签冻结正文](20o-b2-freeze-draft.md) §五～§九；
4. [F-2/F-9 cmath 行为与参考](20o-b2-freeze-draft.md#七cmath行为误差与二进制转换f-2f-8f-9已确认)；
5. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
6. [F-12 错误表](20o-b2-freeze-b3-error-table-plan.md)。

## 八、落点

```text
core/rust/crates/xiao-intrinsics/intrinsics.json             intrinsic 34～48
core/rust/crates/xiao-runtime/src/value/arithmetic/cmath.rs  15 项私有内核
core/rust/crates/xiao-runtime/src/abi_cmath.rs               15 个 C 入口
core/rust/crates/xiao-runtime-abi/src/lib.rs                 15 个 C 声明
core/rust/crates/xiao-runtime/src/value/ops.rs               float ** 切共享 pow
stdlib/cmath.xiao                                            15 个公开包装
tests/fixtures/docs/cmath/                                   cmath 夹具
docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/               B3-0/B3-1 Windows 本机证据
```

本页记录 B3 的分批规划与进度；B3-0/B3-1 已完成，B3-2 起、B4～B7 与 B2-C 仍后置。

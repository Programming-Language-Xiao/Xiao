# 20O-B2-B3-3A/B. cmath C ABI 与 intrinsic 34～48 实施规划

> **截至本规划批次退出，B3-2A 与 B3-3A/B 已完成 Windows 本机内部验收，B3-4、B4～B7 与 B2-C 未开始；后续B3-4源码包装另批登记。**
> 本批入口为 `5343baac`，产品代码 `89d782e1`；Runtime float `**` 与 `cmath.pow` 已共享内核，LLVM 静态路径仍留 B4。
> 本页只做 15 个 cmath C ABI 符号与 intrinsic 34～48 声明；不接类型/IR/TAC/VM/LLVM，不写 `stdlib/cmath.xiao`。
> `stdlib.cmath` 仍 planned；ABI 仍 1.18、生成器 20、标准库接口 1。
> 实际执行见[实施记录](20o-b2-b3-3a-b-progress.md)及[本批证据](assets/20o-b2-b/entry-20261109-b3-cabi/README.md)；以下保留开工与退出要求。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B3-1 | 15 项私有内核通过 17,243 条参考，max 1 ULP |
| B3-2A | Runtime float `**` 已切共享内核，1,075 组逐项登记，B2-A 与 B1 回归通过 |
| 当前内部面 | intrinsic 1～48 活动、ID 100 墓碑；15 个 `xiao_runtime_cmath_*`，无 `stdlib/cmath.xiao` |
| 本批目标 | 15 个 C 符号、intrinsic 34～48 声明、声明/C 符号/Runtime 一一对应 |
| 未做 | B3-4 `stdlib/cmath.xiao`、B4 语言全链路、B5 版本、B6 性能、B2-C 发布 |

B3-3A/B 是 **C ABI + 声明批**，不是语言接线。B4 才让 `std.cmath` 在类型/IR/VM/LLVM/归档中可调用。

34～48 的 active 仅表示内部声明已登记，当前 VM/LLVM 尚未绑定这些 ID，也没有 `stdlib/cmath.xiao` 包装。B4 接线与相应验收完成前，不得发布或让公开产物消费这些 ID；当前 1.18/20 开发快照不构成可发布 intrinsic 支持。

### 1.1 要做

1. 为 15 项 cmath 增加 C ABI 符号，按 F-5 的借用/out/状态表实现；
2. 在 `intrinsics.json` 增加 34～48，保持 1～33 与 ID 100 不变；
3. 每项声明：源级入口、参数名、`float` 返回、`Pure`、无能力、C 符号；
4. 一元函数一个借用输入，min/max/pow 两个借用输入，统一拥有 out；
5. 状态 0 成功替换 out；1/2/3 结构错误保持 out 且不碰挂起槽；6 语义错误保留 out 并设置可恢复错误；8 沿 Fatal；
6. 覆盖 15 个符号的空指针、未知标签、坏句柄、类型错、域/非有限和 out 保留/替换；
7. 保存符号表、声明表、Runtime 摘要、测试计数和 B1/B2/B3 回归；
8. 形成 B3-4 包装入口。

### 1.2 明确不做

- 不新增 34～48 之外的 intrinsic/C 符号，不改 1～33/ID 100；
- 不实现 `stdlib/cmath.xiao` 包装（B3-4）；
- 不接类型/IR/TAC/VM/LLVM/序列化（B4）；
- 不升 ABI 1.19/生成器 21，不改 `.xiaoc`/`.xar`/原生元数据；
- 不实现 sfloat cmath，不新增宿主能力；
- 不升级 registry/教程，不跑旧 721，不关闭历史债务。

## 二、开工闸门

1. B3-0/B3-1 审计通过，`88886dd5` 两个允许路径未漂移；
2. B3-2A 完成，float `**`/cmath.pow 共享内核和 B2-A 回归有证据；
3. intrinsic 1～33 活动、34～48 不存在、ID 100 墓碑；
4. ABI 1.18/生成器 20/标准库接口 1/cmath planned 未变；
5. 15 项私有内核、参考、C 符号基线可复现；
6. 无 `xiao_runtime_cmath_*`、无 `stdlib/cmath.xiao`；
7. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、B3-3A：15 个 C ABI 符号

### 3.1 符号与签名

按 F-5 表，15 个符号：

```text
xiao_runtime_cmath_sqrt    (value, out)
xiao_runtime_cmath_floor   (value, out)
xiao_runtime_cmath_ceil    (value, out)
xiao_runtime_cmath_round   (value, out)
xiao_runtime_cmath_trunc   (value, out)
xiao_runtime_cmath_abs     (value, out)
xiao_runtime_cmath_min     (left, right, out)
xiao_runtime_cmath_max     (left, right, out)
xiao_runtime_cmath_pow     (base, exponent, out)
xiao_runtime_cmath_sin     (value, out)
xiao_runtime_cmath_cos     (value, out)
xiao_runtime_cmath_tan     (value, out)
xiao_runtime_cmath_exp     (value, out)
xiao_runtime_cmath_ln      (value, out)
xiao_runtime_cmath_log10   (value, out)
```

全部返回 `i32` 状态，输入 `*const XiaoValue` 借用，out `*mut XiaoValue` 已初始化、拥有且不与输入重叠。

### 3.2 状态与 out

| 状态 | 触发 | out/挂起槽 |
| --- | --- | --- |
| 0 | 成功 | 释放并替换旧 out；不清既有挂起错误 |
| 1 | 必需输入/out 为空 | 保持 out，不碰挂起槽 |
| 2 | 未知 XiaoValue 标签/ABI 结构非法 | 保持 out，不碰挂起槽 |
| 3 | 缺少有效句柄、存活盒子种类错或无有效载荷 | 保持 out，不碰挂起槽；不承诺安全识别已释放的悬空地址 |
| 6 | 已知有效标签但类型错，或域/非有限/预算/受控分配失败 | 保持 out，设置对应可恢复错误 |
| 8/Fatal | 不可恢复故障 | 沿既有 Fatal 边界 |

1. 全部入口先拒绝 NaN/Inf；域错误/非有限结果为 009；
2. 类型错为 002；预算/分配失败按既有 011/033；
3. 输入强引用覆盖调用；成功不清既有挂起错误；
4. 结构错误不读取/改写挂起槽；失败不返回部分结果；
5. 不把状态 6 混成 1/2/3，也不把 Fatal 伪装成状态 6。

### 3.3 Runtime 适配

1. C 入口调用 B3-1 的私有内核，不复制数学实现；
2. 15 个入口与私有函数一一对应，二元函数只接受两个 `Float`；
3. 不新增数值提升/窄化；参数标签按降低后的目标检查；
4. 不把 `Pure` 解释成 LLVM readnone 或无错承诺；
5. 不新增宿主能力；所有错误沿既有可恢复通道；
6. 保存 C 符号表、Runtime 摘要、状态矩阵和 15 项测试日志。

退出：15 个 C 符号存在且与私有内核一一对应；状态/out 测试通过。

## 四、B3-3B：intrinsic 34～48 声明

1. 在 `intrinsics.json` 增加 34～48，保持 1～33 与 ID 100；
2. 每项声明字段：
   - 源级名：F-5 表 `std.cmath.*` 对应原语名（`cmath_sqrt` 等）；
   - 参数：一元 `float value`；min/max/pow 两个 `float`；
   - 返回：`float`；
   - 效果：`Pure`；
   - 能力：`None`；
   - C 符号：15 个 `xiao_runtime_cmath_*`；
3. 15 个声明 ID 连续 34～48，无缺号/重复；
4. 一元/二元参数数量与 F-5 表一致；
5. 声明表校验和生成器同步；不改 1～33 的声明/状态；
6. 本批只加声明和 C ABI，不接类型/IR/VM/LLVM；
7. 保存声明表、34～48 集合、1～33/100 不变证据。

退出：34～48 在内部快照活动，活动最大 48；ABI 仍 1.18/20；B4 未开始。

## 五、B3-3C：测试与证据

1. 15 个 C ABI 成功路径：sqrt/floor/ceil/round/trunc/abs/min/max/pow/sin/cos/tan/exp/ln/log10；
2. 15 个 C ABI 结构错误：空指针、未知标签、坏句柄；
3. 类型错误：已知标签但非 float；二元参数缺失/类型错；
4. 语义错误：域、NaN/Inf、非有限结果、预算/分配失败；
5. out：成功替换旧值、失败保留旧值、成功不清挂起错误；
6. 声明：ID 34～48 连续、Pure/None、C 符号一致、1～33/100 不变；
7. 回归：B3-1 参考、B3-2A 共享 pow、B2 单入口、B1 20/17/双精确/28 守卫；
8. 复核无类型/IR/TAC/VM/LLVM/stdlib/registry/ABI 改动；
9. 保存源码、锁、Runtime、工具链、目标、PID、日志和前后摘要。

退出（本批时点）：15 个 C ABI 与 34～48 声明有完整证据；B3-4 当时未开始；cmath 仍 planned。

## 六、证据矩阵

| 层 | 必测 |
| --- | --- |
| 符号 | 15 个 `xiao_runtime_cmath_*`，无多余符号 |
| 声明 | ID 34～48、Pure/None、C 符号一致、1～33/100 不变 |
| 状态 | 0/1/2/3/6/8，out 保留/替换，挂起槽 |
| 域 | NaN/Inf、009、非有限、子正规/有符号零 |
| 类型 | 002，二元参数数量/标签 |
| 预算 | 011/033 与 009 分开 |
| 回归 | B1 20/17/双精确/28、B2 单入口、B3-1 参考、B3-2A pow |
| 边界 | 无 B3-4/B4/版本/registry/721 |

## 七、退出条件

1. 15 个 C 符号与 15 项私有内核一一对应；
2. 状态 0/1/2/3/6/8 与 out/挂起槽符合 F-5；
3. intrinsic 34～48 声明完整，1～33/100 不变，活动最大 48；
4. 没有类型/IR/TAC/VM/LLVM/stdlib/registry/ABI 改动；
5. B1/B2/B3 回归通过；旧 721 未运行；历史债务未核销；
6. `stdlib.cmath` 仍 planned；ABI 1.18/生成器 20/标准库接口 1 未变；
7. B3-4 在本批退出时未开始；当前源码包装与回归证据另见后续批次；B4 未开始。

## 八、风险与禁止

- 15 个 C 符号各自实现数学，绕过私有内核；
- 状态 6 与 1/2/3 混用，或失败时改写 out/丢失挂起错误；
- 把 Pure 声明成可删除或 readnone；
- 在 B3-3 提前接类型/IR/VM/LLVM 或写 `stdlib/cmath.xiao`；
- 提前升 ABI 1.19/21、改 registry 或 cmath 教程；
- 用 B3-1 参考成绩冒充 C ABI/语言全链路；
- 为通过状态测试删错误或改 out 契约。

## 九、交接

B3-3A/B 完成后进入 B3-4：`stdlib/cmath.xiao` 15 个包装。B4 再接类型/IR/TAC/VM/LLVM/序列化全链路。

新 Agent 先读：

1. [B3-2/B3-5 开工规划](20o-b2-b3-2-b3-5-kickoff-plan.md)；
2. [B3-2A pow 切换规划](20o-b2-b3-2a-pow-switch-plan.md)；
3. [B3 cmath 规划](20o-b2-b3-cmath-plan.md) §7.1；
4. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
5. [F-12 错误表](20o-b2-freeze-b3-error-table-plan.md)。

## 十、落点

```text
core/rust/crates/xiao-runtime/src/abi_cmath.rs                 15 个 C 入口
core/rust/crates/xiao-runtime-abi/src/lib.rs                   15 个 C 声明
core/rust/crates/xiao-intrinsics/intrinsics.json               intrinsic 34～48
core/rust/crates/xiao-runtime/src/value/arithmetic/cmath.rs    15 项私有内核
docs/DevDocs/assets/20o-b2-b/entry-<date>-b3/                  B3-3 证据
```

本页记录 B3-3A/B 的规划与内部验收；证据仅为 Windows 本机。B3-4 包装、B4 语言全链路、B5 版本与 B2-C 仍后置。

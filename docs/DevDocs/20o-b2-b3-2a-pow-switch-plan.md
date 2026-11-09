# 20O-B2-B3-2A. float `**` 与 `cmath.pow` 共享内核切换规划

> **B3-0/B3-1 已完成；B3-2A 尚未开工，B3-3 起、B4～B7 与 B2-C 仍未开始。**
> 入口为 B3-1 代码 `88886dd5` 与证据 `96f28de0`；当前 `float **` 仍走宿主 `powf`，私有 `cmath.pow` 尚未接线。
> B3-2A 只把 `float **` 切到共享私有内核；不新增 intrinsic 34～48、不写 C ABI/stdlib、不升 ABI 1.19/21。
> `stdlib.cmath` 仍 planned；旧 721 不运行，历史债务保持独立。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| 私有内核 | `cmath::pow` 已通过 17,243 条 MPFR/GMP 参考，非零结果 ≤4 ULP |
| 旧路径 | `RuntimeValue::power` 的 Float 分支仍是 `finite_float64(left.powf(right))` |
| 差表 | 1,075 组：1,011 一致、40 值/错误边界、24 组 1 ULP 位模式差异 |
| 本批目标 | 一次性把 float `**` 切到 `cmath::pow`，登记全部差异并回归 B2-A |
| 未做 | C ABI/stdlib/intrinsic 34～48、类型/IR/VM/LLVM 全链路、版本切换、性能 |

B3-2A 是共享 `pow` 的行为切换批，不是 B3-3 C ABI 批，也不是 B4 语言接线。切换后 `cmath.pow` 与 float `**` 必须只有一套域/末位规则。

### 1.1 要做

1. 为 1,075 组差异建立逐项清单：输入位模式、旧值、新值、参考、差异类别、F-2/F-9 归属、公开路径可达性；
2. 确认 40 组值/错误边界是否符合 F-2 的“cmath 全部入口先拒绝 NaN/Inf”；
3. 确认 24 组 1 ULP 位模式差异是否都在 F-2/F-9 的 4 ULP 内，且参考值与特殊值规则已登记；
4. 把 `ops.rs` 的 Float 分支改为调用共享 `cmath::pow`，移除旧的宿主 `powf` 路径；
5. 保持 `sfloat` 的 `**` 路径不变，不把 float 切换扩大成 sfloat；
6. 运行 B2-A 算术/语言/重放和 B3-1 参考回归；
7. 保存切换前后同输入位模式/错误身份、B2-A 日志和代码身份；
8. 形成 B3-3A C ABI 入口。

### 1.2 明确不做

- 不新增 intrinsic 34～48，不写 C ABI 符号或 `stdlib/cmath.xiao`；
- 不接类型/IR/TAC/VM/LLVM 的 cmath 公开路径（B4）；
- 不升 ABI 1.19/生成器 21，不改 `.xiaoc`/`.xar`/原生元数据；
- 不实现 sfloat cmath，不改 sfloat `**`；
- 不运行旧 721，不关闭历史债务；
- 不把 1,075 组差异静默当作容差或删掉用例。

## 二、开工闸门

1. B3-1 审计通过，`88886dd5` 两个允许路径未漂移；
2. 17,243 条参考、15 函数、max 1 ULP、0 不可验证可复现；
3. `pow-diff` 计数与分类一致：1,011 一致、40 边界、24 个 1 ULP；
4. B2-A 算术/语言/重放和 28/28 守卫可复现；
5. 当前 `ops.rs` Float 分支仍是宿主 `powf`，没有半接线；
6. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、分批执行

### B3-2A-1：差异清单与规则归属

1. 逐行读取 `pow-diff` 1,075 组，生成 `pow-switch-manifest.json`；
2. 每条记录：id、a/b 位模式、旧值/错误、新值/错误、参考、差异类别、规则、公开路径可达性；
3. 40 组值/错误边界逐项确认：重点覆盖 NaN/Inf 输入、`0**±Inf`、非有限结果、负底非整数；
4. 24 组位模式差异逐项确认：均为 1 ULP，且参考/符号/零/域分类正确；
5. 对超出 F-2/F-9 的差异标“待冻结”，不进入切换。

退出：1,075 组全部有归属；没有未分类差异。

### B3-2A-2：切换 float `**`

1. 把 `RuntimeValue::power` 的 `NumericPair::Float` 分支改为调用 `cmath::pow`；
2. 删除该分支旧的 `left.powf(right)` 与重复有限值检查；错误仍走 009；
3. 保持实参求值顺序、source span、错误传播和资源释放不变；
4. `sfloat` 分支保持 `powf`，不改 sfloat 行为；
5. 不修改 `cmath::pow` 的参数/返回/错误规则，只复用它；
6. 不新增公开 intrinsic/C ABI；`cmath` 仍是私有模块；
7. 一次提交完成切换，不保留 `old/new` 双路径或运行时开关。

退出：float `**` 只调用共享内核；源码中不存在第二条 Float 幂实现。

### B3-2A-3：回归与证据

1. 运行 B2-A 算术、语言、精确入口和重放测试；
2. 运行 B3-1 的 17,243 条参考与 pow 差表；
3. 对 40 组边界变化逐项确认 F-2 规则；若 B2-A 测试断言旧行为，只在 F-2 明确要求时更新并保留旧日志；
4. 对 24 组 1 ULP 变化确认 ≤4 ULP，并记录公开路径；
5. VM 与原生 O0/O3 各跑一组精确入口；不把本批写成性能成绩；
6. B1 七库 20 入口、资源 17、双精确、28/28 守卫回归；
7. 保存切换前后位模式表、B2-A 日志、源码/锁/Runtime/工具链身份；
8. 复核没有 intrinsic/ABI/stdlib/registry 改动。

退出：B2-A 回归通过；差异全部有归属；没有未登记行为变化。

### B3-2A-4：收口与 B3-3 交接

1. 复核 float `**` 与 `cmath.pow` 同输入位模式一致；
2. 复核 `sfloat **`、整数 `**`、lfloat `**` 路径未变；
3. 记录 B3-2A 只做共享 pow，C ABI 在 B3-3A；
4. 形成 B3-3A 入口：15 个 C ABI 符号与 15 项 Runtime 内核。

退出：共享 pow 内部收口；B3-3A 未开始；没有公开 intrinsic/C ABI。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 差异清单 | 1,075 组全部归类，40 边界、24 个 1 ULP |
| 规则 | F-2 NaN/Inf/域、F-9 共享内核与 ≤4 ULP |
| 切换 | float `**` 只调 `cmath::pow`；无私自 `powf` |
| 回归 | B2-A 算术/语言/重放；B3-1 参考；B1 20/17/双精确/28 守卫 |
| 后端 | VM 与原生 O0/O3 精确入口 |
| 边界 | 无 intrinsic 34～48/C ABI/stdlib/ABI/registry 改动 |
| 历史 | 旧日志、旧差异、旧失败保留 |

## 五、退出条件

1. float `**` 与 `cmath.pow` 共用同一内核，源码无第二条 Float 幂实现；
2. 1,075 组差异全部有 F-2/F-9 归属，40 组边界与 24 个 1 ULP 逐项登记；
3. B2-A 算术/语言/重放回归通过；B1 20/17/双精确/28 守卫通过；
4. `sfloat **` 未变；没有 intrinsic/C ABI/stdlib/registry 改动；
5. ABI 1.18/生成器 20/标准库接口 1 未变；cmath planned；
6. 旧 721 未跑；历史债务未核销；B3-3A 未开始。

## 六、风险与禁止

- 把 40 组值/错误边界当成容差，不按 F-2 判 NaN/Inf；
- 把 24 个 1 ULP 差异静默吞掉，或改参考值迁就实现；
- 保留 `old/new` 双路径、环境开关或按调用方选择实现；
- 顺手切换 sfloat `**`，扩大 B3-2A 边界；
- 用宿主 `powf` 作为新路径，或让 `cmath.pow` 与 float `**` 各留一套；
- 更新 B2-A 测试却不保留旧失败/旧断言；
- 提前加 intrinsic 34～48/C ABI/stdlib 或升级 registry。

## 七、交接

B3-2A 完成后进入 B3-3A：15 个 cmath C ABI 符号。B3-3B 再接 intrinsic 34～48；B3-4 加 `stdlib/cmath.xiao` 包装；B4 才接类型/IR/TAC/VM/LLVM 全链路。

新 Agent 先读：

1. [B3-2/B3-5 开工规划](20o-b2-b3-2-b3-5-kickoff-plan.md)；
2. [B3-1 证据](assets/20o-b2-b/entry-20261108-b3/README.md)；
3. [pow 差表](assets/20o-b2-b/entry-20261108-b3/b1/pow-diff.json)；
4. [F-2/F-9](20o-b2-freeze-draft.md#七cmath行为误差与二进制转换f-2f-8f-9已确认)；
5. [B2-A 实施记录](20o-b2-a2-a3-arithmetic-progress.md)。

## 八、落点

```text
core/rust/crates/xiao-runtime/src/value/ops.rs                 float ** 切共享 cmath::pow
core/rust/crates/xiao-runtime/src/value/arithmetic/cmath.rs    私有 pow 内核（不改）
core/rust/crates/xiao-runtime/src/value/arithmetic/cmath_tests.rs  共享一致性测试
docs/DevDocs/assets/20o-b2-b/entry-<date>-b3/                  B3-2A 证据
```

本页只规划 B3-2A；B3-3～B3-5、B4～B7 与 B2-C 仍后置。

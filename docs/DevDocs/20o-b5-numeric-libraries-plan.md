# 20O-B5. 数值库、统计与确定性随机实施规划

> **B5 尚未开始；B4 完成或明确授权后才有 B5-0 探测/冻结入口。**
> B5 覆盖 `decimal`、`fractions`、`statistics` 浮点扩展和确定性 `random`；不实现 CSPRNG（B7）。
> B5 复用 B2 的 lint/lfloat 底座，不重开 B2 数值语义；B4 的 json_value 与 B5 数值类型分开。
> 不实现 B6 regex/compression、B7安全、B8网络、B9数据库；不跑旧721。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B2 | B2-C 最终公共收口尚未完成；B5 产品须绑定其最终数值底座 |
| B4 | JSON/TOML 尚未完成；B5 值类型与 B4 共享值的边界由 B5-0 固定 |
| B0 | D-2 decimal、D-3 fractions、D-5 statistics、D-6 random 方向已确认 |
| 本批目标 | decimal/fractions/statistics浮点/确定性random |
| 未做 | CSPRNG、网络、数据库、压缩、正则、B4值模型 |
| 状态 | 四模块registry/教程初始未交付；验证后按范围升级 |

B5 是 **数值库批**，不是 B2 数值运行时重复。lfloat/lint 的解析/算术/转换已由 B2 交付；B5 只在其上增加公开数值库 API，不能改变 B2 语义。

### 1.1 要做

1. 探测 B2 底座和四模块现有/候选 API、平台/依赖/错误/资源；
2. 冻结 `decimal`：与 lfloat 的关系、解析/格式化、精度/舍入/比较/转换边界；
3. 冻结 `fractions`：lint 分子/分母、自动约分、正分母、零分母、与 decimal/float 转换；
4. 冻结 `statistics` 浮点：mean/median/variance/stddev、空集/NaN/精度/溢出；
5. 冻结确定性 `random`：算法/种子/状态/序列/复现/并发边界，不依赖隐式全局；
6. 实现四模块、教程/示例/错误/资源；
7. 接通类型/IR/TAC/VM/LLVM/归档、std/stdlib双路径、导出/移除/隔离；
8. 三平台证据、registry只在验证范围升级。

### 1.2 明确不做

- 不重写/改变 B2 lfloat/lint 语义或回写 B2-C 的 1.19/21 历史产物；若 B5 需要新 intrinsic/值类型/ABI 增量，按 B5-0 另立版本并冻结；
- 不把 decimal/fractions 并入 B4 json_value；
- 不实现 CSPRNG/加密随机（B7）、网络/数据库（B8/B9）；
- 不新增未冻结能力位/错误码/intrinsic；
- 不提前升级 registry/教程，不跑旧721；
- 不把 B5 扩成 B6-B9。

## 二、开工闸门

1. B4 完成或明确授权 B5-0 探测/冻结；产品实施等待 B4 收口；
2. B2-C 最终公共收口完成，lfloat/lint 底座/ABI1.19/21稳定；
3. D-2/D-3/D-5/D-6 方向、20P Day-1 标准、错误/能力/取消可读；
4. 当前标准库加载器/类型/IR/VM/LLVM/归档可复现；
5. B1七库20入口/17资源/双精确/28守卫可复现；
6. 三平台目标/工具链/CI可用；
7. 唯一任务根、独立target/Runtime/缓存已建立；
8. 四模块精确API/错误/资源/算法/教程未冻结项列入B5-0，不得先写产品后补冻结。

## 三、分批执行

### B5-0：探测、冻结与教程草案

1. 探测 decimal/fractions/statistics/random 的候选实现、依赖/平台/错误/资源/性能；
2. 冻结 `decimal`：
   - 与 lfloat 的表示/身份/显示/精度关系；
   - 解析/格式化/比较/相等/哈希；
   - 显式 precision/rounding 的入口或明确复用 B2 `lfloat_divide`；
   - 转换/错误/资源；
3. 冻结 `fractions`：
   - lint 分子/分母、正分母、自动约分、零分母；
   - 比较/算术/显示；
   - 转 decimal 指定 precision/rounding；
   - 转 float 显式允许精度损失；错误/资源；
4. 冻结 `statistics`：
   - mean/median/variance/stddev（样本/总体口径）；
   - 空集/NaN/Inf/精度/溢出/排序预算；
   - 输入规模/输出类型/错误；
5. 冻结 `random`：
   - 算法与版本、种子范围/0、状态表示/复制；
   - 序列复现/并发/不隐式全局；
   - 不提供CSPRNG；
6. 冻结四模块签名/参数/返回/效果/所有权/能力/错误码/资源/性能预算；
7. 冻结intrinsic/ABI/生成器/.xiaoc/.xar身份增量（如需）；B5 不修改/回写 B2-C 的 1.19/21 历史快照，新增量另立版本；
8. 四份教程草案、固定示例、registry候选（planned）；
9. 记录延期API/平台。

退出：四模块契约/算法/错误/资源/教程冻结；未写产品。

### B5-1：`decimal`

1. 解析/格式化/比较/相等/哈希；
2. precision/rounding/转换；
3. 错误/资源/原子性；
4. 单元/边界/教程；
5. 与lfloat关系复核。

退出：decimal内部可用；无B2语义变化。

### B5-2：`fractions`

1. lint分子/分母、正分母、约分；
2. 算术/比较/显示；
3. 零分母/溢出/资源；
4. decimal/float转换（显式精度/损失）；
5. 单元/边界/教程。

退出：fractions内部可用；不隐式丢精度。

### B5-3：`statistics`浮点

1. mean/median/variance/stddev；
2. 空集/NaN/Inf/精度/样本口径；
3. 排序/输入规模/资源；
4. 单元/边界/教程。

退出：statistics内部可用；错误/精度明确。

### B5-4：确定性`random`

1. 固定算法/版本/种子/状态；
2. 序列复现/复制/并发边界；
3. 无隐式全局/CSPRNG；
4. 单元/边界/教程。

退出：random内部可用；固定种子序列可复现。

### B5-5：语言、双路径与承载

1. 四模块类型/IR/TAC/VM/LLVM/归档；
2. `std.*`/`stdlib.*`双路径、导出/移除/隔离；
3. 常量/归档/XCON/并发承载；
4. 能力/错误/取消/资源；
5. B1七库20/17/双精确/28守卫；
6. 三平台VM/原生O0/O3。

退出：四模块内部全链路一致；无第二套数值语义。

### B5-6：测试、教程与收口

1. 四模块单元/边界/资源/性能/三平台/教程；
2. 教程示例进入VM/原生精确入口；
3. registry只升验证范围；
4. 失败/不可验证受限交接；
5. B1回归/28守卫；
6. 与B4值模型边界复核；
7. 形成B6交接。

退出：B5内部收口；B6未开始；未升级未验证范围。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| decimal | 解析/格式/精度/舍入/比较/转换/资源 |
| fractions | 约分/正分母/零分母/算术/转换/精度 |
| statistics | mean/median/variance/stddev/空集/NaN/精度 |
| random | 算法/种子/状态/序列/并发/无全局 |
| 管线 | 类型/IR/TAC/VM/LLVM/归档/双路径/移除/隔离 |
| 平台 | Windows/Linux/macOS、O0/O3、CI |
| 边界 | 无CSPRNG/B6-B9、无B2语义改动、无721 |

## 五、退出条件

1. B5-0四模块API/错误/算法/资源/教程冻结；
2. decimal/fractions/statistics/random内部实现通过；
3. 双路径/导出/移除/隔离/三平台；
4. 教程进入CI，registry只升验证范围；
5. B1回归/28守卫通过；
6. 无CSPRNG/B6-B9、无B2语义/未冻结ABI/721；
7. B6交接明确；B5内部收口。

## 六、风险与禁止

- 把decimal直接等同lfloat或改B2语义；
- fractions隐式约分以外丢精度/负分母；
- statistics用float求和导致不可解释误差；
- random隐式全局/不稳定算法/冒充CSPRNG；
- 把json_value与数值类型混合；
- 提前升级registry/教程；
- 跑旧721/关闭历史债务。

## 七、交接

B5完成后进入[B6原生工具规划](20o-b6-native-tools-plan.md)：regex/compression/fmt扩展。B6依赖原生库策略；B7安全/CSPRNG后置。

新 Agent 先读：

1. [B4 JSON/TOML规划](20o-b4-json-toml-plan.md)；
2. [B2数值基础规划](20o-b2-numeric-foundation-plan.md)；
3. [B2-C最终收口](20o-b2-c-final-closeout-plan.md)；
4. [20P Day-1架构](20p-day1-library-architecture.md)；
5. [B1收口](20o-b1-closeout.md)。

## 八、落点

```text
stdlib/decimal.xiao / fractions.xiao / statistics.xiao / random.xiao
docs/UseDocs/library/                                  四模块教程
docs/module-registry.json                              四模块候选/最终状态
core/rust/crates/xiao-*/                               类型/IR/VM/LLVM/数值
docs/DevDocs/assets/20o-b5/entry-<date>/               B5证据
```

本页只规划B5；B6-B9仍后置。

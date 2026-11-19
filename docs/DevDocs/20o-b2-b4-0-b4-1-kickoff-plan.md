# 20O-B2-B4-0/B4-1. 向量迁移与 cmath 类型/IR 开工规划

> 本页阶段状态记录当时的退出时点；后续B4-2B已在 `246bea46` 完成Windows本机内部验收，见[B4-2B记录](assets/20o-b2-b/entry-20261113-b4-2b/README.md)。原退出条件及历史证据保持其时点身份。

> **B3-4 源码包装已收口；B4-0/B4-1 已完成当时的 Windows 本机验证，后续 planned 回归已由 FIX1 修复；B4-2A 已退出，B4-2B 起、B5～B7 与 B2-C 仍未开始。**
> B4-0/B4-1 固定产品提交为 `c7a45e10b76b26915499717563761ed20fda6e6c`；[本机收口证据](assets/20o-b2-b/entry-20261110-b4-0-1/README.md)。这不是三平台或 cmath 可执行性证据。
> 入口为 B3-4 产品代码 `7d8b85e4` 与 66 文件收口证据；当前 34～48 只有内部声明，VM/LLVM 未绑定。
> B4-0 只迁移产品测试正式向量，B4-1 只接类型/IR；不接 TAC/VM/LLVM、不升 ABI 1.19/21、不改 registry。
> `stdlib.cmath` 仍 planned；30 条 cmath 导入路径仍明确拒绝。
> 2026-11-12 加注：FIX1 对本页固定 `c7a45e10` 直接重跑 22c 为 27/29，对 B3-4 `7d8b85e4` 为 29/29，确认本页类型接入让原有“包装编译失败”拒绝失效。当时的前端/向量收口不包含这两项通过结论；`c4610211` 增加驱动器 planned 门控，22c 已恢复 29/29，见[归属记录](assets/20o-b2-b/entry-20261112-b4-2a-fix1/README.md)。历史执行和收口文件保持原样。

## 一、入口与目标

| 项 | 当前状态 |
| --- | --- |
| B3-4 | 源码包装收口；15 forwarder、30 条 planned 拒绝、802/可达性修复通过 |
| 代码 | `7d8b85e4`；B3-4 closeout `language_available=false` |
| B4-0/B4-1 | Windows 本机验证收口；向量迁移、类型/IR 绑定完成，完整 B4 仍未收口 |
| 当前版本 | ABI 1.18、生成器 20、接口 1、intrinsic 1～48（34～48 内部） |
| B4-0 目标 | 把 `cmath_tests.rs`/`abi_cmath_tests.rs` 的正式向量迁入 `tests/fixtures/docs/cmath/` |
| B4-1 目标 | 类型检查/IR 接受 34～48 的一元/二元 `float` 原语 |
| 不在本页 | TAC/VM/LLVM、静态 pow 共核、双路径/归档、版本、性能、721 |

B4-0/B4-1 是 B4 的开工前置，不是语言可用性完成；B4-2 才接 TAC/VM，B4-3 才接 LLVM。

### 1.1 要做

1. 迁移两处 `include_str!` 使用的正式向量到 `tests/fixtures/docs/cmath/`；
2. 校验迁移副本与 DevDocs 历史副本逐字节一致，记录 SHA-256；
3. 更新测试引用，重新编译/运行 B3-3 C ABI 与 B3-1 内核回归；
4. 类型检查识别 34～48 的 `float -> float` 一元/二元调用；
5. 支持位置/命名，四参规则不适用于 cmath；无默认值/重载/别名；
6. IR 保留 ID、参数顺序、返回类型和 source span；
7. 静态非法类型走既有诊断，不提前折叠可 catch 的 009；
8. 保存迁移、类型/IR、回归和未接线边界证据。

### 1.2 明确不做

- 不接 TAC/VM/LLVM/序列化/归档；
- 不把 30 条 planned 导入改成可执行；
- 不新增 intrinsic/C 符号，不改 1～48/ID 100；
- 不升 ABI 1.19/21，不改 registry/教程/锁/工作流；
- 不实现 sfloat cmath 或新数值转换；
- 不跑旧 721，不关闭历史债务。

## 二、开工闸门

1. B3-4 收口审计通过，代码 `7d8b85e4` 未漂移；
2. wrapper 8/0、22c 29/0、reachability 30、B3-CABI 22 可复现；
3. 两处 `include_str!` 仍指向 `entry-20261108-b3/b1/reference-vectors.tsv`；
4. intrinsic 34～48 声明、15 C 符号、共享 pow 证据可复现；
5. `stdlib.cmath`/UseDocs/registry 仍 planned；
6. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、B4-0：正式向量迁移

### 3.1 迁移物与身份

1. 源文件：`docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/b1/reference-vectors.tsv`；
2. 目标：`tests/fixtures/docs/cmath/reference-vectors.tsv`；
3. 记录原文件/新文件字节数、SHA-256 和逐字节比较结果；
4. DevDocs 历史副本保留，不删除、不改写；
5. 如果 C ABI 测试还需要 `reference-inputs.json`/`reference-manifest.json`，只迁移测试实际读取的最小集合；
6. 不把参考生成器或 MPFR/GMP 工具带进产品依赖。

### 3.2 测试引用迁移

1. 更新 `cmath_tests.rs` 的 `include_str!` 指向新 fixture；
2. 更新 `abi_cmath_tests.rs` 的 `include_str!` 指向新 fixture；
3. 确认两处不再出现 `entry-20261108-b3` 路径；
4. 重新编译 `xiao-runtime`/`xiao-driver` 测试；
5. 运行 B3-1 内核参考、B3-3 C ABI 17,243 条参考；
6. 保存迁移前后测试计数、源码身份和 fixture 摘要。

退出：产品测试不再依赖 DevDocs 路径；迁移副本逐字节一致；两条参考回归通过。

## 四、B4-1：类型检查与 IR

### 4.1 类型检查

1. 识别 `IntrinsicKind::Cmath` 的 34～48；
2. 一元：一个 `float value`；二元：`float left/right` 或 `base/exponent`；
3. 参数全部必填；支持既有位置/命名调用；
4. 不增加默认值、重载、别名、可变参数；
5. 不新增隐式数值提升/窄化；静态非法类型走既有类型诊断；
6. 不把 `Pure` 当“无错”或允许删除；
7. 30 条 planned 导入路径仍按 B3-4 边界拒绝，B4-1 不改变导入可用性。

### 4.2 IR

1. 34～48 的调用保留精确 ID、参数顺序、返回 `float` 和 source span；
2. 一元/二元形状与 F-5 表一致；
3. 不提前改 TAC/字节码容器版本；
4. 不生成 C ABI/Runtime 调用；VM/LLVM 留 B4-2/B4-3；
5. 保存 IR 形状、类型错误、span 和 34～48 映射。

### 4.3 B4-1 测试

1. 15 项一元/二元合法调用；
2. 缺参、多参、错类型、默认值、重载/别名负例；
3. `std.cmath`/`stdlib.cmath` 两条 planned 路径仍拒绝；
4. B3-4 wrapper 8/0、22c 29/0、reachability 30 回归；
5. B1/B2/B3 历史审计/固定证据不回归；
6. 记录 Windows 本机与未执行平台。

退出：类型/IR 可表达 15 项 cmath；VM/LLVM 仍未绑定；30 条 planned 路径仍拒绝。

## 五、证据矩阵

| 层 | B4-0 | B4-1 |
| --- | --- | --- |
| 向量 | 源/目标字节+SHA、逐字节一致 | 两处 include 不再指向 DevDocs |
| 测试 | 内核参考、C ABI 17,243 | 15 项合法/负例、span |
| 类型 | 不改 | 34～48、float、位置/命名、无默认 |
| IR | 不改 | ID/顺序/返回/span、无 TAC 版本变化 |
| 回归 | wrapper/22c/reachability/B3-CABI | 同左 + B1/B2 历史 |
| 边界 | 无 intrinsic/C ABI/版本改动 | 无 VM/LLVM/导入可用性改动 |

## 六、退出条件

1. 两处正式向量迁入 `tests/fixtures/docs/cmath/`，逐字节一致；
2. 产品测试不再引用 DevDocs 路径；内核/C ABI 参考通过；
3. 类型检查接受 34～48 的一元/二元 `float` 原语；
4. IR 保留 ID/参数/返回/span，未改 TAC/字节码版本；
5. 30 条 planned 导入路径仍拒绝；
6. 无 TAC/VM/LLVM/序列化/归档、C ABI/intrinsic/ABI/registry/锁改动；
7. B3-4/B1/B2/B3 回归通过；旧 721 未运行；
8. 本页原退出时 B4-2 未开始；后续 B4-2A/FIX1 退出见[当前 FIX1 记录](assets/20o-b2-b/entry-20261112-b4-2a-fix1/README.md)，`stdlib.cmath` 仍 planned。

## 七、风险与禁止

- 只复制向量不核对字节，或迁移后仍留 DevDocs 路径；
- 在 B4-1 顺手接 TAC/VM/LLVM，把类型/IR 批扩成语言全链路；
- 把 30 条 planned 导入改成可执行来“证明”类型接通；
- 新增隐式转换/默认参数/重载；
- 把 `Pure` 当无错或可删除；
- 提前升 ABI/registry 或跑 721。

## 八、交接

B4-1 完成后进入 B4-2：TAC/字节码与 VM 绑定；B4-3 接 LLVM 静态/动态与共享 pow；B4-4 双路径/归档/序列化；B4-5 收口。

新 Agent 先读：

1. [B4 语言全链路规划](20o-b2-b4-language-integration-plan.md)；
2. [B3-4 收口证据](assets/20o-b2-b/entry-20261110-b3-wrapper-final/README.md)；
3. [B3-4 handoff](assets/20o-b2-b/entry-20261110-b3-wrapper-final/handoff-b4.md)；
4. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
5. [F-12 错误表](20o-b2-freeze-b3-error-table-plan.md)。

## 九、落点

```text
tests/fixtures/docs/cmath/reference-vectors.tsv        迁移后的正式向量
core/rust/crates/xiao-runtime/src/value/arithmetic/cmath_tests.rs  include 迁移
core/rust/crates/xiao-runtime/src/abi_cmath_tests.rs     include 迁移
core/rust/crates/xiao-types/src/checker/                cmath 类型检查
core/rust/crates/xiao-ir/                               cmath IR
docs/DevDocs/assets/20o-b2-b/entry-<date>-b4/            B4-0/B4-1 证据
```

本页只做 B4-0/B4-1；B4-2 起、B5～B7 与 B2-C 仍后置。

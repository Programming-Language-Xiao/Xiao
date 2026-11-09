# 20O-B2-B3-4. `stdlib/cmath.xiao` 十五项公开包装规划

> **B3-0/B3-1、B3-2A、B3-3A/B 已完成内部验收；B3-4 源码包装已进入当前工作区，语言接线与独立收口仍留 B4；B5～B7 与 B2-C 仍未开始。**
> 入口为 B3-3 代码 `89d782e1` 与证据；当前 intrinsic 34～48 仅内部 active，VM/LLVM 未绑定。
> B3-4 只增加 `stdlib/cmath.xiao` 的 15 个公开包装；不接类型/IR/TAC/VM/LLVM，不升 ABI 1.19/21。
> `stdlib.cmath` registry 仍 planned；不把包装或教程写成 verified，不跑旧 721。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B3-3 | 15 个 C ABI、intrinsic 34～48 内部 active；源码包装已另批加入 |
| 当前登记表 | intrinsic 1～48 active（34～48 仅内部声明）、ID 100 墓碑；ABI 1.18/生成器 20/接口 1 |
| 本批目标 | `stdlib/cmath.xiao` 15 个公开包装、`stdlib/cmath.md` 模块说明、导出集合与源码边界证据 |
| 未做 | 类型/IR/TAC/VM/LLVM、`std.cmath` 可执行导入、B5 版本、B6 性能、B2-C 发布 |
| 状态 | `stdlib.cmath` 仍 planned；UseDocs 教程仍 planned；registry 不升级；语言接线留 B4 |

B3-4 是 **源码包装批**，不是语言可用性批。B4 才把 `std.cmath` 接进类型/IR/VM/LLVM 和归档；B4 前包装不能在公开产物中运行。

[历史基线目录](assets/20o-b2-b/entry-20261109-b3-wrapper/README.md)保留包装进入主工作区前的失败/修补记录；当前源码契约、hex 原生隔离、子 VM 别名与归档边界见[802 修复证据](assets/20o-b2-b/entry-20261110-b3-fix-802/README.md)。这些记录仍不能作为 B3-4 收口或 `std.cmath` 可用证据。

### 1.1 要做

1. 新增 `stdlib/cmath.xiao`，导出 15 个公开包装；
2. 每个包装只调用对应原语：`cmath_sqrt`、`cmath_floor`、…、`cmath_log10`；
3. 参数名与 F-5 表一致：一元 `value`；二元 `left/right` 或 `base/exponent`；
4. 参数和返回均为 `float`，不增加默认值、重载、别名或隐式转换；
5. 不复制数学实现，不在 Xiao 层做域/ULP 判断；
6. 新增 `stdlib/cmath.md` 模块说明，保持 planned/内部状态；
7. 保持 `docs/UseDocs/library/cmath.md` planned，不写可执行导入示例；
8. 保存 15 个导出、原语调用、无重复/缺失和 registry 未升级证据。

### 1.2 明确不做

- 不接类型/IR/TAC/VM/LLVM/序列化（B4）；
- 不把 `std.cmath` 写成已可导入、已 verified 或三平台可用；
- 不新增 intrinsic 49+、C 符号或宿主能力；
- 不升 ABI 1.19/生成器 21，不改 `.xiaoc`/`.xar`/原生元数据；
- 不实现 sfloat cmath；
- 不运行旧 721，不关闭历史债务。

## 二、开工闸门

1. B3-3 审计通过，`89d782e1` 的 13 个允许路径未漂移；
2. intrinsic 34～48 active、1～33/ID 100 不变；
3. 15 个 C 符号与私有内核一一对应；
4. ABI 1.18/生成器 20/标准库接口 1/cmath planned 未变；
5. `stdlib/cmath.xiao` 已存在，但 `std.cmath`/`stdlib.cmath` 仍不可执行导入；
6. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、分批执行

### B3-4A：包装源码与导出集合

1. 新建 `stdlib/cmath.xiao`，只放 15 个公开函数；
2. 一元：

```text
sqrt(value)   floor(value)  ceil(value)   round(value)  trunc(value)
abs(value)    sin(value)    cos(value)    tan(value)
exp(value)    ln(value)     log10(value)
```

3. 二元：

```text
min(left, right)  max(left, right)  pow(base, exponent)
```

4. 每个函数返回 `float`，只调用同名原语：`cmath_sqrt(value)` 等；
5. 不在 Xiao 层复制 NaN/Inf/域/ULP 逻辑；
6. 不增加默认参数、可变参数、重载或别名；
7. 不引入新模块级状态、全局变量或宿主能力；
8. 导出集合必须恰好 15 项，无重复、无缺失。

退出：`stdlib/cmath.xiao` 恰好 15 个导出，全部为 1/2 参 `float -> float`，只调用已确认原语。

### B3-4B：模块说明与状态

1. 新增 `stdlib/cmath.md`，说明 planned/内部状态、15 项范围、参数名和错误传播；
2. 不写“已可导入”“已 verified”或最终示例；
3. 更新 `docs/UseDocs/library/cmath.md` 仅到“包装源码已存在、语言接线留 B4”；
4. `docs/module-registry.json` 的 `stdlib.cmath` 保持 planned；不填 verified code/tests；
5. 不新增模块兼容身份或接口版本；
6. 不把 `stdlib/cmath.xiao` 加入普通包依赖或依赖锁。

退出：源码/说明/教程口径一致；registry 仍 planned；没有语言可用性声明。

### B3-4C：源码级证据与 B4 交接

1. 保存 `stdlib/cmath.xiao` 的 15 个导出、参数数量和原语调用表；
2. 保存 `stdlib/cmath.md`、UseDocs、registry planned 的核对；
3. 复核没有类型/IR/TAC/VM/LLVM/ABI/锁/工作流改动；
4. 复核 intrinsic 34～48 仍 active、15 C 符号存在、无 49+；
5. 复核 `std.cmath` 仍未可执行导入，B4 才接双路径；
6. 保存源码、工具链、目标、PID 和前后摘要；
7. 形成 B4 入口：类型/IR/TAC/VM/LLVM/序列化双路径、导出/移除/归档/普通包隔离。

退出：15 个包装源码与说明、无关 hex 原生隔离、子 VM 与归档 planned 边界有证据；B4 未开始；cmath 仍 planned。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 导出 | 15 项、无重复/缺失、参数名/数量/返回 float |
| 原语 | 每项只调 `cmath_*`，无 Xiao 数学实现/域判断 |
| 说明 | `stdlib/cmath.md`/UseDocs 仍 planned、内部状态 |
| registry | `stdlib.cmath` planned，未升级 code/tests |
| 边界 | 无类型/IR/TAC/VM/LLVM/ABI/锁/工作流改动 |
| 回归 | intrinsic 34～48、15 C 符号、1～33/100 不变 |
| 历史 | 旧 721 未跑、B2-C 未开始、债务未核销 |

## 五、退出条件

1. `stdlib/cmath.xiao` 恰好 15 个 `float -> float` 包装；
2. 每个包装只调用已确认 `cmath_*` 原语，无数学/域逻辑复制；
3. 参数名/数量与 F-5 表一致，无默认值/重载/别名；
4. `stdlib/cmath.md`、UseDocs、registry 均保持 planned/内部状态；
5. 没有类型/IR/TAC/VM/LLVM/ABI/锁/工作流改动；
6. intrinsic 34～48 active、15 C 符号、1～33/100 不变；
7. 旧 721 未运行；历史债务未核销；B4 未开始；
8. B4 入口明确：双路径可执行导入不在本批声称。

## 六、风险与禁止

- 在包装里复制 NaN/Inf/域/ULP 判断，形成第二套规则；
- 把 `stdlib/cmath.xiao` 写成已可导入/已验证；
- 提前改 registry、教程或接口版本；
- 增加默认参数、重载、别名或 sfloat 入口；
- 顺手接类型/IR/VM/LLVM，把 B3-4 扩成 B4；
- 用 B3-3 C ABI 证据冒充 `std.cmath` 语言可用性；
- 运行旧 721 或关闭历史债务。

## 七、交接

B3-4 完成后进入[B4 cmath 语言全链路与承载规划](20o-b2-b4-language-integration-plan.md)：类型/IR/TAC/VM/LLVM/序列化双路径，`std.cmath`/`stdlib.cmath` 导出、移除、归档和普通包隔离；B4 前把两处正式向量迁入 `tests/fixtures/docs/cmath/`。B5 切 ABI 1.19/21，B6 性能，B7 收口，B2-C 最终 721/发布。

新 Agent 先读：

1. [B3 cmath 规划](20o-b2-b3-cmath-plan.md) §7.1；
2. [B3-3A/B 记录](20o-b2-b3-3a-b-progress.md)；
3. [B3-3 证据](assets/20o-b2-b/entry-20261109-b3-cabi/README.md)；
4. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
5. [F-12 错误表](20o-b2-freeze-b3-error-table-plan.md)。

## 八、落点

```text
stdlib/cmath.xiao                                    15 个公开包装
stdlib/cmath.md                                      模块说明（planned）
docs/UseDocs/library/cmath.md                        教程状态（planned）
docs/module-registry.json                            stdlib.cmath 保持 planned
docs/DevDocs/assets/20o-b2-b/entry-<date>-b3/        B3-4 证据
```

本页只规划 B3-4；B4 语言全链路、B5 版本与 B2-C 仍后置。

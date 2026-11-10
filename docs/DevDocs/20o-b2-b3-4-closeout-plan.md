# 20O-B2-B3-4. `stdlib/cmath.xiao` 源码包装收口规划

> **B3-4 planned 源码包装范围已独立收口；不代表语言可用。证据与只读审计见 `entry-20261110-b3-wrapper-final`。**
> 入口为 B3-3 代码 `89d782e1`、B3-4 包装提交 `6ddc0c35`、802/可达性修复 `ea9e33ba`/`7d8b85e4`，以及修复证据。
> B3-4 只收口“planned 源码包装 + 隔离修复”，不是 `std.cmath` 语言可用；类型/IR/TAC/VM/LLVM 仍留 B4。
> `stdlib.cmath` registry 仍 planned；不升 ABI 1.19/21，不跑旧 721。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| 包装源码 | `stdlib/cmath.xiao` 15 个 `float -> float` 转发包装 |
| 模块说明 | `stdlib/cmath.md`、UseDocs cmath 均 planned |
| 当前代码 | 产品代码 `7d8b85e4`（包装 + 802/可达性修复）；B3-CABI 历史代码 `89d782e1` |
| 测试 | wrapper 8/0、22c 29/0、reachability 30 用例、B3-CABI 固定审计 22 项 |
| 收口证据 | [独立回执与审计](assets/20o-b2-b/entry-20261110-b3-wrapper-final/README.md)；当前状态以最终 `closeout.json` 为准 |
| 未做 | B4 语言全链路、B5 版本、B6 性能、B2-C 721 |
| 状态 | `stdlib.cmath` planned；语言不可用；B3-4 只对源码包装与隔离回归收口 |

B3-4 收口只确认“包装源码、planned 边界和隔离修复可复核”，不能写成 `std.cmath` 可执行、B3 整批完成或 B2-C 发布。

本机收口结果绑定固定代码 `7d8b85e4`：wrapper 8/0、并发 29/0、LLVM 56 单测和4项标准库绑定回归、archive wiring 11/0、Rust workspace测试、Clippy、fmt、锁与仓库文档门禁通过；B1 18/18、B2 14项/4个精确目标及B3-CABI 22项为固定历史审计。完整命令和原始日志见[收口证据](assets/20o-b2-b/entry-20261110-b3-wrapper-final/README.md)。审计不要求当前HEAD等于产品提交，但校验其为HEAD祖先且受保护产品树未变。

### 1.1 要交付

1. 独立 B3-4 证据目录与只读 `audit.py`；
2. `stdlib/cmath.xiao` 的 15 个导出、参数名、原语调用和“无数学实现”证据；
3. `stdlib/cmath.md`、UseDocs、stdlib README、registry planned 的一致性；
4. wrapper 8/0、22c 29/0、reachability 30、B3-CABI 22 项执行身份；
5. 802 修复和可达性修复的历史失败/最终日志绑定；
6. 30 条 cmath 导入路径的明确拒绝断言；
7. 无类型/IR/TAC/VM/LLVM cmath 绑定的边界证明；
8. B4 入口与向量迁移清单。

### 1.2 明确不做

- 不接类型/IR/TAC/VM/LLVM/归档执行；不把 30 条路径改成可执行；
- 不升级 registry/教程，不写 `std.cmath` 已可用；
- 不新增 intrinsic/C 符号，不改 1～48/ID 100；
- 不升 ABI 1.19/21，不改锁/工作流；
- 不运行旧 721，不关闭历史债务；
- 不把 B3-4 源码收口写成 B4 语言验收。

## 二、开工闸门

1. B3-3 审计、B3-CABI 审计、B3-4 修复证据均通过；
2. `stdlib/cmath.xiao` 15 个导出、参数与原语一致；无额外顶层状态/辅助函数；
3. wrapper 8/0、22c 29/0、reachability 30 项和 B3-CABI 22 项可复现；
4. 30 条 `std.cmath`/`stdlib.cmath` 导入路径仍未接线；
5. registry/UseDocs/stdlib README 均 planned；
6. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、分批执行

### B3-4-0：代码与证据快照

1. 固定产品代码 `7d8b85e4`、包装源码 `stdlib/cmath.xiao`、模块说明 `stdlib/cmath.md`；
2. 固定 B3-CABI 历史代码 `89d782e1` 与其 86 文件审计；
3. 固定 802 修复证据 `entry-20261110-b3-fix-802` 和可达性证据 `entry-20261110-b3-reachability`；
4. 记录源码清单、锁、工具链、目标、Runtime 和独立任务根；
5. 不复用 B3 的 Runtime/缓存/执行产物。

退出：B3-4 代码/证据身份可复现；无未登记产品差异。

### B3-4-1：独立审计设计

1. 新建 `entry-20261110-b3-wrapper-final/audit.py`，只读、固定 Git 对象、不写清单；
2. 允许集合覆盖：
   - `stdlib/cmath.xiao`、`stdlib/cmath.md`；
   - `core/rust/crates/xiao-driver/tests/20o_b2_b3_wrapper.rs`；
   - `core/rust/crates/xiao-codegen-llvm/src/dynamic.rs`/`dynamic/imports.rs`/`dynamic/README.md`；
   - `core/rust/crates/xiao-codegen-llvm/tests/stdlib_bindings.rs`；
   - `core/rust/crates/xiao-driver/tests/22c_vm_concurrency.rs`；
   - `docs/UseDocs/library/cmath.md`/`README.md`、`stdlib/README.md`；
   - 必要的测试/证据路径；
3. 拒绝任何 intrinsic 34～48 绑定、C ABI 改动或 `std.cmath` 可执行导入；
4. 核对 wrapper 8/0、22c 29/0、reachability 30、B3-CABI 22；
5. 核对 30 条 cmath 路径仍为 planned 拒绝；
6. 核对 registry/UseDocs/stdlib README 仍 planned；
7. 核对 802/可达性修复日志哈希和源码身份；
8. 拒绝把“测试观察成功”当“语言可用”。

退出：审计可解释 B3-4 每个执行点；不能通过删断言/ignore 变绿。

### B3-4-2：执行与证据

1. 运行 wrapper 8 项、22c 29 项、reachability 30 项、B3-CABI 22 项；
2. 运行 B1/B2 历史审计与 B3-3 固定证据；
3. 运行 docs/usedocs/layout、fmt、clippy、workspace 门禁；
4. 保存原始日志、PID、退出码、源码前后摘要；
5. 保留 802 原失败、首轮 codegen/clippy 失败和修复日志；
6. 保存 Windows 本机与未执行平台说明；
7. 形成只读 `closeout.json`，明确 `b3_4_closed=true` 仅限源码包装，`language_available=false`。

退出：B3-4 源码包装证据完整；没有语言可用或版本升级声明。

### B3-4-3：B4 交接

1. 列出 15 项包装 → 原语 → intrinsic 34～48 的映射；
2. 列出 B4 必须完成的类型/IR/TAC/VM/LLVM、静态 pow 共核、双路径、归档；
3. 列出两处向量迁移 `tests/fixtures/docs/cmath/`；
4. 列出 30 条 planned 路径在 B4 的可执行语料更新方式；
5. 形成 B4 入口；B5/B6/B2-C 仍后置。

退出：B4 入口明确；B3-4 只算源码包装收口。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 导出 | 15 项、参数名/数量/float 返回、无额外顶层 |
| 原语 | 每项只调 `cmath_*`，无 Xiao 数学/域逻辑 |
| 状态 | `stdlib.cmath` planned、UseDocs/stdlib README 一致 |
| 测试 | wrapper 8/0、22c 29/0、reachability 30、B3-CABI 22 |
| planned 边界 | 30 条导入路径全部拒绝，无意外 executable |
| 隔离 | hex 原生不预加载 cmath；子VM/归档别名规范 |
| 历史 | 802/可达性失败与修复日志保留 |
| 边界 | 无 intrinsic/C ABI/类型/IR/VM/LLVM/ABI/721 改动 |

## 五、退出条件

1. `stdlib/cmath.xiao` 恰好 15 个转发包装；
2. wrapper 8/0、22c 29/0、reachability 30、B3-CABI 22 项通过；
3. 30 条 cmath 路径仍明确拒绝；`language_available=false`；
4. registry/UseDocs/stdlib README 仍 planned；
5. 无 intrinsic 34～48 语言绑定、C ABI/VM/LLVM 改动；
6. ABI 1.18/生成器 20/接口 1/1～48/ID 100 未变；
7. 旧 721 未运行；历史债务未核销；B4 未开始；
8. B4 向量迁移/双路径清单已交接。

## 六、风险与禁止

- 把 wrapper 测试的“拒绝”写成 cmath 可用；
- 通过改断言/ignore 让 30 条路径变绿；
- 把 802/可达性修复当 cmath 语言接线；
- 提前升级 registry/教程或改 ABI；
- 删除 802/首轮失败日志；
- 把 B3-4 源码收口写成 B3 整批或 B2-C 发布。

## 七、交接

B3-4 收口后进入 B4：类型/IR/TAC/VM/LLVM/序列化双路径，静态 pow 共核，`std.cmath`/`stdlib.cmath` 导出/移除/归档/隔离；B4 前迁移两处正式向量。

新 Agent 先读：

1. [B3-4 包装规划](20o-b2-b3-4-stdlib-wrapper-plan.md)；
2. [B4 语言全链路规划](20o-b2-b4-language-integration-plan.md)；
3. [802 修复证据](assets/20o-b2-b/entry-20261110-b3-fix-802/README.md)；
4. [可达性证据](assets/20o-b2-b/entry-20261110-b3-reachability/README.md)；
5. [B3-3 记录](20o-b2-b3-3a-b-progress.md)。

## 八、落点

```text
stdlib/cmath.xiao                                    15 个包装
stdlib/cmath.md                                      模块说明
docs/UseDocs/library/cmath.md                        教程 planned
core/rust/crates/xiao-driver/tests/20o_b2_b3_wrapper.rs  包装/隔离/planned 边界
core/rust/crates/xiao-codegen-llvm/src/dynamic/imports.rs 可达导入
docs/DevDocs/assets/20o-b2-b/entry-20261110-b3-wrapper-final/  B3-4 收口证据
```

本页只做 B3-4 源码包装收口；B4 语言全链路、B5 版本与 B2-C 仍后置。

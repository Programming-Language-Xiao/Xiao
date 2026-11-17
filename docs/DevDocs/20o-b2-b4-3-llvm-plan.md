# 20O-B2-B4-3. cmath LLVM 动态/静态与共享 pow 实施规划

> **2026-11-13 版本裁定执行：星崽在本会话已批准“B4 升到生成器21，更新规划并调整B5边界”。B4-3改LLVM调用文本，按后端版本规则20→21；ABI仍1.18、接口1。B5只将ABI升1.19并核验既有生成器21的最终拒绝/重生成矩阵，不再次因本批接线递增生成器。下文将21留B5或B4不得升21的旧句均由本裁定替代；签署的冻结正文及历史证据不改写。**

> **B4-2A、FIX1、B4-2B与B4-3已完成Windows本机内部退出；B4-4、B4-5、B5～B7与B2-C后置。B4-3 fixed-final2 的四组静态/动态O0/O3、回归和只读证据审计均通过，见[实施证据](assets/20o-b2-b/entry-20261113-b4-3/README.md)及其`closeout.json`。**
> **2026-11-15 回归与修复：B4-3 的全量 `numeric_range` 豁免使 `n0_b_dynamic_native` 递归用例失去 `weak` 依赖，B4-5 因此阻塞；星崽裁定收窄豁免到 float pow，见[FIX.1 规划](20o-b2-b4-3-fix1-numeric-range-routing-plan.md)。fixed-final2 历史证据不改写。**
> **2026-11-18 FIX.2 已完成**：共享组件推导覆盖 LLVM `switch` 可达路径，Windows/Linux真实Cmath包装两别名O0/O3逐字通过；Linux ELF `Complete` 的声明/观察组件一致，Windows COFF仍标 `UnverifiedCoffExports`。见[FIX.2证据](assets/20o-b2-b/entry-20261118-b4-component-fix/README.md)。B6-1仍须在自身任务根重跑；fixed-final2历史证据不改写。
> 入口为 B4-1 类型/IR、B4-2 VM 执行和已冻结的 15 个 Rust-only Runtime 门面；当前LLVM静态/动态调用与共享Float pow已通过fixed-final2验证。
> B4-3 只做 LLVM 动态/静态路径与 float `**` 共核；按已批准版本裁定升生成器20→21，ABI维持1.18；不接导入/归档/序列化（B4-4）。
> 30条cmath导入路径仍planned拒绝；`language_available=false`。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| 类型/IR | 15 项 `IntrinsicCall` 保留 ID/参数/span（B4-1） |
| VM | B4-2 提供 VM 分派与错误/清理；本批不复用 VM 状态机 |
| Runtime | 15 个 `xiao_runtime::cmath_*` Rust 门面与 15 个 C ABI 符号 |
| 静态 LLVM | 开工快照Float `**`用`llvm.pow.f64`；当前已切共享cmath_pow，sfloat保留`llvm.pow.f32` |
| 动态 LLVM | 开工快照34～48未lowering；接线、fixed冷构建与O0/O3矩阵已通过 |
| 版本 | B4-3前ABI 1.18/生成器20；本批后ABI 1.18/生成器21，接口1 |

B4-3是**LLVM后端内部验收**，不是`std.cmath`导入可用。B4-4才接双路径/归档/序列化；B5只切ABI1.19，生成器沿用21。

### 1.1 要做

1. LLVM 动态路径为 34～48 生成 Runtime C ABI 调用；
2. 静态路径为 15 项 cmath 生成同一 Runtime 内核调用；
3. 静态 Float `**` 从 `llvm.pow.f64` 切到共享 cmath_pow 内核；sfloat 的 `llvm.pow.f32` 保持原路径；
4. 正确处理 C ABI 的借读输入、拥有 out、i32 状态和挂起错误；
5. 保留 009/002 错误身份、source span 和既有静态/动态错误通道；
6. 保持 sfloat `**` 现有路径，不扩大范围；
7. 保存动态/静态 O0/O3、符号、错误和回归证据；
8. 保持 30 条 cmath 导入路径拒绝。

### 1.2 明确不做

- 不接 `std.cmath`/`stdlib.cmath` 导入、导出、移除、归档（B4-4）；
- 不接序列化/并发承载（B4-4）；
- 不新增intrinsic49+/C符号，不改1～48/ID100；
- ABI保持1.18；生成器由20升21，因为LLVM调用文本已变化；不改registry/教程/锁；
- 不实现 sfloat cmath；
- 不跑旧 721，不关闭历史债务。

## 二、开工闸门

1. B4-2A/B 审计通过，VM 15 项、TAC 往返、30 条 planned 拒绝可复现；
2. 15个C ABI符号、17,243条正式参考、1,075行pow历史差异fixture和共享Runtime内核可复现；
3. 开工历史快照静态Float `**`指向`llvm.pow.f64`、sfloat `**`指向`llvm.pow.f32`；当前B4-3接线提交为`e0d5993e`，最终执行由fixed-final2快照记录；
4. 开工基线ABI1.18/生成器20/接口1的入口可复现；当前接线后ABI仍1.18、生成器21；
5. B4-0/B4-1、B3-4、B1/B2/B3 回归可复现；
6. 唯一任务根、独立 target/Runtime/缓存已建立。

### 2.1 开工单（2026-11-13）

1. 入口：B4-2 Windows 内部退出记录 [closeout](assets/20o-b2-b/entry-20261113-b4-2b/closeout.json)；数学/Runtime 实现 `246bea46`，自审最终产品 `7c49fd65`（模块拒绝消息与测试）；驱动 50/50、Runtime 272/272、VM 35/35、字节码 65/65。开工前复核 `246bea46..7c49fd65` 的产品差异仅限 module_loader 消息、VM 专项测试与说明；若出现其它产品差异，按实际差异重新登记并重跑对应回归。
2. 开工源码为ABI1.18/生成器20：静态Float `**`仍为`llvm.pow.f64`、sfloat仍为`llvm.pow.f32`，34～48无LLVM lowering。当前B4-3最终产品快照为`e0d5993e`，独立fixed-final2任务和全部执行见本页证据。
3. 必须保留：B4-2 的 15 项 Runtime 门面/VM 分派、FIX1 planned 门控、30 条导入后调用拒绝、22c 29/29、子 VM/归档边界 3 项，以及 B1/B2/B3/B3-4/B4-0/1/B4-2 固定审计；不得靠放宽、删除或忽略测试推进。
4. 顺序：B4-3A动态→B4-3B静态→B4-3C测试→B4-3D收口。生成器20→21是本会话已授权范围；若还需改checker、intrinsic表、C ABI或再改版本，须另立裁定。
5. 证据：新建 `assets/20o-b2-b/entry-20261113-b4-3/`；原始日志按 SHA-256 内容寻址，同一内容只存一个对象；每次运行保存 PID、时间、argv、cwd、环境、源码/DevDocs 摘要和退出码。文档修改合并到定稿后只跑一次最终 docs 门禁，中间轮次不重复全量门禁；B4-2/FIX1 历史目录不改写。
6. 特别核对：静态 `float **` 切到共享 `cmath_pow` 后，`llvm.pow.f64` 不再作为 Float `**` 的第二实现；sfloat保持 `llvm.pow.f32`；将B3-2A的1075组pow差异迁至`tests/fixtures/docs/cmath/pow-legacy-diff.tsv`，逐行与正式参考输入/共享pow黄金绑定，并在静态/动态O0/O3对全部输入核对Float `**` 与 `cmath.pow` 位模式/错误一致。
7. 不做：B4-4双路径/归档/序列化、ABI 1.19；本批生成器21，registry/教程和旧721；Linux/macOS、远端CI、性能不具备时如实登记。

## 三、分批执行

### B4-3A：动态 LLVM 路径（已实现并通过fixed验收）

1. 为 34～48 生成 `xiao_runtime_cmath_*` 调用；
2. 一元：一个借读输入 + 拥有 out；二元：min/max/pow 两个输入 + out；
3. 参数按源码顺序求值，out 槽初始化且不复用输入；
4. 状态 0 读取 out；状态 6 保留 out 并沿既有错误通道；
5. 1/2/3 结构错误不碰挂起槽，按既有 Fatal/状态处理；
6. 不因 `Pure` 把调用当可删除；
7. 保存动态 IR、C 符号、状态/错误和 O0/O3 证据。

退出：动态 LLVM 可执行 15 项 cmath；错误/所有权与 C ABI 一致。

### B4-3B：静态 LLVM 路径（已实现并通过fixed验收）

1. 为 15 项 cmath 生成 `xiao_runtime_cmath_*` 调用；静态产物设置 `uses_runtime=true`；
2. 静态 float `**` 从 `llvm.pow` 改为同一 cmath_pow 调用；
3. 保留 `sfloat **` 现有 `llvm.pow.f32` 或既有路径，不扩大；
4. 状态非零时沿既有静态错误通道（raise/report/exit）处理；不伪造可恢复 Fatal；
5. 输入/out 复用与释放按既有静态 ABI 规则；
6. 不改 `.xiaoc`/`.xar`/原生元数据版本；
7. 保存静态 IR、符号、O0/O3、错误/清理证据。

退出：静态/动态 float `**` 与 cmath 共用内核；无 `llvm.pow.f64` 第二实现。

### B4-3C：测试与证据（Windows fixed-final2已通过）

1. 15 项 cmath 动态/静态 O0/O3；
2. float `**` 与 cmath_pow 同输入位模式/错误；历史1075组pow差异按固定表逐条对账；
3. 002/009、状态 1/2/3/6、挂起/错误通道；
4. 15 个 C 符号存在、无 cmath 第二内核；
5. 30 条 planned 导入路径仍拒绝；
6. B4-0/1/2、B3-4、B1/B2/B3回归；逐目标计数，保留已有ignore身份；pow历史差异表1075行精确核对；
7. 保存源码、工具链、目标、PID、日志、前后摘要；
8. 记录 Windows 本机与未执行平台。

fixed-final2 冷构建的 Runtime `xiao_runtime.lib` 为27,482,962字节，SHA-256 `27b460d2fde00c0e36f2799474e3a21310d5555530cbc8d7694caf6df220702d`。静态/动态 O0/O3 四个产物各自核对18,318项、3,153个错误结果和1,075组 pow 输入；状态0/1/2/3/6/8、错误009、坏标签out保持、语言退出/catch也通过。B1 VM/原生精确入口、B2-A/B原生入口、七项历史审计、workspace测试、Clippy与fmt通过；30条planned导入仍拒绝。

2026-11-15 审核补记：本批修改`dynamic/predicate.rs::program_uses_runtime`，使仅含`numeric_range`检查、且没有其它动态触发条件的定宽标量程序走静态降低；它也影响不含cmath的程序。该接线依赖静态算术、有限值和窄化检查落实数值边界，其余检查、堆值、模块等仍会触发动态路径。B4-3的B1/B2 VM及原生语料已通过，但没有专门断言“仅numeric_range检查”的静态/动态对照用例；cmath矩阵不能代替这项证明。[B5测试清单](20o-b2-b5-version-matrix-plan.md)已登记该专项，B4-5交接保留这个覆盖限制。历史回执与收口JSON保持原始身份。

生成器21已生效；B4-3的版本证据限于独立冷构建及本批O0/O3执行。20/21原生元数据接受/拒绝和旧缓存miss/重生成的完整矩阵留B5。当前元数据验证器在ABI、目标、特性及标准库身份匹配时接受生成器14～21，旧20原件可被接受；缓存指纹含生成器号，这一接受行为不等于缓存命中，B4-3没有实跑旧缓存失效探针。

退出：Windows内部LLVM动态/静态证据完整；ABI1.18/生成器21；B4-4未开始；language false。

### B4-3D：收口与B4-4交接（Windows内部已完成）

1. 复核 15 项 cmath/float pow 共核；
2. 复核无导入/归档/序列化/ABI/registry 改动；
3. 复核 30 条 planned 仍拒绝；
4. 形成 B4-4 入口：`std.cmath`/`stdlib.cmath` 双路径、导出/移除/归档/序列化。

退出：B4-3 Windows内部收口；ABI1.18/生成器21；B4-4未开始；cmath仍planned。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 动态 | 15 项、C 符号、状态/out、009/002 |
| 静态 | 15 项、uses_runtime、O0/O3、错误/清理 |
| 共享 pow | float `**`/cmath_pow 同输入位模式；无 `llvm.pow.f64` |
| sfloat | 保持既有 `llvm.pow.f32` 路径 |
| 边界 | 无导入/归档/序列化/ABI/registry/721 |
| 回归 | B4-0/1/2、B3-4、B1/B2/B3 |
| 导入 | 30 条 planned 仍拒绝 |

## 五、退出条件

1. 动态/静态 LLVM 均执行 15 项 cmath；
2. float `**` 与 cmath_pow 共用内核，静态无 `llvm.pow.f64` 第二实现；
3. C ABI 状态/out/错误通道正确；
4. sfloat `**`、30 条 planned 导入、registry/cmath 状态未变；
5. 无导入/归档/序列化/ABI/registry 改动；
6. B4-0/1/2、B3-4、B1/B2/B3固定审计与本批回归通过；旧721未跑；
7. B4-4 未开始；Linux/macOS、远端CI、性能仍未执行或声称。

## 六、风险与禁止

- 静态路径继续保留 `llvm.pow`，形成第二内核；
- 动态/静态各自实现 cmath，绕过 Runtime；
- 状态 6 被当成功，或 1/2/3 写挂起槽；
- 把 `Pure` 当可删除；
- 提前接导入/归档/序列化或改 ABI/registry；
- 用 B4-3 的 Windows 本机证据冒充三平台或 B2-C。

## 七、交接

B4-3 Windows内部退出已交付；下一阶段为B4-4。B4-5整批收口仍后置；B5只将ABI切到1.19并执行矩阵，沿用生成器21。

新 Agent 先读：

1. [B4 语言全链路规划](20o-b2-b4-language-integration-plan.md)；
2. [B4-2A/B 执行规划](20o-b2-b4-2a-b-execution-plan.md)；
3. [B4-0/B4-1 记录](assets/20o-b2-b/entry-20261110-b4-0-1/README.md)；
4. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
5. [F-9 共享内核](20o-b2-freeze-draft.md#七cmath行为误差与二进制转换f-2f-8f-9已确认)。

## 八、落点

```text
core/rust/crates/xiao-codegen-llvm/src/dynamic/       动态 cmath 调用与状态映射
core/rust/crates/xiao-codegen-llvm/src/static_cmath.rs 静态标量ABI窄桥
core/rust/crates/xiao-codegen-llvm/src/ir.rs          静态 cmath/float pow
core/rust/crates/xiao-runtime/src/                    15 个 Rust/C ABI 内核
core/rust/crates/xiao-codegen-llvm/tests/             动态/静态 O0/O3
docs/DevDocs/assets/20o-b2-b/entry-<date>-b4/         B4-3 证据
```

本页记录B4-3；Windows fixed执行及pow差异核对已完成并形成内部交接。B4-4起、B5～B7与B2-C仍后置。

# 20O-B2-B4. cmath 语言全链路与承载实施规划

> **B3-4 planned 源码包装收口已完成；B4 语言全链路仍未开始。**
> 入口为 B3-3 代码 `89d782e1`、B3-4 固定代码 `7d8b85e4` 与其源码收口证据；当前 intrinsic 34～48 仅内部 active，VM/LLVM 未绑定。
> B4 只做类型/IR/TAC/VM/LLVM/序列化与双路径承载；不升 ABI 1.19/21，不升级 registry，不跑旧 721。
> `stdlib.cmath` 仍 planned；B4 的通过只作为内部语言验收，B5 切版本，B2-C 才最终发布。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B3-3 | 15 个 C ABI、intrinsic 34～48 内部 active；无 VM/LLVM 绑定 |
| B3-4 | 15 个 planned 源码包装和隔离回归已独立收口；registry 仍 planned |
| 当前版本 | ABI 1.18、生成器 20、标准库接口 1、活动 intrinsic 最大 48（仅内部） |
| 本批目标 | 类型/IR、TAC/字节码、VM、LLVM、序列化、`std.cmath`/`stdlib.cmath` 双路径 |
| 未做 | B5 版本矩阵、B6 资源/性能、B7 收口、B2-C 721/最终教程 registry |
| 状态 | cmath 仍 planned；最终公共发布不成立 |

B4 是 `cmath` 的**语言全链路内部验收批**，不是最终发布。B5 才切 ABI 1.19/生成器 21，B2-C 才做最终 721 与状态升级。

### 1.1 要做

1. B4-0 把两处产品测试正式向量迁入 `tests/fixtures/docs/cmath/`，DevDocs 保留副本与摘要；
2. 类型检查接受 34～48 的一元/二元 `float` 原语，禁止隐式数值提升和新转换；
3. IR/TAC/字节码承载 34～48，不提前改容器版本；
4. VM 按 ID 绑定 15 项共享内核，保留 source span、错误传播和清理；
5. LLVM 动态与静态路径接入同一内核；静态路径不得继续把 `llvm.pow` 当第二实现；
6. `std.cmath` 与 `stdlib.cmath` 双路径、导出、逐函数移除、归档和普通包隔离；
7. 序列化/并发值承载按既有边界，不暴露 Runtime 布局；
8. 保存 VM/native/归档/序列化/错误/资源证据，形成 B5 入口。

### 1.2 明确不做

- 不升 ABI 1.19/生成器 21，不改 `.xiaoc`/`.xar`/原生元数据版本；
- 不升级 `stdlib.cmath` registry 或最终教程；
- 不新增 intrinsic 49+、C 符号 34～48 之外入口或宿主能力；
- 不实现 sfloat cmath、默认参数、重载、别名或新数值自动转换；
- 不运行旧 721，不关闭 22C/22D、19/11/22；
- 不把 B4 内部语言验收写成 B2-C 最终发布或三平台公共产品。

## 二、开工闸门

1. B3-0/B3-1、B3-2A、B3-3A/B 固定审计通过；
2. B3-4 源码包装 closeout 与 30 条 planned 拒绝回归通过，registry 仍 planned；
3. intrinsic 1～48 活动、ID 100 墓碑、ABI 1.18/生成器 20/接口 1 未变；
4. 15 个 C 符号、17,243 条正式参考和共享 pow 证据可复现；
5. 两处 `include_str!` 的向量路径与 B4-0 迁移方案已确认；
6. B1/B2/B3 回归、28/28 守卫和独立 Runtime 身份可复现；
7. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、分批执行

### B4-0：向量迁移与语言基线

执行细化见[B4-0/B4-1 开工规划](20o-b2-b4-0-b4-1-kickoff-plan.md)；B4-0只迁移向量与基线。

1. 将 `cmath_tests.rs` 与 `abi_cmath_tests.rs` 使用的正式向量迁入 `tests/fixtures/docs/cmath/`；
2. DevDocs 证据保留历史副本与摘要，逐字节核对迁移副本与原副本一致；
3. 更新两处测试的 `include_str!` 指向新 fixture，重新编译并运行；
4. 记录迁移前后源码身份、向量摘要、测试入口和输出；
5. 保存 B3-3 的 15 C 符号、34～48 声明、共享 pow 和 B1/B2 基线；
6. 建独立语言任务根，不复用 B3 的 Runtime/缓存/执行产物。

退出：产品测试不再依赖 DevDocs 路径；向量字节一致；基线通过。

### B4-1：类型检查与 IR

执行细化见[B4-0/B4-1 开工规划](20o-b2-b4-0-b4-1-kickoff-plan.md)；B4-1不接TAC/VM/LLVM。

1. 识别 34～48 的一元/二元 `float -> float` 原语；
2. 参数全部必填；一元 `value`，二元 `left/right` 或 `base/exponent`；
3. 支持既有位置/命名调用，不增加默认值、重载、别名；
4. 不新增隐式数值提升/窄化；静态非法类型走既有类型诊断；
5. IR 保留 34～48 的 ID、参数顺序和 source span；
6. 不把可 catch 的 009 折叠成编译错误；
7. 保存类型/IR 负例和 IR 形状。

退出：类型/IR 可表达 15 项 cmath；没有隐式新转换。

### B4-2：TAC/字节码与 VM

执行细化见[B4-2 TAC/字节码与VM规划](20o-b2-b4-2-tac-vm-plan.md)；B4-2不接LLVM/导入/归档。

1. TAC/字节码按 34～48 编码，使用既有标签/容器版本；
2. VM 按 ID 绑定共享内核，不复制数学实现；
3. 错误传播保留 source span、挂起错误和可 catch 路径；
4. 调用失败/成功清理、out 所有权和临时值释放沿既有规则；
5. 不把 34～48 仅因 `Pure` 就从“结果未使用”删除；
6. 保存字节码往返、VM 15 项、错误/span/清理证据。

退出：VM 可执行 15 项 cmath，错误/清理与既有语言路径一致。

### B4-3：LLVM 动态与静态路径

执行细化见[B4-3 LLVM与共享pow规划](20o-b2-b4-3-llvm-plan.md)；B4-3不接导入/归档/序列化。

1. 动态降低调用 `xiao_runtime_cmath_*` 或既有动态 Runtime 入口，不另写数学；
2. 静态路径必须接入同一内核；不得继续用 `llvm.pow.f64/f32` 作为 float `**` 的第二实现；
3. 若静态路径当前不链接 Runtime，先停下评估链接/符号方案，不能以两个近似实现绕过；
4. 15 项 cmath 的静态/动态路径参数、返回、错误传播一致；
5. 保留 `sfloat **` 现有路径，不扩大 B4 范围；
6. 保存原生 O0/O3、符号、错误和回归证据。

退出：静态/动态 float `**` 与 cmath 共用内核；无第二实现。

### B4-4：双路径、归档、序列化与隔离

执行细化见[B4-4 双路径/归档/序列化规划](20o-b2-b4-4-dual-path-archive-plan.md)；B4-4不改容器schema/ABI。

1. `std.cmath` 与 `stdlib.cmath` 双路径由固定标准库加载器提供；
2. 15 个公开包装真实导出，逐函数移除后导入失败；
3. 真实 `.xar`/`.xiaoc` 归档包含 34～48 调用并可重放；
4. 普通包隔离：不把 cmath 写入 PackageRegistry/依赖锁；
5. 序列化/并发值承载按既有文本标签/边界，不暴露 Rust 布局；
6. 不新增宿主能力；registry/cmath 仍 planned；
7. 保存导出/移除/归档/隔离/序列化证据。

退出：双路径和归档一致；cmath 仍 planned。

### B4-5：证据与 B5 交接

执行规划见[B4-5 语言全链路内部收口](20o-b2-b4-5-closeout-plan.md)；B4-5只做内部收口与B5交接。

1. 15 项 cmath 在类型/IR/TAC/VM/LLVM 的完整路径；
2. 15 个 C 符号/Runtime/声明/包装一一对应；
3. 错误/资源/清理/span/out/序列化/归档/隔离证据；
4. B1/B2/B3 回归：20 入口、资源、双精确、28 守卫、33 单入口；
5. float `**`/cmath.pow 共享内核和 1,075 组差异保持；
6. 记录 Windows 本机和未执行平台；B5/B6 补三平台与性能；
7. 形成 B5 入口：ABI 1.19/生成器 21 与拒绝矩阵。

退出：B4 内部语言证据完整；B5 未开始；cmath 仍 planned。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 向量 | 迁移副本与 DevDocs 原副本逐字节一致；两处 include 更新 |
| 类型/IR | 15 项、一元/二元、无默认/重载、无新提升、源码 span |
| TAC/字节码 | 34～48 编码/往返、容器版本未变 |
| VM | 15 项结果、009、清理、挂起错误 |
| LLVM | 动态/静态同内核、O0/O3、符号、float ** 无第二实现 |
| 双路径 | `std.cmath`/`stdlib.cmath` 导出、逐函数移除、普通包隔离 |
| 归档/序列化 | `.xar`/`.xiaoc` 重放、XCON/文本承载未变 |
| 回归 | B1/B2/B3 20/资源/双精确/28/33 |
| 边界 | ABI1.18/20、cmath planned、registry 未改、721 未跑 |

## 五、退出条件

1. 两处正式向量已迁移，产品测试不再依赖 DevDocs；
2. 15 项 cmath 在类型/IR/TAC/VM/LLVM 全链路一致；
3. 静态/动态 float `**` 与 cmath 共用同一内核，无 `llvm.pow` 第二实现；
4. `std.cmath`/`stdlib.cmath` 双路径、导出/移除/归档/隔离通过；
5. 错误/资源/清理/span/out/序列化证据完整；
6. B1/B2/B3 回归通过；旧 721 未运行；历史债务未核销；
7. ABI 1.18/生成器 20/标准库接口 1/cmath planned 未变；
8. B5 未开始；三平台/性能仍未声称；B2-C 未开始。

## 六、风险与禁止

- 静态 LLVM 继续用 `llvm.pow`，形成第二套 float `**`；
- VM/C ABI 各自实现 cmath，绕过共享内核；
- 把 34～48 只加类型/IR 未绑 VM/LLVM 就宣称语言可用；
- 把 `std.cmath` 写成已 verified 或修改 registry；
- 向量迁移只改测试路径、不核对字节；
- 归档/序列化暴露 Runtime 布局或改容器版本；
- 提前升 ABI 1.19/21、跑 721 或关闭历史债务。

## 七、交接

B4 完成后进入 B5：ABI 1.19、生成器 21、旧/新产物拒绝矩阵。B6 做资源/性能，B7 收口，B2-C 做最终 721、三平台公共产物与教程/registry。

新 Agent 先读：

1. [B3 cmath 规划](20o-b2-b3-cmath-plan.md) §7.1；
2. [B3-3A/B 记录](20o-b2-b3-3a-b-progress.md)；
3. [B3-4 包装规划](20o-b2-b3-4-stdlib-wrapper-plan.md)；
4. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
5. [F-9 共享内核](20o-b2-freeze-draft.md#七cmath行为误差与二进制转换f-2f-8f-9已确认)；
6. [F-12 错误表](20o-b2-freeze-b3-error-table-plan.md)。

## 八、落点

```text
core/rust/crates/xiao-types/                         类型检查
core/rust/crates/xiao-ir/                            IR
core/rust/crates/xiao-bytecode/                      TAC/字节码
core/rust/crates/xiao-vm/                            VM 绑定
core/rust/crates/xiao-codegen-llvm/                  动态/静态降低、静态 pow 共核
core/rust/crates/xiao-runtime/                       共享内核/C ABI
stdlib/cmath.xiao                                    15 个包装
tests/fixtures/docs/cmath/                           迁移后的正式向量
docs/DevDocs/assets/20o-b2-b/entry-<date>-b4/         B4 证据
```

本页只规划 B4；B4 尚未实现，B5～B7 与 B2-C 仍后置。

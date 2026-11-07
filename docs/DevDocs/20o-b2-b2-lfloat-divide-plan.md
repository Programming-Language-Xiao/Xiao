# 20O-B2-B2. `lfloat_divide` 与 intrinsic 33 实施规划

> **B2 单入口已在 `37b92c53` 实现，Windows 内部验收见[实施记录](20o-b2-b2-lfloat-divide-progress.md)；B3～B7 与 B2-C 仍未开始。**
> 开工入口为 `faecd26d`，复用 B1 私有内核代码 `35872782`；当前内部快照是 ABI 1.18、生成器20、活动 intrinsic 最大33，仅供内部验证。
> B2 只实现全局 `lfloat_divide` / intrinsic 33，不实现 15 项 `cmath`、不升 ABI 1.19/21、不改 registry。
> `stdlib.cmath` 仍 planned，旧 721 条 F10 不运行，历史债务保持独立。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B0/B1 | B0 基线已完成；B1 私有 `libm`/`cmath`/显式除法内核已在 `35872782` 交付并通过只读审计 |
| 内部快照 | ABI1.18、生成器20、活动intrinsic最大33、标准库接口1；B5切最终版本 |
| 本批目标 | 全局 `lfloat_divide(lfloat, lfloat, int, str) -> lfloat`、intrinsic 33、C ABI 与 VM/LLVM 接线 |
| 版本策略 | B2 只在内部快照激活 33；B5 才统一升 ABI 1.19/生成器 21 |
| 未激活 | intrinsic 34～48、`std.cmath`、`cmath` 教程/registry verified |
| 历史边界 | 721 条 F10、22C/22D、19/11/22 不核销 |

B2 是 `lfloat_divide` 的**公开单入口实现**，不是 B2-B 整批完成。B3 才接 15 项 `cmath`，B5 才切版本，B2-C 才做最终 721 与公共发布。

### 1.1 要做

1. 在 `intrinsics.json` 增加 ID 33：四参、返回 `lfloat`、`Allocates`、无宿主能力；
2. 类型检查/IR 接受四参全局函数，四参必填，支持位置/命名，不增加默认值或重载；
3. Runtime 公开入口调用 B1 的 `explicit_division` 内核，先精确商余、再单次最终舍入；
4. 新增 C ABI `xiao_runtime_lfloat_divide`，按 F-5 的借用/out/状态契约实现；
5. 接通 TAC/字节码、VM、LLVM 动态降低和序列化/归档；
6. 覆盖七种 rounding、precision 1～32,768、零分母、短结果、scale/资源与失败原子性；
7. 保存 intrinsic 33、C 符号、VM/LLVM 和两后端精确入口证据；
8. 保持 B1 七库 20 入口、资源 17 项、双精确和 28/28 守卫。

### 1.2 明确不做

- 不激活 intrinsic 34～48，不写 `stdlib/cmath.xiao` 或 `cmath` 教程；
- 不升 ABI 1.19/生成器 21，不改 `.xiaoc`/`.xar`/原生元数据版本；
- 不实现 `sfloat` 版 `lfloat_divide`、默认参数、方法别名或新数值自动转换；
- 不把 `1/3` 显式接口报 034；034 只属于精确语言 `/`；
- 不运行旧 721 条 F10，不把 B0/B1 基线当成 B2 成绩；
- 不提前改 registry、`cmath` 状态或关闭历史债务。

## 二、开工闸门

B2 执行前必须确认：

1. B0/B1 的只读审计在当前仓库通过（B0 18/18、B1 18/18）；
2. B1 代码提交 `35872782` 的 7 个允许路径未漂移，私有内核可复用；
3. 当前 intrinsic 1～32 活动、33～48 不活动、ID 100 墓碑未变；
4. ABI 1.18/生成器 20/标准库接口 1/cmath planned 未变；
5. B1 七库 20 入口、资源 17、双精确和 28/28 守卫可复现；
6. F-1/F-5/F-12 正文与 `explicit_division` 的 precision/rounding 语义一致；
7. 唯一任务根、独立 target/Runtime/缓存已建立，不复用 A7/B1 产物。

## 三、分批执行

### B2-0：入口、快照与公开面基线

1. 固定 B1 代码 `35872782`、B0 入口 `d08b8460`、A7 产品基线 `e7bf7a49`；
2. 复核 intrinsic 33～48 全部不活动，记录当前 32 个活动 ID；
3. 保存 B1 20 入口/17 资源/双精确/28 守卫基线；
4. 记录当前 C ABI 符号表和 VM/LLVM intrinsic 绑定，确认没有 `lfloat_divide`；
5. 建独立证据目录，不覆盖 B0/B1。

退出：入口身份和公开面基线可复现；未写公开入口。

### B2-1：intrinsic 33 声明与类型/IR

1. 在 `core/rust/crates/xiao-intrinsics/intrinsics.json` 增加 ID 33；
2. 源级名 `lfloat_divide`，四参 `lfloat, lfloat, int, str`，返回 `lfloat`；
3. 效果 `Allocates`，能力 `None`；不得标成可删除的纯调用；
4. 类型检查：四参必填、位置/命名均可，不允许默认值、重载、别名或隐式新提升；
5. IR 生成四参调用，保留源级实参顺序和源码位置；
6. TAC/字节码使用既有 intrinsic 编码；不提前改容器版本；
7. 该步只完成声明与类型/IR，不接 Runtime/VM 执行。

退出：声明表自洽，33 与 1～32 不冲突；32 之前入口不变。

### B2-2：Runtime 与 C ABI

1. 新增公开 Runtime 入口，复用 B1 `request`/`divide`，不另写商余/舍入算法；
2. `precision` 只接受 1～32,768；`precision=0`/超界为 012；
3. rounding 只接受冻结七名；未知名称为 012；
4. 分母为零为 013；合法非零分母不因非终止报 034；
5. 输入/工作/结果/scale 按 F-3 检查，超限为 033；输出分配失败为 011；
6. C ABI `xiao_runtime_lfloat_divide(const XiaoValue*, const XiaoValue*, const XiaoValue*, const XiaoValue*, XiaoValue*) -> i32`；
7. 输入借读且强引用覆盖调用；out 必须已初始化、拥有且与输入不重叠；
8. 成功 0 释放并替换旧 out；已知类型/参数/预算失败为状态 6 并保留 out；1/2/3 结构错误保持 out 且不碰挂起槽；
9. 失败无部分结果、输入不变、精确保留挂起错误。

退出：33 号 Runtime/C ABI 在单元测试中覆盖七种舍入与错误顺序；尚无 VM/LLVM 语言路径。

### B2-3：VM、LLVM 与语言承载

1. VM 绑定按 33 的四参签名调用 Runtime；不复制数学实现；
2. LLVM 动态降低生成 C 调用；符号、参数顺序和 out 槽与 F-5 一致；
3. 原生静态路径不新增第二套实现；若当前 lowering 没有对应能力，按既有动态 Runtime 路径接线；
4. 序列化/归档保留 intrinsic 33；不改变现有标签/版本；
5. 错误传播保留 source span、可 catch 009/012/013/033 的既有路径；
6. 不把失败折叠成不可捕获编译错误，不用宿主 `f64` 计算商；
7. 资源释放/finally 路径保持既有契约。

退出：VM/native 对同一四参调用产生相同结果；归档可往返 33。

### B2-4：测试与证据

1. 七种舍入：`2.5/1`、`-2.5/1`、`2.1/1`、`-2.1/1` 的 F-1/F-A 算例；
2. 精确短结果：`1.00/2` p=10 → `0.5`；`1/2` p=32768 → `0.5`；
3. 单次舍入：`9.99/1` p=2 → `1e1`；`3/8` p=2 → `0.38`；
4. 非终止显式结果：`1/3` p=3 → `0.333`，不报 034；
5. precision 0/32,769、未知 rounding、零分母、超预算和 scale 边界；
6. C ABI：空指针、未知标签、类型错配、out 保留、成功替换；
7. VM/native 各一个 `--exact` 入口，恰好 1 passed/0 failed/0 ignored；
8. B1 七库 20/资源 17/双精确/28 守卫回归；不跑 721；
9. 证据保存源码、锁、Runtime、工具链、目标、PID、日志和前后摘要。

退出：33 号入口在 Runtime/C ABI/VM/LLVM 和错误/资源路径有证据；cmath 仍未激活。

### B2-5：内部收口与 B3 交接

1. 复核 intrinsic 最大变为 33，34～48 仍不活动；
2. 复核 ABI 1.18/生成器 20/标准库接口 1/cmath planned/registry 未改；
3. 记录 B2 是内部验证，不发布 1.19/21，不写最终教程；
4. B1 基线仍绿；旧 721 未跑；历史债务未关闭；
5. 形成 B3 入口：15 项 cmath、intrinsic 34～48、`cmath.pow` 与 float `**` 接线。

退出：B2 单入口内部收口；B3 未开始；未把 33 写成 B2-B 整批完成。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 声明 | ID 33、四参签名、Allocates、无能力、1～32 不变 |
| 类型/IR | 位置/命名、四参必填、静态非法类型、源码位置 |
| Runtime | 七种舍入、precision 1/32768/0/32769、零分母、短结果、单次舍入 |
| 错误 | 002/011/012/013/033；不产生 034/009 |
| 原子性 | 失败保留 out、输入不变、无部分结果、挂起错误 |
| C ABI | 5 指针、状态 0/1/2/3/6、out 所有权/不重叠 |
| VM/LLVM | 同结果、同符号、序列化/归档、错误 span |
| 回归 | B1 20/17、双精确、28/28；721 未跑 |
| 边界 | 34～48 未激活、ABI1.18/20、registry/cmath planned |

## 五、退出条件

1. intrinsic 33 活动并与 C 符号、VM/LLVM 绑定一一对应；
2. `lfloat_divide` 七种舍入、precision、scale、错误和资源全部有证据；
3. C ABI 状态 0/1/2/3/6 与 out 保留/替换符合 F-5；
4. VM/native 同四参调用一致，归档可承载 33；
5. B1 20 入口/17 资源/双精确/28 守卫回归通过；旧 721 未运行；
6. intrinsic 34～48 仍不活动；ABI 1.18/生成器 20/标准库接口 1 未变；
7. `stdlib.cmath` 仍 planned，registry 未改；历史债务未核销；
8. B3 入口明确，不能把 B2 单入口写成 B2-B 整批收口。

## 六、风险与禁止

- 在 B2 顺手激活 34～48，导致 B3 边界和 ABI 矩阵混乱；
- 另写一套 f64 商余/舍入，绕开 B1 的精确内核；
- 把 `1/3` 显式调用误报 034，或把零分母/参数域顺序颠倒；
- C ABI 失败时改写 out 或丢失挂起错误；
- 用 `Allocates` 入口当纯调用删除，或把可 catch 错误折叠成编译错误；
- 在 B2 提前升 1.19/21 或生成跨版本产物；
- 用 B0/B1 基线、旧 721 或 A6 性能冒充 B2 成绩。

## 七、交接

B2 完成后进入 B3：15 项 `std.cmath`、intrinsic 34～48、`cmath.pow` 与 float `**` 共用内核。B4 接语言全链路，B5 再升 ABI 1.19/生成器 21，B6 做资源/性能，B7 收口后交 B2-C。

新 Agent 先读：

1. [B2-B 实施规划](20o-b2-b-implementation-plan.md)；
2. [B0/B1 收口](20o-b2-b0-b1-closeout.md)；
3. [已签冻结正文](20o-b2-freeze-draft.md) §五～§九；
4. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
5. [F-12 错误表](20o-b2-freeze-b3-error-table-plan.md)；
6. [F-3 资源/性能](20o-b2-freeze-c-resource-dependency-plan.md)。

## 八、落点

```text
core/rust/crates/xiao-intrinsics/intrinsics.json             intrinsic 33
core/rust/crates/xiao-runtime/src/abi*.rs                    C ABI 入口
core/rust/crates/xiao-runtime/src/value/arithmetic/          explicit_division 复用
core/rust/crates/xiao-types/                                 四参类型
core/rust/crates/xiao-ir/                                    IR 调用
core/rust/crates/xiao-bytecode/                              TAC/字节码
core/rust/crates/xiao-vm/                                    VM 绑定
core/rust/crates/xiao-codegen-llvm/                          LLVM 动态降低
docs/DevDocs/assets/20o-b2-b/entry-<date>-b2/                B2 证据（待建）
```

本页保留B2范围与退出门槛；实际实施和证据见[实施记录](20o-b2-b2-lfloat-divide-progress.md)。B3～B7与B2-C仍后置，Linux/macOS本批运行证据尚未取得。

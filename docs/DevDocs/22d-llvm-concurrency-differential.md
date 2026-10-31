# 22D. LLVM 并发执行、差分与跨平台

> **这是 22 阶段的第四批：让 LLVM 原生后端消费 22B 的并发类型、事实、边界动作和取消检查，
> 并用同一组黄金场景验证 VM 与 LLVM 的输出、错误、退出码和释放记录一致。**
> 22A/22A-FIX 的 API、ABI、Send/Sync 和诊断已冻结；22B 的静态字段和 22C-FIX.1 的 VM 行为是
> 本批的输入，本批不重新决定语言语义。
>
> 状态：**已完成（2026-10-31，经 22D-FIX.1 收尾）**。代码提交 `ab92df1` 的
> Windows/Linux/macOS 原生门禁、Linux workspace 默认测试及完整 workspace gates 全部通过。
> 当前同 IR 差分为 12 个成功、16 个运行时错误和一个嵌套 detached 场景；C-1～C-11
> 按规定验证层次关闭，九条退出条件与最终证据见 §9.3；§9.2 保留前一轮基线。
> 并发 ABI 内部差分沿用 D-2/D-5 的标签 18、ABI 1.17 和不透明锁守卫方案；不写
> `stdlib/concurrent.xiao` 函数体，不冻结原生并发产物元数据，不修改 32B 共享头、API、
> Send/Sync 表或诊断编号。

## 一、入口条件与当前证据

### 1.1 前置

1. [22A. 并发入口冻结](22a-concurrency-freeze.md) §2.2/§2.6/§2.9：语言表面、32B ABI、
   错误、产物门禁；
2. [22A-FIX. 契约冲突收口](22a-fix-concurrency-freeze.md)：Runtime/pool 入口、shared 句柄、
   关闭语义和 `.xar` feature 位；
3. [22B. 并发类型、生命周期与 IR](22b-concurrency-types-lifetime-ir.md)：`IrConcurrencyCall`、
   `IrCrossThreadFact`、`IrBoundaryTransfer`、`IrCancellationCheck` 和 verifier；
4. [22C-FIX.1. 并发 VM 调用与端到端](22c-fix1-concurrency-vm-wiring.md)：已关闭的 VM 行为、
   26 条回归和 `0caebc9` 的 CI 证据；
5. [10. LLVM 原生后端](10-native-backend.md)、[10A. N0 原生闭环](10a-n0-native-closure.md)：
   LLVM 文本、Runtime ABI、错误路径和原生构建驱动器；
6. [12. 测试与开发里程碑](12-tests-and-milestones.md) P0：22D 退出条件。

### 1.2 已关闭的前置

22C-FIX.1 已完成：代码提交 `0caebc9` 的 workspace gates `40709431538` 通过；
maintenance `40709431618` 第 2 次尝试通过，Linux/Windows/macOS 各自运行
`22c_vm_concurrency` 26/26。首次 Windows 20D 进程 fixture 时序失败已在重跑中通过；
22D 不重开 22B 类型/事实语义，也不以 VM 回归代替 LLVM 证据。

### 1.3 原生实现与阶段边界

| 面 | 当前事实 | 对 22D 的影响 |
| --- | --- | --- |
| LLVM 输入 | `xiao-codegen-llvm` 消费同一份 `IrProgram` 的只读 `NativeConcurrencyPlan` | 沿用 `IrConcurrencyCall` 和 `IrOwnership`，不增加前端 |
| 并发调用 | 17 个入口按同源计划拦截；函数/trampoline 绑定 IR 定义节点，根/局部别名均有真实汇编回归 | 结果、错误与清理矩阵见 §9.3；模块体并发仍结构化拒绝 |
| 静态/动态分派 | 只有内部构建显式允许并发计划后才进入动态 ABI lowering；公开原生构建继续结构化拒绝 | 22E 前不得提前开放用户并发产物 |
| ABI 值 | 标签 18、不透明盒子与 32B/16B 布局不变 | 三平台 C 对象探针和 Rust ABI 单测通过，ABI 仍为 1.17 |
| Runtime ABI | 并发调用、trampoline、Runtime/pool/shared/lock/cancel 已接通 | XCON、错误槽、宿主取消及真实 finish 失败按层次登记，Rust 泛型布局未暴露 |
| 线程入口 | 统一函数指针 + 不透明上下文；Runtime 管理线程/任务 | Runtime 背压/关闭与源码 detach/关闭路径已回归，入口等待 detached 回调结束再刷新 trace |
| 错误/取消 | 消费 `safe_point`/`wait`/`lock_wait`，027 不可 catch，清理不重复投递取消 | 四位置、finally/drop、主/suppressed、028–032 与用户栈帧证据见 §9.3 |
| 构建/产物 | `NativeBuild`/`FrontendNativeDriver` 已存在；原生并发产物元数据明确后置到 22E | 22D 只生成内部原生测试产物；不新增 `.xar` 原生 feature 容器或发布验收 |
| 测试 | 三平台精确执行汇编、C 布局对象和同 IR 12/16/1 差分 | 175 项 Runtime、22C 26/26、workspace 与完整 gates 通过；允许的层次及可交换对象均已登记 |

## 二、目标与不负责

### 2.1 目标

1. LLVM 从同一份类型化 IR 生成线程入口、线程/任务池/多 Runtime、共享句柄、锁、边界
   XCON 复制/序列化和取消/清理代码；
2. 原生 Runtime ABI 提供 opaque 句柄和稳定函数入口，不暴露 Rust 对象布局；
3. VM 与 LLVM 对同一并发黄金场景的输出、错误码、退出码、释放记录和主/suppressed
   错误一致；
4. Windows/Linux/macOS 原生构建和并发回归；32B 共享头/句柄布局与 ABI 1.17 的跨平台
   检查；
5. 竞争、取消四位置、finally/drop、detached 失败和 Runtime/pool 关闭的确定性回归；
6. 保持 22C 的 VM 行为和 22B 的静态校验不变；22E 只消费本批稳定下来的原生行为。

### 2.2 不负责

- 不实现 `stdlib/concurrent.xiao` 函数体、UseDocs 发布或用户标准库入口（22E）；
- 不冻结原生并发产物元数据、原生 required-feature 容器或 `.xar`/`.xiaoc` 升级矩阵（22E）；
- 不改 22A/22A-FIX 的 API、32B 共享头、普通 `StrongHandle`/`WeakHandle` 布局、Send/Sync
  表或 027–032 编号；
- 不引入 Actor、`async`/`await`、协程调度器或官方并发容器；
- 不把优化级别、LTO、JIT 或性能目标加入本批（仍归 15/19）；
- 不把 07-D 原型或 Rust 内部 `Arc`/`Mutex` 布局写成产品 ABI。

**产物边界**：22D 生成的 NativeBuild 可执行文件用于内部差分和 CI；用户可见的原生并发
产物验收、元数据和 required-feature 语义在 22E 单独冻结。22D 不宣称原生并发产物已可发布。

## 三、设计裁定点

以下裁定是实施形状，不改变 22A/22B/22C 已冻结的语义。D-2/D-5 已按星崽确认的推荐方案实施；其余内容仍不改变既有语言语义。

### D-1（并发计划来源）：同源 IR，不按名称重解析

**建议取甲**：以 `IrProgram.concurrency_calls`、`IrOwnership.cross_thread_facts`、
`IrBoundaryTransfer`、`IrCancellationCheck` 为唯一输入；`xiao-codegen-llvm` 在内部建立
按 `IrSpan` 和 operation 查找的只读 `NativeConcurrencyCall`。TAC 的
`TacConcurrencyPlan` 继续只服务 VM；不从源码名称、导入名或函数名重新推断操作。

**乙**：把 TAC 计划转换成 LLVM；会让原生后端依赖字节码 lowering，并扩大跨 crate 耦合。
**丙**：在 LLVM 重新扫描调用名；违反 22B 的“不按名称推断”约束，不采用。

验证：同一 IR 的每个并发调用 span 必须唯一对应一个 native 计划；没有计划的调用必须
在 lowering 前结构化拒绝；VM 与 LLVM 的计划 span/参数类型一致。

### D-2（原生并发值表示）：opaque 句柄 + 单一并发值标签

**已确认采用甲**：在 `xiao-runtime-abi` 增加一个不透明并发值表示。追加
`XiaoValueTag::Concurrency`（下一个未使用标签）和一小组 `XiaoConcurrencyKind`
（Runtime/Thread/Task/Pool/Shared/SharedMut/LockGuard）；载荷只保存 opaque box 指针，
所有字段访问和所有权转移都经 C ABI。共享句柄包装继续使用 16B `XiaoSharedHandle`，
32B `XiaoSharedHeader` 不改变。

**乙**：为并发类型使用独立的 `%xiao.concurrency` LLVM 槽，不扩展 `XiaoValueTag`；
可以避免新值标签，但每个动态槽/函数签名/释放计划都要分支，且与现有 `%xiao.value`
容器和表回调不一致。

**丙**：复用 `ExternalResource` 或裸指针；会混淆 tag/所有权和资源类别，不采用。

**确认结果**：新增值标签仅用于 22D 内部原生差分，沿用 ABI 1.17；原生产物 required-feature
和发布元数据仍后置 22E，不把该内部试验路径宣称为已发布 ABI。

### D-3（线程执行）：Runtime trampoline + 函数指针 + 不透明上下文

**建议取甲**：LLVM 为每个 `spawn`/`submit` 入口生成统一 trampoline；Runtime C ABI 接收
`entry` 函数指针、目标 Runtime、XCON 上下文和结果缓冲区。Runtime 负责创建 OS 线程、
安装 Runtime/取消上下文、调用 trampoline、回传结果；LLVM 不直接创建线程，也不把 Rust
闭包或 `Rc` 跨线程传递。

**乙**：LLVM 自行调用 OS 线程/同步原语，Runtime 只提供值操作；会复制关闭协调、取消和
detached 逻辑，不采用。

验证：线程入口 ABI 在三平台一致；子执行单元不共享 VM/LLVM 栈或 Rust `Rc`；函数指针
在 COFF/ELF/Mach-O 上可调用；线程 panic/错误经 Runtime 错误槽回到 join/wait。

### D-4（边界值）：复用 XCON v1 的 C ABI

**建议取甲**：在 `xiao-runtime-abi` 暴露边界编码/解码函数（输入 `XiaoValue` 数组或等价
视图，输出拥有缓冲区），复用 22C 的 XCON v1 限制、深度/节点/大小、逆向 cleanup_order
和 029。原生 lowering 在启动子执行单元前完成编码；子 trampoline 解码后运行；返回值按
同一规则编解码。显式 shared 句柄不塞进 XCON，使用独立并发值通道。

**乙**：LLVM 自己生成序列化字节；会复制格式并导致 VM/LLVM 分歧，不采用。

验证：多参数、深拷贝、16 MiB/64 MiB 边界、恶意 tag 13/14、清理逆序、主/suppressed；
边界失败不启动线程/任务。

### D-5（共享/lock）：32B 头仍为私有实现，C ABI 不暴露字段

**已确认采用甲**：C ABI 只暴露 Runtime/shared 创建、`read`、`lock`、`lock_read`、`lock_write`、
guard 释放和 shared 句柄传递函数；`XiaoSharedHeader` 的 strong/weak/tag/lock_state 仅在
`xiao-runtime` 内部和布局测试中使用。跨 Runtime 只允许带源 Runtime ID 的只读句柄；
`shared_mut` 同 Runtime；守卫离开作用域释放。

**确认结果**：lock guard 返回 opaque guard 指针；所有字段由 Runtime 管理，LLVM 不依赖 Rust
guard 布局。

验证：32B/16B 布局，弱升级，并发读写，锁取消/唤醒，guard 恰好一次释放，跨 Runtime 031。

### D-6（取消、错误、detached）：复用 22C 身份，不新增码

**建议取甲**：原生 C ABI 使用现有错误槽和 027–032；027 不进入 Xiao catch；外部宿主
取消与并发 027 分开；detached 失败进入进程级结构化通道，不改变主退出码；Runtime/pool
关闭顺序按 22C。原生 release trace 记录并发释放事件，差分与 VM 使用同一事件投影。

验证：取消四位置、finally/drop 一次、main/suppressed、detached 失败、Runtime close drain、
跨 Runtime 031、池关闭 030、容量 032。

### D-7（差分口径）：同一 IR、同一组黄金场景

**建议取甲**：建立 `22d_llvm_concurrency` 差分夹具，输入同一份已验证 IR；分别运行
VM（未优化/优化字节码）和 NativeBuild，比较：

- stdout 文本（统一换行后的可观察文本）；
- 错误码/错误类别/退出码；
- 主错误与 suppressed 链；
- 线程/任务/共享/锁的 release trace 投影；
- Runtime/pool/detached 关闭结果。

线程调度和 detached 完成顺序只比较**集合/最终顺序**，不比较 OS 调度时序；可变顺序
必须在夹具中显式声明。VM 和 LLVM 不允许各自新增语义。

### D-8（平台与 ABI）：三平台验证，不单平台冻结

**建议取甲**：Windows、Linux、macOS 都生成并运行内部原生并发夹具；32B 头/16B shared
包装用 C ABI 布局探针和实际 LLVM 产物检查对齐、尺寸、字节序和指针宽度；任一新符号/
标签在 22E 前只作内部 ABI，不进入发布矩阵。32 位目标继续按 22A 以 `X20-TARGET-001`
拒绝。

**乙**：先在 Linux 实现，再把 Windows/macOS 推后；与 22D 退出条件冲突，不采用。

### D-9（失败关闭）：未实现的并发能力必须显式拒绝

在 22D 完成前，原生 lowering 遇到尚未支持的并发调用必须返回结构化 `CodegenError`，
不能生成普通未知调用、静默退化为 VM 路径或丢弃 facts/清理。每个 API 的 native 支持
在差分夹具中逐项标记；没有 native 实现的入口不得被宣称支持。

本批还把 LLVM 文本可汇编性纳入 D-9：导入别名完成登记前不得固化函数表位置，
`spawn`/`submit` 的函数目标找不到已登记定义时必须结构化拒绝；模块体并发尚无独立
计划，必须报告“模块体并发暂不支持”，不能回退为普通动态调用。原生取消检查在实参
求值后执行，与 VM 的参数绑定顺序一致；VM 仍在指令边界检查，native 当前在语句边界
检查，这一粒度差异保留为后续差分基准项。

## 四、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 基线盘点：确认 22C-FIX.1 代码/CI；列出 native 缺失的 ABI/标签/调用/产物路径；当前并发源码必须 fail-closed | workspace/26 条 VM 回归；原生 unsupported 负例 |
| 1 | 建立 `NativeConcurrencyCall` 只读计划：从 `IrConcurrencyCall` + ownership facts/transfers/checks 生成；按 span 查找；不读取源码名称 | IR/TAC 对照；span/参数类型/operation 快照；无计划拒绝 |
| 2 | 扩展 `xiao-runtime-abi`：并发值表示、opaque handle、C 函数声明、状态码/错误槽、ABI 布局测试；不暴露 Rust 布局 | ABI size/align/offset；C/LLVM 布局探针；旧 ABI 拒绝 |
| 3 | 实现 `xiao-runtime` 的 C ABI 适配：Runtime/Thread/Task/Pool/Shared/Lock、XCON 边界、取消/detached；复用 scheduler/concurrency 内部 API | Rust ABI 单测、跨线程压力、错误槽、释放 trace |
| 4 | LLVM lowering：并发调用拦截、参数求值顺序、XCON 编码、结果解码、槽/释放计划、Runtime 创建/关闭 | LLVM 文本快照；最小 spawn/join、runtime_create/close |
| 5 | 线程/任务池/多 Runtime：trampoline、函数指针、上下文、join/wait/detach/cancel/shutdown、父子 Runtime 上下文 | 多 Runtime、pool、detach、关闭 drain；Windows/Linux/macOS 夹具 |
| 6 | shared/lock：shared_read/shared_mut/read/lock/lock_read/lock_write、32B 头、弱引用、跨 Runtime 031 | 共享/锁/取消/泄漏/跨 Runtime 负例 |
| 7 | 边界与清理：多值 XCON、失败逆序、主/suppressed、线程 panic、finally/drop、取消四位置 | 差分 + release trace；恶意 tag 029 |
| 8 | 差分与门禁：VM/LLVM 同一 IR 的多后端比较；加入原生 CI 作业；未实现 API 显式拒绝 | `22d_llvm_concurrency`、workspace、clippy/fmt/docs、三平台原生产物 |
| 9 | 文档收口：更新 22D 状态、22E 入口、落点和证据；不发布 native concurrency artifact metadata | 文档门禁、CI 运行号、README/12/22 索引 |

## 五、测试与验收

### 5.1 黄金场景

至少覆盖：

- `spawn(runtime, fn, a, b, ...)` → `join` 的标量、`str`、递归安全 tuple 和多参数；
- `detach` 成功/失败、detached 结构化错误、主退出码不变；
- `pool_create/submit/wait/shutdown`、队列满背压、关闭 030/031；
- 多 Runtime 并行、Runtime ID、直接 owner 关闭和跨上下文 031；
- `shared_read`/`read`、`shared_mut`/`lock`/`lock_read`/`lock_write`、guard 作用域与释放；
- 取消排队/运行/等待/完成待回收四位置，finally/drop 恰好一次；
- join/wait/lock 的 027、外部取消通道、主/suppressed；
- XCON 深拷贝、大小/深度/节点/恶意 tag 13/14；
- 普通句柄、动态值、资源、可变容器的静态拒绝；
- Runtime/pool close drain、跨线程 detached、线程 panic 028、031/032。

### 5.2 差分口径

同一份 IR/源码分别运行：

1. VM（至少未优化字节码，按现有差分规则加入优化级别）；
2. NativeBuild（内部 LLVM 产物，至少 `-O0` 基线）。

比较 stdout、错误码、错误类别、退出码、主/suppressed、release trace 投影和关闭结果。
调度时序只允许在夹具声明的可交换集合中差异；不得用“结果碰巧相同”代替事件/释放记录
比较。

当前释放投影按首次释放归一化对象 ID，过滤 native `ConcurrencyRelease` 包装盒子事件，
压缩同一对象同一动作的相邻重复；保留不同对象和动作之间的顺序，以及 shared 析构、
弱引用和 guard 释放。运行中取消夹具以锁轮询握手，轮询次数和临时 guard 数量取决于
调度；该夹具比较 027、退出码及 finally 的精确输出，分别确认两侧产生释放记录，
不对其逐事件序列作相等断言。其余五个成功、八个错误和一个 detached 场景逐事件比较
上述释放投影。这验证的是投影后的完整顺序，临时别名引用计数不属于差分相等条件。

### 5.3 反向与门禁

- 未支持的并发入口必须显式拒绝，不能退化成普通调用或静默 VM；
- ABI/32B/tag/句柄/错误身份不得在 22D 改变；
- 普通 `StrongHandle`/`WeakHandle` 仍单线程；
- 原生并发产物不得被误写进 22E 的 required-feature/native artifact 元数据。

### 5.4 平台与 CI

- Windows、Linux、macOS 各自构建/运行原生并发夹具；Windows→Linux→macOS 保持既有
  平台适配顺序；
- 32B 头/C ABI 布局三元组（COFF/ELF/Mach-O）与指针宽度检查；
- 三个平台的 22C VM 26/26 回归不能退化；
- workspace gates、clippy、fmt、docs/usedocs/layout、`git diff --check`；
- 22D 的原生作业必须显式执行，不再只依赖 ignored 过滤或 Windows 时序重试。

### 5.5 22C 审核遗留的承接

- 22C 的 20D Windows fixture 首次失败已由 maintenance 重跑转绿；22D CI 应保留足够的
  超时/诊断，不能把偶发时序失败当成并发代码通过。
- 22C 的公开 `TaskHandle` 等待/断连边界和 `ChildEvents` 容量边界已由回归覆盖；
  22D 原生路径必须调用同一 Runtime ABI，不另写一套取消/释放规则。
- 22C 文档中的 26 条回归、workspace/maintenance 运行号在本计划中作为基线，不重新计为
  22D 的原生证据。

## 六、退出条件

1. 所有 22A 冻结的并发入口（除 22E 标准库函数体）都有 LLVM lowering；未实现入口显式
   fail-closed；
2. VM 与 LLVM 对黄金场景的 stdout、错误身份/类别/退出码、主/suppressed 和 release
   trace 一致；
3. 32B shared header、16B shared handle 和 ABI 1.17 在三平台尺寸/对齐/布局验证，普通
   `StrongHandle`/`WeakHandle` 未变；
4. 取消四位置、finally/drop 一次、lock 唤醒、Runtime/pool drain、detached 失败全部通过；
5. Windows/Linux/macOS 原生并发回归通过；无单平台冻结或“其余平台后补”；
6. 22C 的 26 条 VM 回归、workspace gates 和既有平台门禁无退化；
7. 不写 stdlib 函数体、不新增原生 artifact metadata、不修改冻结 API/ABI/诊断；22E 可以
   只消费稳定行为；
8. 本地 workspace/clippy/fmt/docs/usedocs/layout 与远端 CI 证据齐全；
9. §9.1 的 P3 待收口清单逐项关闭，或在 §九记录显式降级理由与 22E 承接方式。

## 七、风险与禁止

1. **ABI 标签/版本**：并发值标签 18 仅限 22D 内部差分，ABI 1.17 和 22E 发布边界已确认；不得用
   Rust enum 判别式或未登记的 tag。
2. **线程 ABI**：函数指针调用约定、上下文生命周期、线程 panic/错误槽和 Windows COFF
   必须一起验证；不得让 LLVM 直接管理 Runtime 关闭协调。
3. **所有权**：边界 XCON、shared handle、lock guard 和多 Runtime 关闭的所有权必须由
   Runtime 单一实现；不得在 LLVM 生成第二套引用计数。
4. **清理顺序**：finally/drop、主/suppressed、detached 和 Runtime close drain 必须与 VM
   一致；调度不同不能作为顺序差异借口。
5. **平台**：32B/16B 布局和 C ABI 不能由单平台实测冻结；不得用 Windows 成功替代
   Linux/macOS 证据。
6. **范围蔓延**：不接 stdlib 函数体、Actor/async、官方并发容器、原生 artifact 元数据、
   JIT/PGO/LTO/性能目标；这些不属于 22D。
7. **退化**：任何未实现的并发入口必须显式拒绝，不能静默走普通调用、VM 回退或丢弃
   facts/清理。

## 八、落点

```text
docs/DevDocs/22d-llvm-concurrency-differential.md         本计划
docs/DevDocs/22-concurrency-execution-and-shared-handles.md 22D 阶段说明
docs/DevDocs/22a-concurrency-freeze.md                    不可重新选择的 ABI/API
docs/DevDocs/22b-concurrency-types-lifetime-ir.md          IR facts/transfers/checks
docs/DevDocs/22c-fix1-concurrency-vm-wiring.md            已关闭的 VM 行为
docs/DevDocs/12-tests-and-milestones.md                   P0/22D 验收
docs/DevDocs/10-native-backend.md                         LLVM/Runtime ABI
core/rust/crates/xiao-runtime-abi/                        稳定 C ABI 声明与布局
core/rust/crates/xiao-runtime/                            Runtime 实现与 ABI 适配
core/rust/crates/xiao-runtime/src/abi_concurrency_tests.rs XCON、原生错误/释放回归
core/rust/crates/xiao-runtime/src/scheduler_tests.rs       调度、取消四位置与关闭回归
core/rust/crates/xiao-codegen-llvm/                       LLVM lowering/差分
core/rust/crates/xiao-driver/                             内部原生构建/差分驱动
core/rust/crates/xiao-codegen-llvm/tests/                 原生夹具
.github/workflows/                                       三平台原生门禁
```

## 九、实施记录

2026-10-29 已完成第一批原生接线：

- `xiao-codegen-llvm` 新增 `NativeConcurrencyPlan`，从同一份 `IrProgram` 按 `IrSpan` 关联并发调用、跨线程事实、边界传递和取消检查；核对参数/结果类型后绑定入口程序中的实际 IR 节点，模块体的同区间节点不会被误命中；普通项目模块通过语法/类型预检登记模块体并发标记，歧义、缺少节点、类型错配、模块体并发和未知操作结构化拒绝。
- 动态 LLVM 对 `runtime_create/current_runtime/runtime_close`、线程/任务池/shared/lock 等计划调用统一发射 Runtime C ABI；`spawn/submit` 生成函数指针 trampoline，参数按 IR 顺序求值并在成功、失败路径释放临时值。
- `xiao-runtime-abi` 按已确认的 D-2/D-5 追加 `XiaoValueTag::Concurrency = 18`、`XiaoConcurrencyKind`、并发句柄、trampoline 和 detached 错误通道声明；32B shared header、16B shared handle 和 ABI 1.17 保持不变。该路径仍是 22D 内部差分接口，22E 再处理发布元数据。
- `xiao-runtime` 增加并发 ABI 盒子、Runtime/pool、XCON 边界值与显式 shared 通道、read/lock/lock_read/lock_write、取消检查，以及线程/任务 `spawn/join/detach/cancel/submit/wait/shutdown` 的 trampoline 适配；子执行错误包保留错误身份和 suppressed，027 回到父线程后仍不可被 `catch` 捕获，detached 失败由有界进程通道按 Runtime ID 读取。
- `22d_llvm_concurrency` 比较 native 计划与同一 IR 降出的 TAC 操作/span/参数类型/facts/transfers/cancellation；包含未知操作 fail-closed、公开构建门禁、shared/lock 文本回归、根/局部导入别名和分组函数目标的真实 `llvm-as` 解析，以及受控工具链下五个成功输出黄金场景、九个运行时错误场景和一个 detached 失败场景。三平台 CI 显式精确运行 LLVM 汇编与 VM/native 差分用例；最终代码提交 `f4d31b0` 的远端证据见 §9.2。
- 修正了首批盒子直接跨线程传递和通用值释放漏掉并发盒子的缺陷；子执行单元只携带 XCON/显式 shared 信封，在接收线程重新创建 ABI 盒子。取消结果和 detached 错误归一化复用 22C 的 Runtime 内部函数，保持既有 VM 行为。
- 修正了根/局部导入别名插入导致 trampoline 目标函数符号过期、别名函数目标引用未定义 trampoline 的 P1；LLVM 函数与 trampoline 符号现在绑定实际 IR 函数定义节点，函数目标支持括号分组，模块体并发在计划建立阶段结构化拒绝。
- `shared_read`/`shared_mut` 在关闭的当前 Runtime 上返回 031；`spawn`/`submit` 的边界输入 Runtime ID 与返回信封 Runtime ID 分开，`join`/`wait` 按调用方 Runtime 校验 `SharedMut` 结果，与 VM 行为一致。
- 原生释放追踪从线程局部会话扩展为环境变量开启的跨线程会话，trampoline 子线程的参数、结果和临时值释放会汇入同一差分文件；VM 的显式追踪仍保持线程局部语义。
- 外部释放事件同时进入 VM 显式会话和 native 会话；差分将对象 ID 归一化，忽略 native ABI 包装盒子的引用归还和同对象连续临时别名，仅压缩后端复制造成的相邻重复，保留对象间及动作间顺序。VM/native 成功、错误和 detached 夹具现在逐事件比较完整释放顺序；修复了 LLVM 释放计划把 spawn/submit 的复制源误当返回移动而跳过、造成 shared 对象不销毁的问题。
- native 取消检查调整到实参求值之后，Runtime 错误 span 只在子错误没有既有位置时补充，避免覆盖子执行错误的原始位置；native 入口结束时执行当前 Runtime close drain，并按创建时 locale 投影 detached 诊断。
- native Runtime 所有权追踪只在显式入口窗口内生效；入口退出逆序关闭登记的子 Runtime，等待 Runtime-ID detached callback 清零，再投影/消费通道、结束入口窗口并刷新 release trace。
- native finally 路径通过 Runtime 错误 checkpoint 暂存主错误，清理失败恢复为 suppressed；差分用例覆盖该主/次生错误身份。native 函数未处理错误出口按展开顺序追加统一用户栈帧和源码区间；VM/native 嵌套函数错误差分验证帧身份与内到外顺序。native detached 报告与 VM 的错误身份、cause 和 locale 文案投影一致。
- 2026-10-30 修正 finally 中普通函数显式返回清空调用方 checkpoint 的 P1：`xiao_runtime_error_clear` 只清当前错误，独立入口另调 `xiao_runtime_error_checkpoint_reset`；finally 自身的非局部退出仍由 `checkpoint_discard` 处理。Runtime 回归覆盖 clear 后主/次错误恢复和入口遗留栈重置，VM/native 差分覆盖普通 helper 返回及 helper 自身 finally 返回后外层主/次错误保留，并核对错误退出码和清理输出。
- 2026-10-30 在每次 native trampoline 调用前后重置 worker 线程的错误槽与 checkpoint，并把父执行单元的 locale 安装到实际 worker；`native_trampoline_resets_worker_checkpoint_and_inherits_locale` 验证复用线程清理，22D VM/native detached 差分改为覆盖子线程再次 spawn/detach 孙线程。
- 普通项目模块的并发预检现在沿静态导入图向导入方传播并发标记，覆盖经中间模块导出函数的间接路径；`native_project_module_concurrency_is_rejected_structurally` 使用 `main -> helper -> bridge -> std.concurrent` 的真实项目模块验证结构化拒绝。（2026-10-31 审核备注：native lowering 对任一被标记模块即整程序拒绝，桥接模块自身的标记已能触发该拒绝；本条让导入方的标记也完整，属防御性加固，不新增独立的拒绝覆盖面。）
- `xiao_runtime_concurrency_finish` 将 close drain 失败送入统一错误槽，已有主错误保持为主并把 close 失败加入 suppressed；`native_finish_close_error_is_suppressed_under_primary` 验证错误身份与返回状态。（2026-10-31 审核备注：该回归直接调用 `native_finish_status`，未经过完整 close drain；真实入口级的 close 失败差分仍属 §五 差分矩阵的待补项。）
- 旧 ABI 文档统一说明已初始化输出槽、成功替换/失败保留、固定结构输出和借用字节生命周期；`legacy_output_slots_replace_owned_values_and_preserve_on_failure` 与 `legacy_value_wrapper_rejects_wrong_handle_kind_with_none_sentinel` 验证旧句柄/值槽和按值包装边界。
- 将数组、元组、字典、集合和旧表入口迁移到 `xiao-runtime/src/abi_collections.rs`，保留既有 C 符号和 parent module re-export；按物理行门禁统计，`abi.rs` 从 2535 行降至 2055 行，旧尺寸豁免说明已随超限关闭移除。
- opaque 并发 token 使用单调整数 token + Runtime 内部 live registry，retain/release 在 registry 锁内串行，失效 token 不再解引用可复用裸地址；LockGuard 额外校验创建线程，C ABI 并发入口声明补齐参数/输出槽安全契约。
- XCON native trampoline 输入回归注入保留 tag 13/14、超深节点和超大单值；每类都要求结构化 029，且计数证明目标入口未启动。Runtime scheduler 增加屏障控制的 queued/running/waiting/completed-unreclaimed 四位置取消测试，均要求 027 并验证运行/等待边界。
- 公开 `CodegenOptions` 默认拒绝并发原生产物；内部测试使用 `with_internal_concurrency()`。`CODEGEN_VERSION = 12` 隔离 native 栈帧和边界复制源释放的生成文本，Runtime ABI 版本仍为 1.17；native 栈帧函数是运行时内部新增符号，不加入 `xiao-runtime-abi` 公共符号表。
- 首轮 Windows 本机受控验证（`e6fa8c8`）：170 项 Runtime 单测、22D 普通门禁 5 项、真实 `llvm-as` 汇编门禁 1 项，以及五个成功、七个运行时错误、一个 detached 的同 IR VM/native 差分通过。差分比较 stdout、错误主/suppressed、栈帧和归一化的完整释放动作序列。
- 2026-10-31 提交 `e5c83c0` 增加取消与释放顺序回归：22C 排队取消用例用任务独占的两个 IR 值定位转移计划，继续核对 `cleanup_order` 逆序；22D 同 IR 差分新增运行中取消（验证 027、finally 恰好一次）和完成态取消（验证 027）。差分矩阵扩至五个成功、九个运行时错误和一个 detached 场景；运行中取消夹具的释放比较范围见 §5.2。本次受控夹具在本机及三平台远端均通过；早期临时取消驱动记录过 134，当前夹具未复现该现象，不以此宣称定位了当时的崩溃根因。
- 本轮收尾将并发 ABI 与调度器测试迁至 `abi_concurrency_tests.rs` / `scheduler_tests.rs`，保持原测试模块路径和 170 项 Runtime 单测；实现文件分别为 1601 / 1307 行，低于 2500 行限制，并移除调度器尺寸豁免。等待位置的屏障改从 `wait_interruptible` 内部回调发出，确认进入等待路径后才取消。修正 workspace 检查器的正则越界：成员数组之后的合法 profile 通配符不再误报，真实 `*` / `?` 成员仍由回归验证拒绝。布局、检查器 35 项回归、相关 crate 的 Clippy 和 Runtime 单测通过；补齐回调屏障与句柄注册表说明后，文档覆盖率为 90.08%，公共 API 为 100%。
- 2026-10-31 审核 `601963f`：四个新增 Runtime 用例和两个受控原生用例实跑通过，77 个 C 符号在 `abi.rs`/`abi_collections.rs` 拆分前后一一对应且无重复，`abi.rs` 计 2055 个物理行；第 3、4 项按防御性加固和 helper 级证据修正口径，见 §9.1。
- 库测试、workspace/clippy/fmt/docs/usedocs/layout 与本机受控原生执行记录只证明当前实现范围，不代替 §五 的完整差分及 §六 的阶段退出条件。

本轮五项修补的证据见 §9.2；四位置取消的 Runtime 证据与两个 LLVM 取消场景分别登记。
native 入口已增加当前 Runtime close drain、Runtime-ID detached callback 完成屏障、创建时
locale 继承和 native finish 通道消费。并发句柄采用不复用整数 token 的 registry lookup，
稳定入口已明确输出槽契约并拒绝跨线程 LockGuard；其他旧 C ABI 输出槽与包装边界仍需
独立审阅。§五其余黄金场景继续补齐，不改变 22A 冻结的 API、诊断编号、32B/16B 布局
或 22E 产物边界。

### 9.1 P3 待收口清单（并入 22D 收尾）

2026-10-30 审核 `cb9f832` 后登记的六项 P3 均已关闭；对应实现和回归证据已写入本节实施记录：

1. [x] pool worker 在每个 trampoline 前后重置错误槽/checkpoint；`native_trampoline_resets_worker_checkpoint_and_inherits_locale`。
2. [x] 子线程执行时继承创建任务时的 locale；22D VM/native 嵌套 spawn/detach 差分。
3. [x] 并发标记沿项目模块导入图传递（防御性加固：结构化拒绝此前已由桥接模块自身的标记保证）；`native_project_module_concurrency_is_rejected_structurally`。
4. [x] finish close 错误不覆盖既有主错误（helper 级单测；真实入口 close drain 的端到端差分待补）；`native_finish_close_error_is_suppressed_under_primary`。
5. [x] 旧 C ABI 输出槽及按值包装规则已明确并回归；`legacy_output_slots_replace_owned_values_and_preserve_on_failure`、`legacy_value_wrapper_rejects_wrong_handle_kind_with_none_sentinel`。
6. [x] `abi.rs` 已拆分容器/表入口，2055 个物理行低于 2500 行门槛；旧硬耦合豁免文件已移除。

2026-10-31 审核 `601963f` 时复核六项实现均成立，并对第 3、4 项修正口径：第 3 项是标记传播的防御性加固，不是新增的拒绝路径；第 4 项目前是 helper 级回归，真实入口 close drain 的端到端差分仍归 §五 的差分矩阵。两项修正不改变清单的关闭状态。

清单关闭不替代 §六退出条件。XCON 恶意输入、Runtime 取消四位置、native 取消端到端、原生栈帧、稳定场景完整释放顺序及 Windows/Linux/macOS 远端证据均已有本机或对应远端 job 证据；运行中取消的临时锁守卫数量受调度影响，文档已将此差异明确限定，不作为逐事件相等断言。

### 9.2 本轮修补与三平台证据

代码提交：`f4d31b0a71d5775d97e2df13acb985bded30c03c`。以下证据对应此提交，
后续文档提交不计作重新执行代码的证据。

| 修补项 | 回归及结果 | 覆盖范围 |
| --- | --- | --- |
| XCON 恶意输入 | `malformed_xcon_never_enters_native_trampoline` 通过 | native Runtime 接收端注入保留 tag 13/14、超深嵌套、超大单值；结构化 029，入口调用计数保持 0 |
| 取消四位置 | `scheduler::tests::cancellation_at_*` 四项通过；22C 26/26；22D 两个 027 场景通过 | Runtime 的排队、运行、等待、完成待回收屏障回归；LLVM 端到端覆盖运行中 finally 一次和完成态取消；未将 Runtime 四项计为四个 LLVM 差分场景 |
| 原生栈帧 | `native_error_frames_are_appended_during_unwind` 与 `native-stack-frames` 通过 | Runtime 栈帧身份/源码区间；真实 LLVM 产物的 inner → outer 用户帧及同 IR VM 对照 |
| 完整释放顺序 | 五个成功、八个错误和一个 detached 场景逐事件相等 | 比较 §5.2 定义的投影；运行中取消另核对错误/清理和释放记录存在性，轮询临时 guard 不作序列相等断言 |
| 三平台远端 | maintenance [`41176081348`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41176081348) 总结果 `success` | 真实 Runtime 链接、LLVM 汇编和同 IR VM/native 差分；显式 `--exact` 执行并断言恰好 1 条通过，拒绝零匹配 |

| 平台/门禁 | job 证据 | 结果 |
| --- | --- | --- |
| Windows 原生差分 | [124463818334](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41176081348/job/124463818334) | `success` |
| Linux 原生差分 | [124463818388](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41176081348/job/124463818388) | `success` |
| macOS 原生差分 | [124463818308](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41176081348/job/124463818308) | `success` |
| Linux workspace 默认测试 | [124463818217](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41176081348/job/124463818217) | `success` |
| 解析器与产物安全 | [124463818333](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41176081348/job/124463818333) | `success` |
| Windows 产物尺寸基线 | [124463818403](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41176081348/job/124463818403) | `success` |
| 完整 workspace gates | [124463818157](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41176081391/job/124463818157) | `success` |

以上保留首轮五项修补的基线证据。黄金矩阵收尾、真实 finish 故障分支的层次选择和阶段
完成依据见 [22D-FIX.1](22d-fix1-concurrency-differential-closure.md) 及 §9.3；
最终代码的执行证据对应下节 `ab92df1`，不复用基线运行号充当本批结果。

### 9.3 22D-FIX.1 收尾矩阵与证据

在原有 5/9/1 场景上新增七个成功、七个运行时错误场景，当前总计 **12/16/1**。
源码与 C 探针位于 `xiao-driver/tests/concurrency_closure/`，由同一份前端 IR 分别运行
VM 和 NativeBuild。执行日志逐条输出 `22D-CASE`，工作流对汇编、布局及差分三个
ignored 测试逐一 `--exact` 运行，并验证每次恰好通过一条。

| 条目 | 稳定用例/验证 | 层次及判据 |
| --- | --- | --- |
| C-1 | `spawn-join-str-multiargument`、`spawn-join-tuple` | 真实源码；int/str 多参数和纯标量 tuple 往返、stdout、退出码和完整释放投影 |
| C-2 | `detach-success` | 正常自动 drain；两条打印输出按多重集合比较；独立打印临时对象先按对象分离交错，再压缩相邻别名，保留各自动作顺序和对象数量 |
| C-3 | `pool-capacity-zero-032` | 源码 032 身份、recoverable 类别、退出码 3；后续输出不得执行 |
| C-4 | `pool-submit-after-shutdown-030` | 源码 030 身份、退出码 3；目标不得进入，失败路径释放投影一致 |
| C-5 | `multi-runtime-parallel` | 两 Runtime 的 spawn 事件身份不同、两个任务输出和释放投影；结合既有三个 031 负例验证 native 归属检查 |
| C-6 | `lock-guard-scope-release` | 两个函数作用域各释放一个不同 guard，恰好一次；弱升级复用 Runtime `owned_guard_keeps_allocation_alive_and_unlocks_after_alias_drop` |
| C-7 | `native-queued-cancellation`、`native-waiting-cancellation` | 源码 027、不可 catch、finally 一次；稳定共享对象销毁/弱释放序列；Runtime `native_blocked_lock_wait_is_cancelled_by_host_token` 观察实际等待位后取消 |
| C-8 | `native-wait-027`、`native-lock-wait-027` | wait/lock 的 027、finally、自定义 drop 各一次、suppressed 顺序；宿主取消由 `native_external_cancellation_checks_safe_point_wait_and_lock_wait` 验证三个 C ABI 检查点 |
| C-9 | `xcon-oversized-029`、`xcon-bounded-string-success` | 16 MiB + 1 在入口前拒绝、64 KiB 字符串逐字往返；64 层 tuple 的输入/结果字节相等与深度/节点/恶意 tag 拒绝归 native Runtime 单测 |
| C-10 | `entry_close_drain_failure_preserves_primary_and_suppressed` | 仅在测试中登记不匹配 owner，直接调用真实 finish，核对原主错误、既有 suppressed、close 031 的顺序及退出码 3；正常语言表面不能确定构造该错误关系，非源码入口差分 |
| C-11 | `native_concurrency_shared_c_layout_probe` | C11 静态断言生成真实 host 对象，对照 Rust ABI：header 32B/align 8/offset 0,8,16,24,28；handle 16B/align 8/offset 0,8 |

`worker-loop-bounded-stack` 额外核对十万次计数的 stdout `100000\n`、退出码及释放投影。
生成器版本 13 修复边界复制源归还顺序、stack tuple 在动态 ABI 下漏释放，以及循环内
固定 scratch `alloca` 增长导致的 134；API/ABI、Send/Sync 和诊断不变。

`native-wait-027` 另复现了取消展开时 native 漏执行 drop 主体。Runtime drop 回调现在
使用既有错误 checkpoint，清理中的 helper 不重复消费取消；恢复后保留原 027 和清理
错误顺序。源码差分逐字验证 `wait-finally\nwait-drop\n`；对应 Runtime 回归为
`native_cleanup_checkpoint_defers_repeated_cancellation`，没有新增 C 符号或语言入口。

取消握手不比较轮询临时 guard 的全局序列，比较稳定共享对象的最终销毁/弱释放顺序，
并要求守卫释放存在；语言锁等待不能直接观测阻塞时刻，真实阻塞验证由测试专用等待位
观察完成。源码递归 tuple 的优化快照存在 JSON 深度限制，C-9 的 LLVM 夹具按计划选择
字符串；64 层 tuple 归 Runtime 证据，未表述为 LLVM 源码通过。

代码提交：`ab92df1cec63dcefcfefaf928eae739434bd16a9`。本地 workspace 167 个测试组
退出 0，包含 Runtime 175 项、22C 26/26；workspace Clippy、fmt、docs/usedocs/layout、
覆盖率及三个显式原生门禁全部通过。文档覆盖率为 90.09%，公共 API 为 100%。
本批 maintenance [`41192009040`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41192009040)
和完整 workspace gates [`41192009047`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41192009047)
均为 `success`。两者的 head SHA 均为上述代码提交；后续文档补录不计作重新执行代码。

| 平台/门禁 | 实际 job | 结果 |
| --- | --- | --- |
| Windows 原生 | [124510532493](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41192009040/job/124510532493) | `success` |
| Linux 原生 | [124510532568](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41192009040/job/124510532568) | `success` |
| macOS 原生 | [124510532557](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41192009040/job/124510532557) | `success` |
| Linux workspace 默认测试 | [124510532281](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41192009040/job/124510532281) | `success` |
| Windows 产物尺寸基线 | [124510532503](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41192009040/job/124510532503) | `success` |
| 解析器与产物安全 | [124510532522](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41192009040/job/124510532522) | `success` |
| 完整 workspace gates | [124510532811](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41192009047/job/124510532811) | `success` |

| §六退出条目 | 可复核证据与范围 |
| --- | --- |
| 1. 冻结入口与 fail-closed | 12/16/1 源码覆盖全部 17 个并发入口；未知操作、模块体并发、公开 native 构建继续由非 ignored 用例结构化拒绝 |
| 2. VM/LLVM 差分 | 同一 IR、stdout、错误类别/身份/退出码、cause/suppressed 和释放投影；C-2 可交换对象与取消握手的判据已显式登记 |
| 3. 32B/16B/ABI 1.17 | 同源 C 对象探针对照 Rust ABI 尺寸/对齐/偏移；Runtime/ABI crate 单测；ABI 版本与 22A 输入不变 |
| 4. 取消与清理 | 运行/完成态既有用例加 C-7/C-8；Runtime 四位置屏障、真实锁阻塞等待位、自定义 finally/drop 各一次、C-2 入口 drain、C-10 真实 finish 失败分支 |
| 5. 三平台 | 本批三平台原生 job 全部通过，逐一精确运行汇编、C 布局和 29 场景差分；实际链接见上表 |
| 6. 基线不退化 | 22C 26/26、workspace 167 组及 maintenance 既有 native/安全/尺寸门禁 |
| 7. 发布边界 | 本批未改 stdlib 函数体、原生 artifact metadata、冻结 API/ABI/Send/Sync/诊断；生成器 13 只隔离已修复的内部 lowering 文本 |
| 8. 本地与远端门禁 | 本地全量/Clippy/fmt/docs/usedocs/layout/覆盖率通过；本批完整 workspace gates 与 maintenance 均为 success |
| 9. P3 清单与层次 | §9.1 六项原清单保留；C-1～C-11 对应本节矩阵；弱升级、宿主取消、XCON 深度/恶意输入及 finish 故障注入已写明 Runtime 层次与理由 |

C-1～C-11 及 §六九条退出条件按上述层次完成验收，22D 与 22D-FIX.1 关闭。
源码深度限制、宿主取消与 finish 注入的验证范围保留在证据表中；22E 尚未实施。

## 十、状态

- 阶段代号：22D；
- 当前状态：**已完成（2026-10-31）**；代码 `ab92df1` 的 12/16/1 差分、三平台 C 布局对象及完整门禁通过；C-1～C-11 和九条退出条件的验证层次、降级理由与实际运行号见 §9.3；原 P3 六项见 §9.1；
- 当前工作批：22D-FIX.1 已关闭；22E 标准库/原生产物元数据尚未实施；
- 22D 完成后，22E 只消费本批稳定的并发行为和已有 ABI，不反向修改 22D 的跨平台结论。

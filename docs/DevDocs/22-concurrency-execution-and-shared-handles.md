# 22. 并发执行与共享句柄

> **这是 07-D 之后的正式并发实现阶段，22B-FIX.1 静态残余已关闭；22C Runtime/ABI 与 VM 实现已完成，22C-FIX.1 已收口；22D LLVM/差分正在实施。**
> 07-D 已完成 8 项裁定、隔离原型、Linux workspace gates 和四平台测试；22A 已补齐
> 四平台 32B 布局测量并写出语言表面、取消边界、IR 事实、ABI、诊断和发布门禁；22A-FIX
> 已补齐这些契约与既有 `.xar`/`.xiaoc` 格式的冲突。
>
> **阶段一句话**：**把“OS 线程/任务池 + 候选 A + 拆分 CrossThread + 深拷贝默认 +
> 显式共享句柄”落成 VM 与 LLVM 一致的产品能力；不引入 Actor/async/await，也不把原型
> 结构直接当产品 ABI。**
>
> 状态：**22C Runtime/ABI/门禁和 22C-FIX.1 VM 接线已完成（2026-10-28）；22D LLVM/差分已开始实施（2026-10-29），Windows 本机 `spawn/join`、任务池和 shared/lock VM/native stdout 差分已通过**。
> workspace、26 条 VM/源码/产物并发回归、benchmark、research concurrency、Bun、coverage、docs/usedocs、
> 本地安全/产物回归，以及远端多平台 maintenance/CI 均通过；三平台 maintenance 各自执行了 26/26 VM 并发回归。
> 22D 可以开始，但 LLVM 并发 lowering 和原生差分仍需实施；22E 仍后置。

## 一、阶段目标与边界

### 1.1 目标

在 07-D 的研究和 8 项裁定基础上，交付首版并发能力：

1. **执行机制**：采用 **`std.concurrent` 统一模块**；首版含显式 OS 线程 `spawn/join`、
   多边界值、detach、**任务池**和**多 Runtime 并行**（显式 `runtime_create`/`runtime_close`，
   `spawn(runtime, ...)`/`pool_create(runtime, ...)`，Runtime ID 参与语义）；
2. **`Send`/`Sync`**：候选 A 的递归能力推导；标量、文本和递归安全 tuple 接受，当前可变
   容器、动态值和资源值**编译期拒绝**跨线程，普通句柄拒绝，显式共享句柄接受；
3. **`CrossThread`**：拆分为任务边界和线程边界两个生命周期事实，并携带 Runtime ID；
4. **值传递**：跨线程采用**深拷贝 + 版本头/大端长度/类型标签的二进制序列化**；冻结共享
   继续研究；
5. **取消清理**：采用 07-D §10.4 顺序，**首版实现真实持锁等待/唤醒路径**和显式
   `cancel(handle)`；
6. **原子共享句柄**：只读 + 共享可变；采用**统一 32B 头 + 内联原子状态字/平台等待锁**；
   普通 `StrongHandle`/`WeakHandle` 不隐式跨线程；
7. **官方库**：首版 `std.concurrent` 包含线程/共享入口和任务池；链表、树、观察者、图在
   模型冻结后评估。

### 1.2 非目标

- 不实现 Actor/Channel 语言表面、`async`/`await`、协程调度器；
- 不修改普通 `StrongHandle`/`WeakHandle` 的单线程布局；
- 不把 07-D 原型结构体、std `Arc`/`Mutex` 或原型尺寸直接写成产品 ABI；
- 不做官方并发容器、JIT 或 Actor/async；跨 Runtime 只做 22A 冻结的多 Runtime 边界，不扩成
  堆岛生态；
- 不跳过 22A 直接写语法、IR 或 Runtime 代码。

## 二、入口条件与当前证据

| 条件 | 当前状态 | 备注 |
| --- | --- | --- |
| B0/N0 稳定消费者 | 已有字节码 VM 和 LLVM 原生闭环 | 仍需 22D 的并发黄金场景 |
| 8 项裁定 | 已写入 `00-decisions.md` | OS 线程/任务池、候选 A、拆分事实、深拷贝、锁/取消、原子句柄、官方库后置 |
| 07-D 原型 CI | `40057462458`（99c3047）success；9 原型测试 | 最新 Linux workspace gates；`39963374595` 作为前一轮代码证据 |
| 07-D 平台测试 | `39965322134`（a8aaa4b）四平台 success | macOS/Windows/Linux 测试通过 |
| 跨平台测量 | `40068582635` 记录 24B 原型，`40093591873` 记录四平台 32B 头布局 | 成本仍是研究输入，布局已作为 22A ABI 输入冻结 |
| 取消等待 | 22A 已裁定真实持锁等待/唤醒进入首版 | 22C/22C-FIX.1 实现并测量 |
| 语言表面 | 22B-FIX.1 已接入参数化入口、作用域/提前退出消费、错误出口和 shared 构造能力 | Runtime 标准库仍待 22E |
| IR/诊断/ABI | `spawn`/`submit` 已产生 facts/transfers/cancellation；`source_runtime_id` 已登记为可选字段 | 22C-FIX.1 已按字段接入 TAC/VM 计划；ABI/产物门禁保持冻结 |
| 发布/兼容矩阵 | 22A 已冻结 required-feature 位集和 ABI 1.17 | 22E 编写实际产物检查与报告 |

## 三、阶段分解

### 22A：冻结与缺口关闭（先做，不写产品代码）

详细冻结清单、裁定表、测量任务和交付格式见
[22A. 并发入口冻结](22a-concurrency-freeze.md)。这是本阶段的**第一道门**，只做决定和
测量，不写语言/IR/Runtime 产品代码：

1. **关闭或接受 07-D 缺口**：Linux/macOS 布局/成本测量、真实取消等待路径；
2. **语言表面**：冻结 `std.concurrent` 的线程/共享/任务池入口、`spawn(fn, a, b, ...)`
   多边界值、`join`/detach、返回值和错误边界；
3. **`Send`/`Sync`**：候选 A 的递归推导规则；动态值、资源、普通句柄、显式共享句柄的
   接受/拒绝表；
4. **`CrossThread` 拆分**：任务边界/线程边界的 IR 事实 + Runtime ID、诊断字段和 lowering
   入口；冻结多 Runtime 创建/关闭/边界规则；
5. **值传递**：深拷贝 + **长度前缀二进制**序列化的格式/版本/字节序/失败清理、主/
   suppressed 错误；冻结共享后置；
6. **原子共享句柄 ABI**：统一 32B 头（strong/weak/tag/lock state 各 8B）+ 内联锁的精确
   布局、对齐、弱升级、对象头回收和 ABI 版本，并补 32B 布局跨平台测量；
7. **取消/清理**：07-D §10.4 顺序在 VM/LLVM 的共同验收口径，真实持锁等待/唤醒路径和
   显式 `cancel(handle)`；
8. **诊断身份**：沿用 X03-TYPE/X06-LIFETIME/X06-RUNTIME，冻结具体编号；
9. **标准库/发布**：`std.concurrent` 线程/共享/任务池 API、版本化序列化、ABI/产物兼容
   矩阵和 UseDocs 入口。

**22A 退出条件**：上述九项均有冻结记录，24B/32B 测量证据已归档，取消等待已明确由
22C/22C-FIX.1 实现；22A 不写产品代码，22B 不再重新决定语义。

### 22A-FIX：契约冲突收口（已完成，不写产品代码）

- `.xar` 保留 20E bit 0，线程/共享/任务池/多 Runtime/序列化/取消位从 bit 1 开始并做子集
  检查；`.xiaoc` 使用独立位表和既有 `XIAOC-*` 错误；原生并发产物后置到 22E；
- 多 Runtime 使用 `current_runtime()` 便捷别名，以及显式 `spawn(runtime, ...)`、
  `pool_create(runtime, capacity)`；runtime/pool 是可复制共享别名，关闭幂等，Runtime 关闭
  优先于池关闭返回 031；共享可变锁守卫提供 `lock_read`/`lock_write`；
- Send/Sync 类型表、`TaskBoundary`/`ThreadBoundary` 事实与 IR 字段、030/031/032 分工、
  join/wait 原错误传播、`.xar` 未知位扩展 `X17-XAR-011` 和 shared 句柄包装 Runtime ID
  均已固定；详见 [22A 修正契约](22a-concurrency-freeze.md#29-22a-fix-修正契约2026-10-24八项均取甲)。

### 22B：类型、生命周期与 IR（静态接线已完成）

规划、现状核对和 D-1～D-8 裁定（D-1 乙、其余甲）见
[22B. 并发类型、生命周期与 IR](22b-concurrency-types-lifetime-ir.md)。

- `xiao-types`/`xiao-lifetime`：候选 A 能力推导、`TaskBoundary`/`ThreadBoundary` 事实 + Runtime ID、
  普通句柄拒绝、多边界值；
- IR：新增事实、所有权/释放动作、深拷贝/序列化/取消边界、verifier；
- 规格测试：合法/非法边界、动态值、资源、跨线程普通句柄、多值打包/解包；
- 不接 VM/LLVM 执行；前端、生命周期和 IR 快照/verifier 已稳定，源码级回归已覆盖。

实施记录和验证见 [22B](22b-concurrency-types-lifetime-ir.md) §十一 及
[22B-FIX](22b-fix-concurrency-wiring.md) 与
[22B-FIX.1](22b-fix1-concurrency-wiring-residuals.md)；FIX.1 的 workspace gates `40212263928`
和 maintenance regression `40212188604` 已通过，不可达 IR facts 修复 `d02c1e6` 又由
`40219553312`/`40219553285` 验证，22C 可以只消费本批静态字段。

### 22C：Runtime ABI、VM 与错误

规划、现状核对和 D-1～D-6 设计点见
[22C. 并发 Runtime ABI、VM 与错误](22c-concurrency-runtime-abi-vm.md)。

- 统一 32B 原子共享句柄头、内联锁、tag、强/弱计数、弱升级、恰好一次析构；
- Runtime ABI 版本升级；普通句柄布局不变；
- Runtime ID 静态标签到实际 ID 映射；
- `.xiaoc`/`.xar` ABI 1.17/feature 门禁；
- 回归：泄漏、双重析构、弱升级竞态、错误顺序、取消四位置；
- **VM `spawn/join`、多边界值、detach 错误通道、深拷贝 + 序列化、真实取消等待、
  清理和 suppressed 错误转入 [22C-FIX.1](22c-fix1-concurrency-vm-wiring.md)。**

### 22C-FIX.1：VM 并发调用与端到端

实施与验收见 [22C-FIX.1. 并发 VM 调用接线与端到端执行](22c-fix1-concurrency-vm-wiring.md)。

- 把 22B facts/transfers/cancellation 转成 VM 可消费的 TAC 并发计划，不按源码名称重解析；
- `RuntimeValue` 增加 Runtime/Thread/Task/Pool/Shared/SharedMut/LockGuard 句柄；
- 每个线程/任务使用独立 VM 实例，边界值深拷贝/XCON 往返；
- 接通 `spawn/join/detach/cancel`、`submit/wait/shutdown`、shared/lock 和 Runtime 关闭；
- 取消安全点、027/028/029/030/031/032、主/suppressed 和 finally/drop 回归；
- 仍不接 LLVM、不写标准库函数体、不改 22A/22B 冻结契约。

**22C-FIX.1 关闭条件已满足**：真实源码经 `xiao run` 覆盖线程、任务池、多 Runtime、shared、
 detach 和取消；26 条 VM/驱动回归、本地 workspace gates、benchmark/research/Bun/coverage、安全回归，
以及远端多平台 maintenance 与 CI 结果均齐全；三平台各运行 26/26 VM 并发回归，现进入 22D。

### 22D：LLVM、差分与跨平台

规划与裁定点见 [22D. LLVM 并发执行、差分与跨平台](22d-llvm-concurrency-differential.md)。

- LLVM 生成同步的线程入口、共享句柄、多值深拷贝/序列化、取消；
- VM/LLVM 同一黄金场景的输出、错误、退出码和释放记录一致；
- Windows/Linux/macOS 跨平台回归；32B 头布局/对齐/版本矩阵；
- 竞争/压力/释放顺序测试；不能用单平台结果冻结 ABI。
- 22D 只生成内部原生差分产物；原生并发产物元数据和标准库函数体仍后置到 22E。

### 22E：标准库、UseDocs 与发布收口

- 首版 `std.concurrent` 线程/共享/任务池入口（22A 冻结的名称）；
- 版本化序列化的公开边界与兼容说明；
- UseDocs 教程、错误和兼容说明；
- `.xiaoc`/`.xar` 的版本拒绝与升级矩阵；原生并发产物待 22E 的元数据契约；
- 官方容器仍后置；最终 ABI/兼容证据与发布报告。

## 四、测试族与验收

### 4.1 测试族

1. **规格/类型**：合法与非法并发边界、动态值、资源、普通句柄、多边界值、detach、任务池、
   多 Runtime 边界；
2. **生命周期/IR**：任务/线程边界 + Runtime ID、释放计划、IR 快照、verifier；
3. **VM/LLVM 差分**：spawn/join、多值深拷贝/序列化、共享句柄、取消、错误/退出码；
4. **原子句柄**：32B 头、强弱计数、弱升级、对象头回收、内联锁、毒化、恰好一次析构；
5. **取消/清理**：真实等待/唤醒、四位置、主/suppressed 错误、finally/drop；
6. **跨平台**：Windows/Linux/macOS 同场景；
7. **兼容/发布**：序列化版本、ABI 版本、`.xiaoc`/`.xar`/原生拒绝与升级、归档大小/安全。

### 4.2 阶段退出条件

1. 22A 五轮裁定和 §2.2 细化冻结齐全；32B 头布局已补跨平台测量；
2. 类型/生命周期/IR 对普通句柄、共享句柄、多边界值和 Runtime ID 有正/负例；
3. VM 与 LLVM 对同一并发黄金场景输出、错误、退出码和释放记录一致；
4. 32B 原子共享句柄 ABI（含内联锁）冻结，跨平台布局/对齐/版本有实测；
5. 取消/清理四位置、真实等待/唤醒和主/suppressed 顺序通过；
6. 普通 `StrongHandle`/`WeakHandle` 的单线程边界没有被放宽；
7. `std.concurrent` 线程/共享/任务池、序列化版本、UseDocs 和兼容矩阵登记；
8. 维护回归、workspace gates、平台复现和发布检查全绿。

其中第 3、5 条的 VM 部分由 22C-FIX.1 先关闭；22D 只消费其稳定行为，不能反向补 VM 生命周期。

## 五、风险与约束

1. **ABI 测量边界**：22A 已用四平台 32B 临时探针确认布局；操作成本和产品 Runtime 行为
   仍不能由该探针推断，必须由 22C-FIX.1/22D 的实现与回归证明。
2. **取消等待未实测**：07-D 只测了标记检查；22A 已决定首版实现真实等待/唤醒路径，
   22C-FIX.1 必须补延迟、吞吐和清理顺序。
3. **共享可变成本高**：锁、毒化、取消、对象头/ABI 必须一起设计，不能只加一个
   `Mutex` 包装。
4. **普通句柄边界**：候选 A 拒绝普通句柄跨线程；不得用名称启发式或运行时猜测绕过。
5. **VM/LLVM 分歧**：并发和释放顺序必须在两个后端同一黄金场景；不能只实现 VM。
6. **范围蔓延**：任务池、Actor、序列化、官方容器都后置；首版先做 OS 线程最小闭环。
7. **诊断身份**：22A-FIX 已冻结新错误码；22B–22E 只能使用固定映射，不能临时编号。
8. **发布兼容**：ABI 版本、`.xiaoc`/`.xar`/原生产物的拒绝/升级策略必须同时更新。

## 六、第一批 22A 的交接摘要（已完成）

22A 只做冻结和缺口关闭，建议按以下顺序：

1. 已复测 Linux amd64/arm64、macOS arm64 的 24B 原型布局/成本和 32B 头布局；
2. 已在 `00-decisions.md`/22A 冻结记录中固定语言表面、`Send`/`Sync`、`CrossThread`、值传递、
   原子句柄 ABI、诊断身份、标准库清单和发布门禁；
3. 更新 [12](12-tests-and-milestones.md) 的 P0 里程碑、测试族和发布/兼容矩阵；
4. 22A-FIX 已完成；下一步进入 22B，先实现类型、生命周期和 IR，不提前接 Runtime。

## 七、落点

```text
docs/DevDocs/22-concurrency-execution-and-shared-handles.md  本阶段
docs/DevDocs/22d-llvm-concurrency-differential.md           22D LLVM/差分计划
docs/DevDocs/22c-fix1-concurrency-vm-wiring.md              已关闭的 VM 收口
docs/DevDocs/12-tests-and-milestones.md                     P0 里程碑
docs/DevDocs/00-decisions.md                                22 阶段登记与冻结决定
docs/DevDocs/README.md                                      22 索引
docs/DevDocs/07d-concurrency-model-research.md              研究输入
docs/DevDocs/21a-rc-cycles-and-concurrency-handoff.md       入口边界
tests/research/concurrency/                                 07-D 原型（只作输入）
```

## 八、状态

- 阶段代号：22；
- 当前状态：22C Runtime/ABI/产物门禁及 22C-FIX.1 VM 实现已完成，本地 workspace、benchmark、research、Bun、coverage、安全/产物 gates 和远端 maintenance/CI 均通过；22B/22B-FIX/FIX.1 静态部分已关闭；22D LLVM/差分正在实施，已接入同源计划、并发 ABI 和 trampoline，差分/三平台门禁仍待完成；标准库函数体未实现；
- 当前工作批：22D LLVM、差分与跨平台（不回补 22B 静态语义，不在本阶段实现 stdlib 函数体，不冻结原生产物元数据）；
- 22C-FIX.1 可以只消费 22B 的类型、事实、动作、verifier 和 22C 已落地的 Runtime API；
  22D/22E 仍不得把 07-D 原型结构写成 ABI 或绕过冻结记录。
- 22D 的详细入口、D-1～D-9、测试矩阵、退出条件和落点见
  [LLVM 并发执行、差分与跨平台](22d-llvm-concurrency-differential.md)。

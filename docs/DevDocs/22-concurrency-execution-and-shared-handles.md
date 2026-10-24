# 22. 并发执行与共享句柄

> **这是 07-D 之后的正式并发实现阶段，22A-FIX 已完成，下一道门是 22B。**
> 07-D 已完成 8 项裁定、隔离原型、Linux workspace gates 和四平台测试；22A 已补齐
> 四平台 32B 布局测量并写出语言表面、取消边界、IR 事实、ABI、诊断和发布门禁；22A-FIX
> 已补齐这些契约与既有 `.xar`/`.xiaoc` 格式的冲突。
>
> **阶段一句话**：**把“OS 线程/任务池 + 候选 A + 拆分 CrossThread + 深拷贝默认 +
> 显式共享句柄”落成 VM 与 LLVM 一致的产品能力；不引入 Actor/async/await，也不把原型
> 结构直接当产品 ABI。**
>
> 状态：**规划中（22A-FIX 已完成，2026-10-24）**。尚未写产品代码；阶段代号 22 已登记，
> 22B 只能按 [22A-FIX](22a-fix-concurrency-freeze.md) 收口后的
> [22A 冻结记录](22a-concurrency-freeze.md) 和 21A §3.2 / 07-D §11.3 的入口条件实现。

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
| 取消等待 | 22A 已裁定真实持锁等待/唤醒进入首版 | 22C 实现并测量 |
| 语言表面 | 已由 22A 冻结 | 使用 `std.concurrent` 和 §2.6 的完整签名 |
| IR/诊断/ABI | 已由 22A 冻结 | 22B/22C 按冻结字段实现 |
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

**22A 退出条件**：上述九项均有冻结记录，24B/32B 测量证据已归档，取消等待已明确由 22C
实现；22A 不写产品代码，22B 不再重新决定语义。

### 22A-FIX：契约冲突收口（已完成，不写产品代码）

- `.xar` 保留 20E bit 0，线程/共享/任务池/多 Runtime/序列化/取消位从 bit 1 开始并做子集
  检查；`.xiaoc` 使用独立位表和既有 `XIAOC-*` 错误；原生并发产物后置到 22E；
- 多 Runtime 使用 `current_runtime()` 便捷别名，以及显式 `spawn(runtime, ...)`、
  `pool_create(runtime, capacity)`；runtime/pool 是可复制共享别名，关闭幂等，Runtime 关闭
  优先于池关闭返回 031；共享可变锁守卫提供 `lock_read`/`lock_write`；
- Send/Sync 类型表、`TaskBoundary`/`ThreadBoundary` 事实与 IR 字段、030/031/032 分工、
  join/wait 原错误传播、`.xar` 未知位扩展 `X17-XAR-011` 和 shared 句柄包装 Runtime ID
  均已固定；详见 [22A 修正契约](22a-concurrency-freeze.md#29-22a-fix-修正契约2026-10-24八项均取甲)。

### 22B：类型、生命周期与 IR

- `xiao-types`/`xiao-lifetime`：候选 A 能力推导、`TaskBoundary`/`ThreadBoundary` 事实 + Runtime ID、
  普通句柄拒绝、多边界值；
- IR：新增事实、所有权/释放动作、深拷贝/序列化/取消边界、verifier；
- 规格测试：合法/非法边界、动态值、资源、跨线程普通句柄、多值打包/解包；
- 不接 VM/LLVM 执行；先让前端和 IR 快照稳定。

### 22C：Runtime ABI、VM 与错误

- 统一 32B 原子共享句柄头、内联锁、tag、强/弱计数、弱升级、恰好一次析构；
- VM 线程 `spawn/join`、多边界值、detach 错误通道、深拷贝 + 序列化、真实取消等待、
  清理和 suppressed 错误；
- Runtime ABI 版本升级；普通句柄布局不变；
- 回归：泄漏、双重析构、弱升级竞态、错误顺序、取消四位置。

### 22D：LLVM、差分与跨平台

- LLVM 生成同步的线程入口、共享句柄、多值深拷贝/序列化、取消；
- VM/LLVM 同一黄金场景的输出、错误、退出码和释放记录一致；
- Windows/Linux/macOS 跨平台回归；32B 头布局/对齐/版本矩阵；
- 竞争/压力/释放顺序测试；不能用单平台结果冻结 ABI。

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

## 五、风险与约束

1. **ABI 测量边界**：22A 已用四平台 32B 临时探针确认布局；操作成本和产品 Runtime 行为
   仍不能由该探针推断，必须由 22C/22D 的实现与回归证明。
2. **取消等待未实测**：07-D 只测了标记检查；22A 已决定首版实现真实等待/唤醒路径，22C
   必须补延迟、吞吐和清理顺序。
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
docs/DevDocs/12-tests-and-milestones.md                     P0 里程碑
docs/DevDocs/00-decisions.md                                22 阶段登记与冻结决定
docs/DevDocs/README.md                                      22 索引
docs/DevDocs/07d-concurrency-model-research.md              研究输入
docs/DevDocs/21a-rc-cycles-and-concurrency-handoff.md       入口边界
tests/research/concurrency/                                 07-D 原型（只作输入）
```

## 八、状态

- 阶段代号：22；
- 当前状态：规划中，未写产品代码；22A 五轮裁定、32B 四平台测量和 22A-FIX 修正契约已完成；
- 下一道门：22B 类型、生命周期与 IR 实现；
- 22B 可以创建类型/生命周期/IR 产品实现；Runtime、LLVM 和标准库仍须等待
  对应子批次，且不得把 07-D 原型结构写成 ABI 或绕过冻结记录。

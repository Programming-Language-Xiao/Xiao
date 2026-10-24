# 22A. 并发入口冻结

> **这是 22 阶段的第一批：冻结与缺口关闭，不写产品代码。** 07-D 已完成 8 项裁定、
> 隔离原型、Linux workspace gates 和四平台测试；本批已完成跨平台测量、语言表面、IR 事实、
> 原子句柄 ABI 和诊断身份冻结，真实取消等待的代码实现交由 22C。
>
> **本批一句话**：**先写出并发入口的语义、ABI、诊断和测量记录，再由 22A-FIX 收口冲突，
> 然后让 22B 写类型/IR；本批不创建语法 AST、不写 Runtime、不改产品 ABI。**
>
> 状态：**22A-FIX 已完成（2026-10-24，含补丁）**。24B 原型输入由 `40068582635` 记录，32B 头的
> 四平台临时探针由 `40093591873` 记录；五轮裁定和 F-1..F-8 修正已写入本文及
> `00-decisions.md`。本批只做测量、决定和文档，不写产品代码；22B 现在可以按修正后的冻结
> 记录进入类型、生命周期与 IR 实施。

## 一、交接上下文

### 1.1 必读

1. [22. 并发执行与共享句柄](22-concurrency-execution-and-shared-handles.md)
   —— 阶段目标、22A–22E 分解和退出条件；
2. [07-D.13. 并发原型最终收口](07d13-concurrency-prototype-final-closeout.md)
   —— 原型证据、跨平台测试和已知缺口；
3. [07-D. 并发模型研究](07d-concurrency-model-research.md) §10–§11
   —— 候选、取消顺序和入口条件；
4. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   §3.2/§五——普通句柄和跨线程边界；
5. [00. 决策基线](00-decisions.md) §07-D 并发模型裁定与 22 阶段登记；
6. [12. 测试与开发里程碑](12-tests-and-milestones.md) P0——阶段出口。

### 1.2 当前证据

| 项 | 状态 | 结论 |
| --- | --- | --- |
| 8 项裁定 | 已记录 | OS 线程/任务池、候选 A、拆分事实、深拷贝、锁/取消、原子句柄、官方库后置 |
| 原型测试 | 9 passed；workspace gates `40057462458` success | 测试可运行 |
| 平台测试 | `39965322134` 四平台 success | macOS/Windows/Linux 测试通过 |
| 测量接线 | `40068582635` 记录 24B 原型输入；`40093591873` 上传 32B 临时探针日志 | 两组证据均可复现 |
| 布局/成本 | Windows x86_64、Linux amd64/arm64、macOS arm64 均报告 32B 头/8B 对齐、u64 载荷偏移 32B/对象 40B、align32 载荷偏移 32B/对象 64B；24B 原型成本见 §3.1 | 布局已冻结；成本仍只作研究输入 |
| 首轮裁定 | §2.1 的 12 项及 §2.3–§2.7 五轮细化已记录（2026-10-24） | 契约已完成最终审阅 |
| 取消等待 | 已裁定首版实现真实等待 | 22C 需要落地并测量 |
| 语言表面/IR/ABI/诊断 | 已冻结 | 22B/22C 按本文交接，不重新决定语义 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 跨平台测量 | **已完成** | `40068582635` 取得 24B 原型记录，`40093591873` 取得 32B 头四平台记录 |
| 取消等待决策 | **已完成** | 真实等待路径进首版，22C 实现并测量 |
| 语言表面 | **冻结** | 模块/函数/参数/返回/错误边界，详见 §2.6 和 §2.8 |
| Send/Sync/CrossThread | **冻结** | 候选 A 规则与拆分事实 |
| 值传递/共享句柄 | **冻结** | 深拷贝默认、显式共享句柄、锁语义 |
| 原子句柄 ABI | **冻结** | 32B 布局/对齐/tag/计数/弱升级/锁状态/版本 |
| 诊断/标准库/发布 | **冻结** | 码族、API 清单、UseDocs 入口和 required-feature 兼容矩阵 |
| 产品代码/语法 AST | **不做** | 22B 才开始 |
| 任务池/Actor/async/官方容器 | **不做** | 后置 |

## 二、冻结清单与裁定表

下表是首轮输入；星崽 2026-10-24 的选择见 §2.1，冲突处以 §2.1 为准。§2.2 的细化项已
写出；2026-10-24 复核发现 F-1..F-9，最终以 [22A-FIX](22a-fix-concurrency-freeze.md)
收口后的版本为准。

| # | 项 | 当前建议（非冻结） | 需要裁定 |
| --- | --- | --- | --- |
| 1 | 首版语言表面 | 最小 OS 线程 `spawn/join`；模块名候选 `std.thread`（不以 `std.concurrent` 先行） | 模块名、导入路径、是否允许裸 `spawn` |
| 2 | 入口签名 | `spawn(fn, value) -> thread<T>`；`join(thread<T>) -> T`；单一边界值进/出 | 是否允许无参、闭包捕获、返回值形态 |
| 3 | 线程句柄语义 | 不透明、join 一次、失败回传 join 点、无隐式 detach | detach/重复 join/线程失败策略 |
| 4 | `Send`/`Sync` | 候选 A 递归推导；标量/文本/只读容器接受；普通句柄、可变容器、动态值、资源拒绝；显式共享句柄单独接受 | 动态值运行时拒绝 vs 编译期拒绝的边界 |
| 5 | `CrossThread` | 拆任务边界/线程边界两个事实；同线程挂起只产生任务边界 | 事实名称、IR 字段、诊断字段 |
| 6 | 值传递 | 跨线程默认深拷贝；失败逆序清理、主/suppressed 错误；序列化/冻结后置 | 深拷贝失败的身份与清理错误顺序 |
| 7 | 共享句柄 API | 显式创建只读/共享可变句柄；共享可变只经显式锁；普通句柄不隐式升级 | 构造/读取/锁/释放的公开名称与所有权 |
| 8 | 原子句柄 ABI | 干净对象头强/弱计数 + tag；载荷偏移/对齐；锁位置；弱升级；恰好一次析构；ABI 版本 | 对象头精确布局、tag 值、锁是否内联 |
| 9 | 取消/清理 | 07-D §10.4 顺序；四位置；主/suppressed 错误 | 真实等待/唤醒路径是否进首版 |
| 10 | 诊断身份 | 新码族，不复用既有码；非 Send/Sync、普通句柄跨线程、共享句柄误用、取消各一类 | 码族与具体编号 |
| 11 | 标准库/发布 | 首版只提供冻结的线程/共享入口；官方容器后置 | 模块清单、UseDocs、`.xiaoc`/`.xar`/原生版本拒绝策略 |

### 2.1 已裁定（2026-10-24）

星崽已完成 §二 的首轮选择：

1. **语言表面**：采用 **`std.concurrent` 统一模块**；`std.thread` 不作为首版独立模块。
2. **入口签名**：采用 **多边界值 `spawn(fn, a, b, ...)`**，`join(handle)` 取回结果；
   具体参数/返回语法见 §2.6，数量和大小上限见 §2.2。
3. **线程句柄**：**允许 detach**；detached 线程失败进入**进程级错误通道**，不向不存在的
   join 点重抛，也不静默忽略。
4. **取消等待**：**真实持锁等待/唤醒路径进入首版**，不能只保留标记检查。
5. **Send/Sync**：动态值和资源值**编译期拒绝**跨线程；普通句柄拒绝；显式共享句柄接受。
6. **CrossThread**：采用**任务边界 + 线程边界两个事实，并携带 Runtime ID**。
7. **值传递**：首版采用**深拷贝 + 版本化序列化**；冻结共享继续后置。
8. **共享句柄 API**：采用 **`std.concurrent` 模块函数**，候选名称 `shared_read`/
   `shared_mut`；不放在普通值方法上。
9. **原子句柄 ABI**：采用**统一 32B 头 + 内联锁**；24B 干净头只作为此前原型的输入，
   不是最终 ABI。
10. **诊断身份**：沿用 **`X03-TYPE-*` / `X06-LIFETIME-*` / `X06-RUNTIME-*`**，不新建
    `X22` 码族。
11. **标准库范围**：`std.concurrent` 首版包含最小线程/共享入口，**同时包含任务池**；
    官方并发容器仍后置。
12. **detach 失败**：进入进程级错误通道记录，进程继续；不重抛到 join。

### 2.2 细化已写死（2026-10-24）

以下条目把五轮裁定落实为契约稿；F-1..F-9 的修正由
[22A-FIX](22a-fix-concurrency-freeze.md) 收口后，才是 22B 可以直接消费的最终版本：

- 入口函数最多接收 **64 个边界参数**；参数按书写顺序逐个编码，单值最多 16 MiB，整帧最多
  64 MiB，超过上限使用 `X06-RUNTIME-029`；函数值必须显式声明参数和一个 `T` 返回值，
  无参函数合法，隐式闭包捕获不合法，捕获内容必须显式作为边界参数传入；不提供裸 `spawn`，
  统一从 `std.concurrent` 导入模块函数；`current_runtime()` 仅是当前 Runtime 的便捷别名，
  规范提交必须使用 `spawn(runtime, fn, ...)`；
- 深拷贝在调用点完成，全部成功后才创建执行单元；第 n 个参数失败时按 `n-1 ... 0` 逆序
  清理已完成的副本，主复制错误保留在调用点，清理错误按逆序进入 `suppressed`，执行单元不
  启动且源值不改变；
- `runtime_create` 分配进程内单调递增的 `u64` Runtime ID，`0` 保留为无效值且 ID 在进程
  生命周期内不复用；`runtime_close` 消费句柄、停止该 Runtime 接收新任务并等待其已提交
  任务/线程，执行单元的失败仍由各自 `join`/`wait` 或 detached 错误通道报告；关闭后的
  Runtime 不能再创建执行单元，`runtime` 句柄不能跨 Runtime 传递，误用返回 `X06-RUNTIME-031`；
  `pool_create(runtime, capacity)` 和 `spawn(runtime, fn, ...)` 显式绑定目标 Runtime；
- `runtime`/`pool` 是可按值复制的共享句柄，别名共享关闭状态；`runtime_close`/`shutdown`
  消费调用方的那一份并关闭底层状态，其他别名仍可观察关闭，重复关闭任一别名幂等成功；
  Runtime 已关闭时 `submit`/`shutdown` 优先返回 `X06-RUNTIME-031`，池自身 shutdown 后
  `submit` 返回 `X06-RUNTIME-030`；
- 跨 Runtime 只允许带源 Runtime ID 的 `shared<T>` 只读句柄或版本化序列化值；
  `shared_mut<T>` 跨 Runtime 在编译期拒绝；关闭 Runtime 不使仍存活的只读句柄失效，但该
  句柄不能再派生线程、任务或共享可变句柄；
- `shared_read(T)`/`shared_mut(T)` 消费输入 `T`；`read(shared<T>)` 返回递归值拷贝；
  `lock(shared_mut<T>)` 借用共享句柄，`lock_read(lock_guard<T>)` 返回递归值拷贝，
  `lock_write(lock_guard<T>, T)` 写入并消费新值，守卫离开作用域释放锁；
- resource/dynamic 值的主拒绝点是编译期 `Send`/`Sync` 检查；若恶意或损坏产物携带 tag 13/14，
  反序列化统一返回 `X06-RUNTIME-029`，tag 预留不扩大可传值集合；
- 32B 头的强/弱计数为 checked `u64` 原子计数；弱升级以 `Acquire` 读取并用
  `compare_exchange` 在非零强计数上递增，失败返回空，不允许复活；强/弱最后一次释放分别
  以 `Release` 递减并在归零时执行 `Acquire` 栅栏，载荷和对象头各析构一次；
- `lock_state` 低 4B 为原子 `u32`：`0` 表示空闲、`1` 表示持有，最高位表示存在等待者，
  其余位为零并保留；抢锁使用 Acquire CAS，解锁使用 Release，并以对象头地址作为平台等待
  原语的键；取消令牌不写入对象头，取消时唤醒所有等待者并返回 `X06-RUNTIME-027`；
- `pool_create(runtime, capacity)` 的容量为必填 `1–256`；容量/参数错误使用
  `X06-RUNTIME-032`，关闭 Runtime 使用 `X06-RUNTIME-031`；队列满时通过可取消背压等待；
  `shutdown` 消费池句柄、拒绝新提交并等待已提交任务，重复关闭幂等成功；尚未开始的任务可
  由 `cancel` 移除，运行中任务在安全点退出，`wait` 取得 `X06-RUNTIME-027`；
- `.xar` 保留 bit 0 的 20E 归档要求位，并发位从 bit 1 起；`.xiaoc` 使用独立 bit 0–5 表；
  `.xiaoc` 错误保持 `XIAOC-*`；`.xar` 缺失要求/版本不匹配使用 `X17-XAR-007`，未知
  required-feature/capability 位使用**扩展后的** `X17-XAR-011`；原生并发产物后置到 22E；
  32 位目标因缺少 64 位原子 ABI 在编译期以 `X20-TARGET-001` 拒绝。

### 2.3 第二轮细化裁定（2026-10-24）

在上表首轮方向基础上，星崽进一步确定：

1. **API 命名**：`spawn`/`join`/`detach` + `shared_read`/`shared_mut`；共享访问用
   `read`/`lock` 模块函数；
2. **多值语义**：`spawn(fn, a, b, ...)` 按位置**逐个深拷贝**，不额外包 tuple；
3. **join/detach**：模块函数 `join(handle)`/`detach(handle)`，句柄被消费；
4. **Runtime ID**：首版**支持多 Runtime 并行**；任务/线程事实携带 Runtime ID，不能只做
   日志字段；
5. **序列化**：**长度前缀二进制**格式；
6. **32B 头**：strong 8B + weak 8B + tag 8B + lock state 8B，载荷从 32B 开始；
7. **诊断编号**：在 **X03-TYPE-030+ / X06-LIFETIME-002+ / X06-RUNTIME-027+** 开新号，
   不复用旧码；
8. **任务池**：`submit(pool, fn, a, b, ...) -> task<T>`、`wait(task<T>)`、`shutdown(pool)`；
9. **取消**：显式 `cancel(handle)`；join/wait 返回取消错误，锁等待取消复用同一语义；
10. **ABI/产物**：Runtime ABI **次版本升级 + required-feature 门禁**；普通句柄布局不变。

这些是接口级决定；精确的 Runtime 创建/跨 Runtime 传输、二进制格式头/字节序/资源类、
lock state 的原子/等待实现、诊断具体编号、任务池容量/关闭语义和次版本号已由 §2.2/§2.6
写死。

### 2.4 第三轮细化裁定（2026-10-24）

1. **多 Runtime**：显式 `runtime_create`/`runtime_close`；跨 Runtime 只传序列化边界值或
   带 Runtime ID 的显式共享句柄；
2. **二进制序列化**：版本头 + **大端长度** + 类型标签；资源/动态类按 22A 规则拒绝；
3. **32B lock state**：原子状态字 + 平台等待（futex/condvar 等）；
4. **ABI/产物**：Runtime ABI 次版本 **+1**，产物 required-feature 门禁；
5. **任务池**：固定容量；`shutdown` 拒绝新任务并等待已提交任务；
6. **任务取消**：未开始任务直接移除；运行中任务标记取消，在安全点退出并返回取消错误；
7. **detached 失败**：结构化错误报告写入统一错误/日志通道，进程继续，不改变主退出码；
8. **诊断编号**：按类别连续分配（X03-TYPE-030+、X06-LIFETIME-002+、
   X06-RUNTIME-027+），具体编号在冻结记录中列全。

本轮已写死：`thread<T>`/`task<T>`/runtime/pool 句柄类型名与所有权，`read`/`lock` 的返回
语义（值/句柄/守卫），二进制 magic/版本/tag 表/长度上限，32B tag 值与 ABI 次版本号，
各诊断的具体编号，以及 UseDocs/发布矩阵；最终内容见 §2.6。

### 2.5 第四轮细化裁定（2026-10-24）

1. **共享读取**：`read(h)` 返回当前执行单元的**值拷贝**；`lock(h)` 返回作用域守卫，
   离开作用域释放锁；
2. **句柄所有权**：`join`/`detach`/`wait` 消费句柄；`cancel` **不消费**句柄，取消后仍可
   `join`/`wait` 取回取消错误；
3. **类型名**：`thread<T>`、`task<T>`、`runtime`、`pool`；
4. **32B tag/ABI**：`1=ordinary`、`2=shared_read`、`3=shared_mut`；Runtime ABI **1.17**；
5. **二进制上限**：长度 `u32`、嵌套深度 `64`、总节点 `1,000,000`；超限返回稳定错误；
6. **诊断编号**：采用固定最小映射（见下表）。

### 2.6 冻结契约（最终审阅：2026-10-24）

**API 形状（类型名/所有权按 §2.5）**：

```text
runtime_create() -> runtime
current_runtime() -> runtime
runtime_close(runtime) -> none
spawn(runtime, fn, a, b, ...) -> thread<T>
join(thread<T>) -> T
detach(thread<T>) -> none
cancel(thread<T> | task<T>) -> none
shared_read(T) -> shared<T>
shared_mut(T) -> shared_mut<T>
read(shared<T>) -> T
lock(shared_mut<T>) -> lock_guard<T>
lock_read(lock_guard<T>) -> T
lock_write(lock_guard<T>, T) -> none
pool_create(runtime, capacity) -> pool
submit(pool, fn, a, b, ...) -> task<T>
wait(task<T>) -> T
shutdown(pool) -> none
```

**诊断最小映射**：

| 编号 | 语义 |
| --- | --- |
| `X03-TYPE-030` | 值不满足候选 A 的跨线程能力 |
| `X03-TYPE-031` | 普通句柄不能跨线程 |
| `X03-TYPE-032` | 共享句柄/普通句柄身份误用或隐式升级 |
| `X06-LIFETIME-006` | 任务边界事实冲突/非法传播 |
| `X06-LIFETIME-007` | 线程边界事实冲突/非法传播 |
| `X06-LIFETIME-008` | 跨 Runtime 边界或 Runtime ID 冲突 |
| `X06-RUNTIME-027` | 线程/任务取消 |
| `X06-RUNTIME-028` | detached 线程失败 |
| `X06-RUNTIME-029` | 二进制序列化/反序列化失败 |
| `X06-RUNTIME-030` | 池自身关闭后拒绝 `submit` |
| `X06-RUNTIME-031` | Runtime 已关闭/跨 Runtime 非法使用 |
| `X06-RUNTIME-032` | `pool_create` 容量或参数错误 |

**二进制头/ tag**：12B 头 = magic 4B + u16 版本 + u32 长度 + u8 tag + 1B 保留，全部大端；
magic 固定为 ASCII `XCON`（`58 43 4f 4e`），版本为 `0x0001`，保留字节必须为 `0`。长度是
当前节点载荷长度，不含 12B 头；整帧由根节点连续编码。`0=none`、`1=int`、`2=sint`、
`3=lint`、`4=float`、`5=sfloat`、`6=lfloat`、`7=bool`、`8=str`、`9=array`、`10=tuple`、
`11=set`、`12=dict`。定宽数值和计数使用大端；字符串载荷必须是 UTF-8；容器载荷先写
`u32` 元素数，再按元素节点递归编码；set/dict 以节点编码的字节序排序保证确定性。`13=resource`/
`14=dynamic` 仅作格式预留和防御性运行时拒绝；动态/资源跨线程仍**编译期拒绝**。

**32B 头布局**：strong 8B + weak 8B + tag 8B + lock_state（u32 状态 + 4B 保留）；头对齐
8B；载荷偏移 32B；干净 `u64` 对象 40B；干净 align32 对象 64B；四平台测量见 §3.5。

**任务池**：`pool_create(runtime, capacity)` 的 capacity 必填，范围 1–256；容量或参数错误
返回 `X06-RUNTIME-032`，Runtime 已关闭返回 `X06-RUNTIME-031`；队列满时 `submit` 施加可取消的
背压，不产生新的“队列满”错误。`pool` 是可复制共享句柄；`shutdown` 消费调用方那一份、
拒绝新提交并等待已提交任务，其他别名仍可观察关闭，重复关闭幂等成功；池自身关闭后
`submit` 返回 `X06-RUNTIME-030`，Runtime 已关闭时优先返回 `X06-RUNTIME-031`；运行中任务
通过 `cancel` 标记退出。

**产物兼容矩阵**：

| 产物 | 首版要求 | 旧 Runtime（<1.17） | 新 Runtime（≥1.17） |
| --- | --- | --- | --- |
| `.xiaoc` | ABI `1.17` + 独立 `.xiaoc` required-feature 位集 | 版本/未知位按 `XIAOC-002/003/005/008` 拒绝 | 支持版本且位集为超集时接受 |
| `.xar` | 索引 `runtime_abi_min=max=1.17`、bit 0 归档要求位和并发位集 | 缺要求/版本按 `X17-XAR-007` 拒绝；未知 feature/capability 位按扩展后的 `X17-XAR-011` 拒绝 | 满足索引范围和位集时接受 |
| 原生并发产物 | 22A-FIX 不纳入首版矩阵；元数据段/符号和错误身份留到 22E | 首版不接受并发入口 | 22E 另行冻结后接受 |

未来 ABI 次版本必须显式扩大 `.xar` 范围并更新 required-feature；旧产物不会静默获得新的
并发语义。

**required-feature 位表**：`.xar` 保留 20E 的 `0x0001=archive_requirements`，并发位从
`0x0002=threads`、`0x0004=shared_handles`、`0x0008=task_pool`、`0x0010=multi_runtime`、
`0x0020=serialization_v1`、`0x0040=cancel_wait` 开始；读取器拒绝未知位，且要求位必须是
支持位的子集。`.xiaoc` 使用独立表：`0x0001=threads`、`0x0002=shared_handles`、
`0x0004=task_pool`、`0x0008=multi_runtime`、`0x0010=serialization_v1`、`0x0020=cancel_wait`；
未知位继续使用 `XIAOC-008`。

**共享句柄包装**：`shared<T>` 的句柄包装固定包含 `runtime_id: u64` 与对象指针；Runtime ID
不进入 32B 对象头。`read(shared<T>)` 可从其他 Runtime 读取，关闭源 Runtime 不使只读载荷
失效；`shared_mut<T>` 跨 Runtime 编译期拒绝。

**审阅结论**：本节是原冻结稿；F-1..F-9 的修正以 §2.9 为准。22A-FIX 关闭后，22B 只能
实现收口后的类型、生命周期和 IR 事实，不能重新选择名称、所有权、边界值集合、错误编号、
格式或 ABI。

### 2.7 第五轮细化裁定（2026-10-24）

1. **32B lock state**：低 4B 为 atomic u32 状态字，高 4B 保留；对象头对齐 8B；平台等待
   基于状态字；
2. **二进制头**：`magic 4B + u16 版本 + u32 长度 + u8 tag + 1B 保留`，共 12B，全部大端；
3. **跨 Runtime 共享**：只读 `shared<T>` 可带 Runtime ID 跨 Runtime；`shared_mut<T>` 跨
   Runtime **编译期拒绝**；值只能走序列化边界；
4. **任务池容量**：`pool_create(runtime, capacity)` 的 capacity 必填，范围 1–256；越界返回
   `X06-RUNTIME-032`，关闭 Runtime 返回 `X06-RUNTIME-031`；
5. **诊断（已被 §2.9 覆盖）**：原记录把 `X06-RUNTIME-032` 写成容量/参数/关闭状态；
   最终映射是关闭池 `submit` 用 `030`、Runtime 关闭用 `031`、仅创建参数用 `032`；
6. **共享类型名**：`shared<T>`、`shared_mut<T>`、`lock_guard<T>`；
7. **序列化 tag**：0–12 为正常标量/容器 tag；额外保留 13=resource、14=dynamic，但
   **动态值和资源值仍按跨线程编译期拒绝**，这两个 tag 只作格式预留/防御性运行时拒绝，
   不表示首版可跨边界传递；
8. **UseDocs/兼容**：22E 首版同步线程/共享/任务池/序列化教程、错误说明和 ABI 1.17
   兼容矩阵。

动态/资源的口径已明确：**编译期拒绝是主规则；tag 预留不改变可传范围**。

### 2.8 最终审阅记录（2026-10-24）

按 §2.2 逐项核对后，以下项目均已有唯一落点：

1. `std.concurrent` 是唯一首版导入路径；模块函数的参数数量、所有权、消费关系和失败边界
   由 §2.6 固定，裸 `spawn`、隐式捕获和普通句柄跨线程均不在范围内；
2. 深拷贝、逆序清理、主错误与 `suppressed` 顺序，Runtime ID 分配/关闭和只读共享跨 Runtime
   规则已固定；resource/dynamic 只有格式预留，运行时防御性错误固定为 `X06-RUNTIME-029`；
3. 32B 头字段、对齐、tag、弱升级内存顺序、锁状态字和 ABI `1.17` 已固定；四平台临时探针
   结果见 §3.5；
4. 任务池容量、`shutdown` 等待、排队/运行中取消和 `wait` 错误已固定；诊断编号表不再有
   “下一空号”或“实现时再定”的占位；
5. required-feature 位集、`.xiaoc`/`.xar`/原生产物拒绝时机、UseDocs 入口和 64 位支持范围
   已固定；UseDocs 页面保持 `planned`，直到 22C–22E 的代码、测试和示例完成后才可改为
   `verified`。

本审阅记录的是当时的冻结结论；2026-10-24 的独立复核发现 F-1..F-9 的契约冲突/空缺，
撤回“语义缺口已全部关闭”的结论，统一由 [22A-FIX](22a-fix-concurrency-freeze.md) 收口。
取消等待的实现和测量仍属于 22C 的交付，不构成 22A 的未决语义。

### 2.9 22A-FIX 修正契约（2026-10-24，八项均取甲）

本节覆盖原 §2.2、§2.6、§2.8 的冲突项；原五轮裁定保留作历史记录，最终实现必须以本节为准。

1. **`.xar` 位表**：bit 0 固定为 20E 的 `archive_requirements`；并发位为
   `0x0002=threads`、`0x0004=shared_handles`、`0x0008=task_pool`、`0x0010=multi_runtime`、
   `0x0020=serialization_v1`、`0x0040=cancel_wait`。读取器拒绝未知位，要求位必须是当前
   支持位的子集；缺少 bit 0 或所需并发位使用 `X17-XAR-007`，未知 required-feature/
   capability 位使用扩展后的 `X17-XAR-011`。`.xiaoc` 使用独立的 bit 0–5 表，未知位保持
   `XIAOC-008`。
2. **多 Runtime 入口**：`current_runtime() -> runtime` 提供当前 Runtime；规范提交入口为
   `spawn(runtime, fn, a, b, ...)` 和 `pool_create(runtime, capacity)`，`submit` 通过池归属
   Runtime。`runtime` 是可按值复制的共享句柄；`runtime_close` 消费调用方那一份并关闭
   Runtime ID，其他别名仍可观察关闭，重复关闭幂等成功；关闭后创建/提交/关闭池返回
   `X06-RUNTIME-031`。
3. **锁守卫访问**：`shared_read(T)`/`shared_mut(T)` 消费 `T`；`read(shared<T>) -> T` 做
   递归值拷贝；`lock(shared_mut<T>)` 借用句柄并返回守卫；`lock_read(lock_guard<T>) -> T` 和
   `lock_write(lock_guard<T>, T) -> none` 借用守卫，守卫离开作用域释放锁。
4. **Send/Sync 实际类型表**：`none`、固定宽度数值、`bool`、`str` 接受；由这些值递归组成的
   `tuple` 接受；产品 `array`、`set`、`dict`/字典表/字典列因当前可变且没有只读类型而拒绝；
   普通 `StrongHandle`/`WeakHandle` 拒绝并使用 `X03-TYPE-031`；显式 `shared<T>`/`shared_mut<T>`
   只接受递归能力表中可接受的 `T`，共享可变句柄仅在同一 Runtime 使用；
   `dynamic`/resource 编译期拒绝并保留 tag 13/14 的防御性运行时拒绝。
5. **CrossThread 事实**：事实名固定为 `TaskBoundary`、`ThreadBoundary`；IR 字段固定为
   `runtime_id: u64`、`task_boundary: bool`、`thread_boundary: bool`；诊断字段固定为
   `boundary_kind`、`runtime_id`、`source_span`。同线程挂起只置任务事实，显式线程迁移同时置
   两个事实，诊断继续使用 `X06-LIFETIME-006..008`。
6. **产物错误身份**：`.xiaoc` 保留 `XIAOC-002/003/005/008`；`.xar` 的版本/ABI/缺失要求
   使用 `X17-XAR-007`，未知 required-feature/capability 位使用**扩展后的**
   `X17-XAR-011`；原生并发产物没有首版 feature 容器，后置到 22E 再冻结元数据段/符号和
   拒绝码。
7. **任务池诊断**：`X06-RUNTIME-030` 只表示池自身关闭后拒绝 `submit`；`X06-RUNTIME-032`
   只表示 `pool_create` 容量/参数错误；满队列通过可取消背压处理；`shutdown` 消费调用方
   别名，其他别名可观察关闭，重复关闭幂等成功；Runtime 已关闭时 `submit`/`shutdown`
   优先返回 `X06-RUNTIME-031`；线程体或任务体失败在 `join`/`wait` 重抛原错误，取消使用
   `X06-RUNTIME-027`，detached 失败使用 `X06-RUNTIME-028`。
8. **Runtime ID 位置**：只读 `shared<T>` 句柄包装固定保存 `{ runtime_id: u64, object_ptr }`；
   32B 对象头不增加字段，32B 四平台测量继续有效；`shared_mut<T>` 跨 Runtime 编译期拒绝。

上述八项关闭 22A-FIX 的契约空白；22B 只消费本节定义的名称、字段、能力表和错误边界。

## 三、测量任务

### 3.1 跨平台布局/成本

在每个平台运行：

```bash
cargo run --release --manifest-path tests/research/concurrency/Cargo.toml -- --iterations 1000
```

平台复现工作流 `365649226` 的运行 `40068582635` 已在四个平台的原生回归后执行同一条命令，
并将每个平台的 `*-concurrency-measurement.log` 与原生日志一起上传；平台测试成功不能替代
测量结果。本轮使用 release 构建、1000 次迭代；Rust 版本由工作流固定为 `1.96.0`。

四个平台均报告相同的布局：干净对象头 `24 B`、含探针载荷偏移 `40 B`、干净 `u64` 对象
`32 B`/偏移 `24 B`、干净 align32 对象 `64 B`/偏移 `32 B`、含探针 `u64` 对象 `48 B`、
含探针 align32 对象 `96 B`/偏移 `64 B`。成本为单次 1000 次整数纳秒均值：

| 平台 | 架构 | clone/drop | 升级成功 | 升级失败 | 锁 | 取消检查 | 含探针析构 | 干净 Box 分配/释放 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Windows | x86_64 | 5 | 6 | 3 | 5 | 0* | 194 | 64 |
| Linux | x86_64 | 5 | 6 | 0* | 5 | 0* | 112 | 32 |
| Linux | aarch64 | 13 | 13 | 0* | 11 | 0* | 100 | 11 |
| macOS | aarch64 | 18 | 17 | 0* | 11 | 0* | 66 | 17 |

`0*` 表示整数纳秒计时低于当前测量分辨率，不是零成本；这些数字是单次研究记录，不是
稳定性能基准，也不单独冻结 ABI。

记录：

- 干净 `u64` 对象、干净 align32 对象、含探针对象的 `size_of`/偏移/对齐；
- clone/drop、弱升级成功/失败、锁、取消检查、含探针/干净分配释放；
- OS、架构、Rust 版本、迭代次数和单次结果；当前原型只输出整数均值，不把单次结果误写成方差；
- Linux amd64/arm64、macOS arm64 与 Windows x86_64 的差异。

**约束**：平台测试通过不等于测量完成；不能用 Windows 数字冻结跨平台 ABI。

### 3.2 取消等待

当前 `cancel_check_ns` 只测 `AtomicU8::load`。2026-10-24 已裁定**首版实现真实等待/唤醒
路径**：持锁 → 等待线程阻塞 → 设置取消 → 返回，22C 实现并记录延迟/吞吐；不能在首版继续
只保留标记检查。

### 3.3 ABI 冻结输入

- 四个平台当前均报告干净头 24B、干净 `u64` 对象 32B/偏移 24B；这些是原型测量输入，
  不是已冻结 ABI；
- 首轮裁定选择**统一 32B 头 + 内联锁**；24B 头是历史输入，22A 已重新给出 32B 布局并
  完成跨平台测量，不能把 24B 直接当最终 ABI；
- 原型的含探针布局只作测试模型，不进入 ABI；
- 首版只承诺 64 位目标（Windows x86_64、Linux amd64/arm64、macOS arm64）；32 位目标以
  `X20-TARGET-001` 拒绝并不进入该 ABI；
- ABI 版本、对象头对齐、弱升级和锁的内存顺序在 22A 冻结。

### 3.4 32B 头本地布局测量（2026-10-24）

用仓外临时 Rust 探针（不进入产品/原型 crate）在 Windows x86_64 上测得：

| 项 | 结果 |
| --- | ---: |
| `Header32` 尺寸 / 对齐 | 32B / 8B |
| `u64` 载荷偏移 / 对象尺寸 / 对齐 | 32B / 40B / 8B |
| `align(32)` 载荷偏移 / 对象尺寸 / 对齐 | 32B / 64B / 32B |

探针字段为 `strong u64 + weak u64 + tag u64 + lock_state u32 + reserved u32`，载荷紧随
32B 头，不含测试探针。该次本地结果由 §3.5 的四平台 CI 临时探针复核；本节保留 Windows
x86_64 的最初本地取证，四平台结论只引用 §3.5 的 CI 日志。

### 3.5 32B 头四平台 CI 测量（2026-10-24）

平台复现运行 `40093591873` 在每个平台的临时目录编译同一份 `repr(C)` 探针，并上传
`*-concurrency-32b-layout.log`。四份日志逐字报告相同布局：

| 平台 | 架构 | `Header32` 尺寸 / 对齐 | `u64` 载荷偏移 / 对象尺寸 / 对齐 | `align(32)` 载荷偏移 / 对象尺寸 / 对齐 |
| --- | --- | ---: | ---: | ---: |
| Windows | x86_64 | 32B / 8B | 32B / 40B / 8B | 32B / 64B / 32B |
| Linux | x86_64 | 32B / 8B | 32B / 40B / 8B | 32B / 64B / 32B |
| Linux | aarch64 | 32B / 8B | 32B / 40B / 8B | 32B / 64B / 32B |
| macOS | aarch64 | 32B / 8B | 32B / 40B / 8B | 32B / 64B / 32B |

该探针只验证 22A 选定字段的 C 布局，不验证计数算法、锁竞争或产品 Runtime 行为；24B
原型的操作成本仍按 §3.1 作为研究输入，不能被这组尺寸解读为性能基准。

## 四、冻结记录格式

22A 并发入口冻结（2026-10-24）已写入本文和 `00-decisions.md`；下列条目是原冻结稿，
2026-10-24 复核发现的冲突由 [22A-FIX](22a-fix-concurrency-freeze.md) 修正后再生效：

- **语言表面**：唯一模块 `std.concurrent`；模块函数为 `runtime_create`、`runtime_close`、
  `spawn`、`join`、`detach`、`cancel`、`shared_read`、`shared_mut`、`read`、`lock`、
  `pool_create`、`submit`、`wait`、`shutdown`；`thread<T>`/`task<T>`/`runtime`/`pool` 和
  `shared<T>`/`shared_mut<T>`/`lock_guard<T>` 的所有权见 §2.6；
- **线程句柄**：`join`/`detach` 消费句柄，`cancel` 不消费；失败在 join 点返回，detached 失败
  写入带 Runtime ID 的进程级结构化错误通道并继续进程；
- **Send/Sync**：候选 A 递归接受标量、固定宽度数值、`bool`、`str` 和递归安全 `tuple`；
  当前可变 `array`/`set`/`dict`/字典表/字典列、普通句柄、动态值和资源编译期拒绝；显式
  共享句柄按 tag 接受，禁止普通句柄隐式升级；
- **CrossThread**：任务边界与线程边界两个事实，均携带 Runtime ID；同线程挂起只产生任务
  边界；非法传播使用 `X06-LIFETIME-006..008`；
- **值传递**：逐参数深拷贝，失败逆序清理，主错误优先、清理错误进入 `suppressed`；格式为
  `XCON`/版本 1 的大端长度前缀节点，参数最多 64 个，单值 16 MiB、整帧 64 MiB；
- **共享句柄**：`read(shared<T>)` 返回值拷贝，`lock(shared_mut<T>)` 返回作用域守卫；跨
  Runtime 只允许带 Runtime ID 的只读句柄；
- **原子句柄 ABI**：32B/8B 头，`strong u64 + weak u64 + tag u64 + atomic u32 + reserved`
  4B，载荷偏移 32B；tag `1/2/3` 分别为 ordinary/shared_read/shared_mut；Runtime ABI 1.17；
  弱升级非零 CAS，强/弱各自恰好一次回收；
- **取消**：22C 必须实现真实持锁等待/平台唤醒；取消令牌唤醒等待者并统一返回
  `X06-RUNTIME-027`，清理顺序沿用 07-D §10.4；
- **诊断**：`X03-TYPE-030..032`、`X06-LIFETIME-006..008`、`X06-RUNTIME-027..032` 固定
  映射；二进制防御性失败为 `X06-RUNTIME-029`，产物缺失要求/版本为 `X17-XAR-007`，未知
  feature/capability 位为扩展的 `X17-XAR-011`；
- **标准库/发布**：首版接口和任务池属于 `std.concurrent`；官方并发容器后置；`.xiaoc` 和
  `.xar` 检查 ABI 1.17 与各自的 required-feature 位集，原生并发产物后置到 22E，32 位目标
  拒绝 `X20-TARGET-001`；UseDocs 冻结入口为
  [并发执行与共享句柄](../UseDocs/language/concurrency/README.md)，实现前保持 `planned`；
- **测量**：24B 原型成本见 `40068582635`，32B 头四平台布局见 `40093591873` 的四份
  `*-concurrency-32b-layout.log`；两组数据均不单独冻结性能结论。

**22A-FIX 生效修正**：以上原冻结条目由 §2.9 覆盖；生效 API 使用
`current_runtime`、`spawn(runtime, ...)`、`pool_create(runtime, ...)`、`lock_read`、
`lock_write`；原 §四 的“只读嵌套容器”以 §2.9 的实际类型表为准（数组/集合/字典拒绝），
原生产物以 §2.6/§2.9 的 22E 后置为准；CrossThread 使用 `TaskBoundary`/`ThreadBoundary`
和三个 IR 字段；`.xar`/`.xiaoc` 位表与错误身份按逐产物矩阵执行；030/031/032 分工、
join/wait 原错误传播和 shared 句柄包装 Runtime ID 均按 §2.9 执行。

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 基线核对：22 阶段、07-D 证据、当前 CI/平台运行号 | 文档/运行号 |
| 1 | 补 Linux/macOS 布局/成本测量，并用同一临时探针复测 32B 头 | `40068582635`/`40093591873` 四平台日志 |
| 2 | 完成 §2.1/§2.3/§2.4/§2.5/§2.7 五轮裁定并写死 §2.2 细化项 | §2.6/§2.8 冻结记录 |
| 3 | 冻结诊断编号、标准库/任务池清单、序列化格式、32B ABI 版本 | `00-decisions.md`/本文 |
| 4 | 更新 12 P0、README、UseDocs 入口和发布兼容矩阵 | 文档/UseDocs |
| 5 | 最终审核：docs/UseDocs/layout、无产品代码、四平台测量摘要 | 本地门禁/`40093591873` |

**本批不写产品代码**；不创建 AST、类型规则、IR、Runtime ABI 或标准库实现。

## 六、验收

1. **测量**：`40068582635` 已记录四平台 24B 原型布局/成本，`40093591873` 已记录四平台
   32B 头布局；不以平台测试代替测量；
2. **取消**：已裁定首版实现真实等待/唤醒路径；22C 必须落地并测量；
3. **冻结表**：§2.1/§2.3/§2.4/§2.5/§2.7 五轮选择、§2.9 的 F-1..F-8 修正和 §2.6
   最终契约均已写出；原“没有空白”的结论已由 22A-FIX 修正后重新成立；
4. **ABI**：统一 32B 头 + 内联锁的字段/对齐/tag/计数/弱升级/版本已写，32B 布局的跨平台
   测量已补；Runtime ID 固定在 shared 句柄包装，原型探针和 24B 输入不直接进入 ABI；
5. **诊断/标准库/发布**：X03/X06 编号、`std.concurrent`/任务池/序列化 API、UseDocs 入口和
   `.xar`/`.xiaoc` required-feature 位表均已按 22A-FIX 冻结；
6. **门禁**：docs/UseDocs/layout 通过；本批无产品代码；
7. **交接**：22B 只能按 22A-FIX 后冻结记录实现类型/生命周期/IR，不再重新决定语义。

## 七、不负责

- 不写并发语法 AST、类型检查、IR、Runtime 或 LLVM 实现；
- 不实现 Actor、async/await、冻结共享或官方容器；任务池和版本化序列化只做 22A 冻结
  所需的 API/格式决定；
- 不修改普通 `StrongHandle`/`WeakHandle` 布局；
- 不把原型尺寸/单平台测量写成产品 ABI；
- 不把 `planned` 的 UseDocs 入口写成已经可用的产品承诺；
- 不在 22A 实现任务池、序列化、真实等待或任何并发 Runtime 代码。

## 八、落点与取证

```text
docs/DevDocs/22a-concurrency-freeze.md              本文
docs/DevDocs/22-concurrency-execution-and-shared-handles.md 阶段与 22B 输入
docs/DevDocs/00-decisions.md                        22A 冻结决定
docs/DevDocs/12-tests-and-milestones.md             P0/22A 里程碑
docs/DevDocs/README.md                              22A 索引
tests/research/concurrency/                         07-D 原型测量（只作输入）
```

```bash
# 原型测量（每个平台）
cargo run --release --manifest-path tests/research/concurrency/Cargo.toml -- --iterations 1000

# 门禁
bun run check:docs
bun run check:usedocs
bun run check:layout
git diff --check
```

## 九、风险

1. **把平台测试当测量**：四平台测试只证明测试可跑，不提供布局/成本数据。
2. **过早冻结 ABI**：24B 历史输入、含测试探针布局或 std `Arc` 尺寸不能直接成为产品 ABI；
   只有本文冻结的 32B 干净布局属于 ABI 输入。
3. **语言表面膨胀**：首版冻结 `spawn/join`、共享入口和任务池边界；Actor、async 和官方容器后置。
4. **共享可变成本**：锁、毒化、取消、对象头和 ABI 必须一起冻结；不能只加 `Mutex` 包装。
5. **诊断撞号**：新码族必须与既有 `X01`–`X20`/`X06-RUNTIME-*` 对账，不能临时编号。
6. **真实等待实现风险**：首版已承诺真实持锁等待/唤醒；22C 必须覆盖取消、超时、唤醒和
   清理顺序，不能退回到只测标记检查。

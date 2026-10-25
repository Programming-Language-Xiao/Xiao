# 22A-FIX. 并发入口冻结的契约冲突与收口

> **这是 22A 与 22B 之间的收口批，只修文档和契约，不写产品代码。**
> 2026-10-24 审核 `620d8a2`（32B 四平台测量接线）和 `487ef1c`（22A 最终冻结文档）时，
> 证据链和门禁通过，但冻结契约内部出现若干冲突/空缺；如果直接进入 22B，类型、IR 和
> Runtime 会各自补一套语义。本批把这些口径写死，再开 22B。
>
> 状态：**已完成（2026-10-24，含补丁）**。F-1～F-8 均取甲，F-9 元数据卫生已同步；本补丁
> 再收口 runtime/pool 句柄模型和 `.xar` 未知 feature 位的错误身份。本批只改文档和契约，
> 不改 `core/`、`cli/`、`stdlib/` 或产品 ABI，不创建并发 AST、类型规则、IR、Runtime
> 或 LLVM 实现。

## 一、为什么是这一批

1. **22A 的证据层已经成立**：`40093591873` 在四个平台实际执行了 32B 头临时探针，四份日志
   布局一致；没有产品代码改动，文档门禁和完整测试通过（见 §二）。
2. **22A 的契约层还没有成立**：`.xar` 的 feature 位与 20E 既有位冲突、多 Runtime 没有提交
   入口、`lock_guard` 没有访问载荷的操作、Send/Sync 的“只读容器”没有映射到产品类型、
   `CrossThread` 的事实名/IR 字段缺失、产物错误身份和 030/032 口径互相覆盖。这些问题不修，
   22B 只能临时决定，22A 的“没有空白”验收结论不成立。
3. **现在修比 22B 之后修便宜**：22B 会把这些名字和规则写进类型/生命周期/IR；届时再改要
   同时改实现和回归。本批只动文档，成本最低。

## 二、已确认的证据与边界

| 项 | 结果 | 说明 |
| --- | --- | --- |
| 32B 四平台测量 | `40093591873` success | head 为 `620d8a2`，分支 `codex/22a-32b-measurement` |
| 四平台布局 | 一致 | `Header32` 32B / 对齐 8B；`u64` 载荷偏移 32B / 对象 40B；`align(32)` 偏移 32B / 对象 64B |
| 测量日志 | 已上传 | 四个 `*-concurrency-32b-layout.log`；平台测试没有被当成测量证据 |
| 产品代码 | 未改 | `origin/main..HEAD` 只有 workflow 和文档 |
| 本地门禁 | 通过 | `check:docs`、`check:usedocs`、`check:layout`、`git diff --check` 均为 0 |
| 完整测试 | 通过 | `cargo test --workspace` exit 0；`bun test` 298 pass / 5 skip / 0 fail |
| 最终文档提交 | 已推送 | `35caa80`/`40104492341`、`d6eee72`/`40104894749` 与 `589dc92`/`40106553383` 均已在 `main` 成功；本批仍未改产品代码 |

`40093591873` 只覆盖 32B 测量接线，不覆盖 `35caa80`/`d6eee72`/`589dc92` 的文档修正；
后者由 workspace gates `40104492341`/`40104894749`/`40106553383` 覆盖。本次证据提交只
记录运行号，不再改变契约；它的自身推送不再另列为契约证据。

## 三、需要收口的契约项

### F-1（阻断）`.xar` 的 required-feature 位与 20E 冲突

**现状**：20E 把归档专用位定为 bit 0（“标准库/能力要求存在”）：
[20E §3.4](20e-archive-wiring.md)，代码常量为
`core/rust/crates/xiao-artifacts/src/lib.rs:26`（`ARCHIVE_REQUIREMENTS_FEATURE = 1`）。
22A 却把同一个位写成 `0x0001=threads`（[22A §2.2](22a-concurrency-freeze.md)），
并在 §2.6 写“位集为超集时接受”；但当前解码器是精确相等：
`core/rust/crates/xiao-artifacts/src/lib.rs:1423`。

**后果**：如果并发位从 `0x0001` 开始，20E 的“归档要求存在”位会被改名；如果从 `0x0002`
开始，22A 的位表就得改。无论哪种，`required_features != ARCHIVE_REQUIRED_FEATURES` 的
精确相等检查都不支持“超集接受”。

**最终选择：甲**：

- 保留 `.xar` 的 bit 0 = `archive_requirements`；
- 并发位从 bit 1 开始：`0x0002=threads`、`0x0004=shared_handles`、`0x0008=task_pool`、
  `0x0010=multi_runtime`、`0x0020=serialization_v1`、`0x0040=cancel_wait`；
- 新读取器检查 `(required_features & REQUIRED) == REQUIRED`，不再要求整体相等；
- `.xiaoc` 的 `required_features` 与 `.xar` 的索引位分别建表，不共用“bit 0 就是 threads”
  的表述；
- 旧归档缺少并发位时，由 ABI 1.17 范围或 feature 子集检查拒绝。

备选：乙，新增独立 feature 字段并提升索引 schema；丙，22A-FIX 暂不分配并发位，等 22C。

### F-2（阻断）多 Runtime 没有“提交到哪个 Runtime”的入口

**现状**：API 是 `runtime_create() -> runtime`、`spawn(fn, a, b, ...)`、
`submit(pool, fn, a, b, ...)`（[22A §2.6](22a-concurrency-freeze.md)）。`spawn`/`submit`
不接收 `runtime`，也没有 `runtime_run`/`enter`/`current_runtime` 之类的入口。

**后果**：用户拿到另一个 `runtime` 句柄后，无法把线程或任务提交到它上面；“多 Runtime
并行”只存在于文档里。22B/22C 会临时发明一个入口，和冻结记录不一致。

**最终选择：甲**：

- 显式参数优先：`spawn(runtime, fn, a, b, ...)`、`pool_create(runtime, capacity)`；
  `submit(pool, fn, a, b, ...)` 继续经 `pool` 归属 Runtime；
- 如果保留“当前 Runtime”写法，补 `current_runtime() -> runtime`，并规定它与
  `runtime_create()` 的关系；
- `runtime_close` 的等待范围、跨 Runtime 共享只读句柄的规则不变。

备选：乙，`runtime_create(fn, a, b, ...)` 直接带入口函数；丙，首版只做一个 Runtime，把多
Runtime 后移。取甲或丙都必须同步 `22` 阶段和 UseDocs 的“多 Runtime”表述。

### F-3（阻断）`lock_guard<T>` 没有访问载荷的操作

**现状**：`lock(shared_mut<T>) -> lock_guard<T>`（[22A §2.6](22a-concurrency-freeze.md)），
文档说“只有守卫可以访问载荷”，但模块函数里只有 `read(shared<T>)`；没有
`read(lock_guard)`、`write(lock_guard, ...)` 或守卫方法。`shared_read(T)`/`shared_mut(T)`
是移动还是复制、`read` 是否递归深拷贝也没有写死。

**后果**：按当前 API，守卫拿到手后无法读或写 `T`；22B 无法定义 `lock_guard<T>` 的方法集，
22C 也无法生成访问入口。

**最终选择：甲**：

- 补模块函数 `lock_read(lock_guard<T>) -> T` 与 `lock_write(lock_guard<T>, T) -> none`，
  或等价地冻结 `lock_guard<T>` 的 `read`/`write` 方法；
- `shared_read(T)`/`shared_mut(T)` 取走 `T`（移动），源值不再可用；若允许复制，必须明确
  深拷贝规则和失败清理；
- `read(shared<T>)` 返回递归值拷贝；`lock` 只借用共享句柄，不消费；
- 守卫离开作用域释放锁；取消/异常路径的释放顺序仍按 07-D §10.4。

备选：乙，守卫通过字段访问语法直接读写；丙，引入 `read(lock_guard)` 重载。取甲最容易在
现有模块函数风格下实现，也最少扩语言表面。

### F-4（阻断 22B）Send/Sync 的“只读容器”没有映射到实际类型

**现状**：22A 写“候选 A 递归接受标量、文本和只读嵌套容器；可变容器拒绝”
（[22A §四](22a-concurrency-freeze.md)）。但产品类型里数组、字典表、字典列和集合都被
登记为可变容器（[00 §集合](00-decisions.md)）；`ReadOnlyContainer` 只是 07-D 研究原型的
枚举（`tests/research/concurrency/src/lib.rs:211`），不是 Xiao 类型。

**后果**：22B 无法判断 `array<int>`、`set<str>`、`dict<str,int>`、嵌套元组到底接受还是拒绝；
“只读”也没有类型层定义。

**最终选择：甲**：冻结一张按实际类型列出的表：

| 值 | 跨线程能力 | 说明 |
| --- | --- | --- |
| `none`、`int`/`sint`/`lint`、`float`/`sfloat`/`lfloat`、`bool`、`str` | 接受 | 标量/不可变文本 |
| 由接受值组成的 `tuple` | 接受 | 递归检查 |
| `array`/`set`/`dict`/字典表/字典列 | 拒绝 | 当前产品类型可变且没有只读类型 |
| 普通 `StrongHandle`/`WeakHandle` | 拒绝 | `X03-TYPE-031` |
| 显式 `shared<T>`/`shared_mut<T>` | 按 tag 接受 | 不做隐式升级 |
| `dynamic`、resource | 拒绝 | 编译期主规则；tag 13/14 只做防御性拒绝 |

“可变容器”必须改成可判定的定义（例如“含普通句柄/资源/动态值，或无法证明递归能力”），
不能继续用未定义的自然语言。备选：乙，首版只接受标量、`str` 和由它们组成的 `tuple`，
数组/集合/字典全部拒绝；丙，新增只读容器类型后再放开。

### F-5（阻断 22B）`CrossThread` 的事实名、IR 字段和诊断字段没有写死

**现状**：22A 的输入表明确要求“事实名称、IR 字段、诊断字段”
（[22A §二](22a-concurrency-freeze.md):68），但最终 §2.2 只写了“任务边界 + 线程边界两个
事实 + Runtime ID”；§四也只有同义描述。没有给出 IR 里的事实名或字段名。

**后果**：22B 正是写生命周期/IR 的批次；到这里只能自己命名，和“22B 不再决定语义”冲突。

**最终选择：甲**：冻结以下最小形状：

- 事实：`TaskBoundary`、`ThreadBoundary`；
- IR 字段：`runtime_id: u64`、`task_boundary: bool`、`thread_boundary: bool`；
- `source_runtime_id: Option<u64>` 是 22B-FIX.1 为 `shared_mut` 同 Runtime verifier 登记的
  可选扩展，使用 `serde(default)`，缺失时按未知来源处理；它不改变 32B 句柄头布局；
- 诊断字段：`boundary_kind`、`runtime_id`、`source_span`；
- 同线程挂起只产生 `TaskBoundary`；显式线程迁移同时产生两个事实；
- 诊断映射继续用 `X06-LIFETIME-006..008`。

备选：乙，明确允许 22B 命名，但把 22A 验收第 3 条改成“语义冻结、名称由 22B 定”。

### F-6（阻断 22C）产物 feature 与错误身份和现有格式不一致

**现状**：

- 22A 写 `.xiaoc` 的版本/特性不足用 `X17-XAR-007`（[22A §2.6](22a-concurrency-freeze.md)）；
  但 `.xiaoc` 未知必需功能位当前返回 `XIAOC-008`
  （`core/rust/crates/xiao-bytecode/src/xiaoc.rs:450`、`:776`），ABI 越界返回 `XIAOC-005`
  （`:442`），`SUPPORTED_REQUIRED_FEATURES` 目前是 0（`:40`）。
- 22A 写原生并发产物也“嵌入 ABI 1.17 + required-feature 位集”，但仓内没有原生产物的
  feature 容器；`required_features` 目前只出现在 `.xiaoc` 头和归档索引。
- 22A 写 `.xar` “位集为超集时接受”，当前解码器是精确相等（见 F-1）。

**后果**：22C 要么改现有 `.xiaoc` 错误契约和回归，要么违反 22A；原生产物没有落点。

**最终选择：甲**：

- 冻结一张逐产物表：
  - `.xiaoc` 格式/必需功能位继续用 `XIAOC-002/003/005/008`；
  - `.xar` 索引版本/ABI/缺失要求用 `X17-XAR-007`；未知 required-feature/capability 位
    使用**扩展后的** `X17-XAR-011`（不是当前 `required_features` 的既有路径）；
  - 原生并发产物如果进入首版，必须定义元数据段/符号和拒绝码；否则从首版矩阵移除；
- 如果要把 `.xiaoc` 统一改成 `X17-XAR-007`，必须单列迁移项，改现有错误映射和回归，
  不能只在 22A 文档里改一句话。

备选：乙，全部统一为 `X17-XAR-007` 并承担 `.xiaoc` 既有码的迁移；丙，原生并发产物
后置到 22E。

### F-7（阻断 22C）030/032 重叠，join 失败没有错误身份

**现状**：

- 22A §2.2 写“队列满、容量越界或关闭状态错误返回 `X06-RUNTIME-032`”；
- 同文 §2.6 表却写 `X06-RUNTIME-030` = “任务池已关闭/拒绝提交”，
  `X06-RUNTIME-032` = “任务池容量/参数/关闭状态错误”；
- §四写“失败在 join 点返回”，但错误表只有 detached 的 `X06-RUNTIME-028`，没有 join
  失败的身份。

**后果**：对“关闭池 submit”“池已关闭再 shutdown”“线程体失败后 join”三种情况，22C 无法
唯一选码；UseDocs 也重复了 030/032 的重叠。

**最终选择：甲**：

- `X06-RUNTIME-030` 只用于“池自身关闭后拒绝 `submit`”；
- `X06-RUNTIME-032` 只用于 `pool_create` 的容量/参数错误，以及明确的参数边界；
- `shutdown` 对已关闭池（Runtime 仍存活）幂等成功；Runtime 已关闭时优先 `031`；
- `join`/`wait` 的线程/任务失败重抛原始错误，不复用 detached 的 028；
- 同步更新 UseDocs 的并发错误表。

备选：乙，保留 032 作为所有池状态错误的统一码，删掉 030 的“关闭”语义；那就要改表，
不能让两个码都描述关闭。

### F-8（需补）`shared<T>` 跨 Runtime 的 Runtime ID 存在哪里

**现状**：22A 说跨 Runtime 只允许“带源 Runtime ID 的 `shared<T>` 只读句柄”，但 32B
头只有 strong/weak/tag/lock_state，没有 Runtime ID 字段。

**后果**：如果 ID 放在对象头，32B 布局要改；如果放在句柄包装，22B/22C 的 ABI 和类型
表示要写死。当前两种都没有定义。

**最终选择：甲**：Runtime ID 放在 `shared<T>` 句柄包装（例如 `{ runtime_id, object_ptr }`），
32B 对象头保持 F-1 的布局不变；跨 Runtime 只读检查使用包装里的 ID。备选：乙，在对象头
增加 8B 字段（会推翻 32B 测量，不推荐）；丙，首版禁止跨 Runtime 共享，只允许序列化值。

### F-9（低）文档与证据卫生

- `docs/UseDocs/language/concurrency/README.md` 的 `module: stdlib.xiao-concurrent` 不在
  `docs/module-registry.json` 的模块 ID 列表里；页面是 `planned`，门禁不报，但元数据悬空。
  要么登记保留 ID，要么改成与既有 `stdlib.xiao-standard-library-*` 一致的命名。
- 32B 探针在 Unix/Windows 两段 workflow 里各有一份源码；一次性证据可以接受，后续改动
  必须同步两份，或移到隔离研究 crate。
- `487ef1c` 已随 `35caa80` 推送；`35caa80`/`40104492341`、`d6eee72`/`40104894749` 与
  `589dc92`/`40106553383` 均已成功。本次证据提交只记录运行号，不再改变契约。

## 四、裁定表

| 编号 | 问题 | 最终选择 | 备选（保留历史） | 影响 |
| --- | --- | --- | --- | --- |
| F-1 | `.xar` feature 位冲突/精确相等 | 甲：保留 bit 0，并发位从 bit 1 起，子集检查 | 乙：新字段 + schema；丙：后置 | 22C 归档门禁 |
| F-2 | 多 Runtime 无提交入口 | 甲：`spawn(runtime, ...)` + `current_runtime`；`pool_create(runtime, ...)` | 乙：`runtime_create(fn, ...)`；丙：首版单 Runtime | 22B/22C API |
| F-3 | `lock_guard` 无访问操作 | 甲：`lock_read`/`lock_write` 模块函数 | 乙：守卫方法；丙：`read` 重载 | 22B/22C 共享可变 |
| F-4 | “只读容器”未映射 | 甲：按实际类型冻结递归表 | 乙：只接受标量/str/tuple；丙：新增只读类型 | 22B 类型检查 |
| F-5 | CrossThread 名称/IR 字段缺失 | 甲：冻结 `TaskBoundary`/`ThreadBoundary` + 字段 | 乙：允许 22B 命名 | 22B IR/生命周期 |
| F-6 | 产物错误身份/原生 features | 甲：逐产物表，`.xiaoc` 保留 XIAOC-*，原生定义或后置 | 乙：统一 X17-XAR-007；丙：原生后置 | 22C 产物门禁 |
| F-7 | 030/032 重叠、join 失败 | 甲：030 仅 submit 关闭池，032 仅创建参数，join 重抛原错误 | 乙：032 统一池状态 | 22C 诊断 |
| F-8 | shared 的 Runtime ID 位置 | 甲：句柄包装，32B 头不变 | 乙：头加字段；丙：禁止跨 Runtime | 22C ABI |

**来源**：2026-10-24 对 `620d8a2`/`487ef1c` 的审核；上述甲/乙/丙需星崽确认后才进入
22A-FIX 的文档修改。星崽已确认 F-1～F-8 全部取甲，以下收口记录生效；22B 可在本批提交后
开始。

### 4.1 已确认的最终落点（2026-10-24）

1. **F-1**：`.xar` bit 0 保留 20E `archive_requirements`；并发位从 bit 1 起，读取器拒绝
   未知位并按 required 子集检查；`.xiaoc` 使用独立 bit 0–5 表和 `XIAOC-008`。
2. **F-2**：新增 `current_runtime() -> runtime` 便捷别名；规范入口为
   `spawn(runtime, fn, a, b, ...)`、`pool_create(runtime, capacity)`，`submit` 经池归属 Runtime。
3. **F-3**：`lock_read(lock_guard<T>) -> T` 与 `lock_write(lock_guard<T>, T) -> none` 为模块
   函数；构造函数消费 `T`，`read` 做递归拷贝，守卫借用句柄并在作用域结束释放。
4. **F-4**：接受 `none`、固定宽度数值、`bool`、`str` 和递归安全 tuple；当前可变
   `array`/`set`/`dict`/字典表/字典列、普通句柄、动态值、resource 拒绝；显式共享按 tag 接受。
5. **F-5**：事实为 `TaskBoundary`/`ThreadBoundary`；IR 字段为 `runtime_id: u64`、
   `task_boundary: bool`、`thread_boundary: bool`；诊断字段为 `boundary_kind`、`runtime_id`、
   `source_span`；同线程挂起只置任务事实，显式线程迁移置两个事实。
6. **F-6**：`.xiaoc` 保留 `XIAOC-002/003/005/008`；`.xar` 的版本/ABI/缺失要求使用
   `X17-XAR-007`，未知 required-feature/capability 位使用**扩展后的** `X17-XAR-011`；
   原生并发产物后置到 22E，不在首版声明 feature 容器。
7. **F-7**：`X06-RUNTIME-030` 只表示池自身关闭后拒绝 `submit`；`X06-RUNTIME-032` 只表示
   `pool_create` 容量/参数错误；满队列使用可取消背压，`shutdown` 在 Runtime 仍存活时幂等
   成功，Runtime 已关闭时优先 `X06-RUNTIME-031`；`join`/`wait` 重抛线程体/任务体原错误，
   取消为 `X06-RUNTIME-027`。
8. **F-8**：只读共享句柄包装保存 `{ runtime_id: u64, object_ptr }`；32B 对象头不增加字段，
   `shared_mut<T>` 跨 Runtime 编译期拒绝。
9. **F-9**：UseDocs `module` 改为已有的 `stdlib.xiao-standard-library`；双平台临时探针接线
   作为已有测量证据保留，不在本批重测。

### 4.2 补丁收口（2026-10-24）

- **句柄模型**：`runtime`/`pool` 是可按值复制的共享句柄，别名共享同一关闭状态。
  `runtime_close`/`shutdown` 消费调用方传入的那一份并关闭底层状态；其他别名仍可观察关闭，
  重复关闭任一别名幂等成功。关闭后除 close 外的 runtime/pool 操作：Runtime 关闭优先报
  `X06-RUNTIME-031`，池自身 shutdown 后报 `X06-RUNTIME-030`；因此 Runtime 已关闭时
  `submit`/`shutdown` 选 031，不选 030。
- **错误身份**：`.xar` 的缺失要求、ABI 或版本不匹配仍为 `X17-XAR-007`；未知
  required-feature 位与未知 capability 位统一为**扩展后的** `X17-XAR-011`；`.xiaoc`
  未知功能位仍为 `XIAOC-008`。这要求 22C 增加“未知特征位 → 011”的映射和回归，
  不把“既有 011”当成已实现。
- **证据**：`35caa80`/`40104492341`、`d6eee72`/`40104894749` 与 `589dc92`/`40106553383`
  已记录；本次证据提交只记录运行号，不再改变契约。

## 五、本批要做的事

1. **改 22A 契约**：已更新 [22A](22a-concurrency-freeze.md) §2.2、§2.6、§2.8，并追加
   §2.9 修正记录；原五轮裁定历史保留，过时的“没有空白”结论已撤回。
2. **同步契约记录**：已把八项甲裁定写入本文 §4.1，作为 22B 的唯一输入。
3. **同步决策与阶段**：
   - [00 决策基线](00-decisions.md)：记录 F-1..F-8 的最终选择；
   - [12 P0](12-tests-and-milestones.md)：把 22A-FIX 放在 22A 和 22B 之间；
   - [22 阶段主文档](22-concurrency-execution-and-shared-handles.md)：下一道门改为 22B，
     写明 22A-FIX 已完成；
   - [README](README.md)：登记 22A-FIX 行。
4. **同步 UseDocs**：[并发执行与共享句柄](../UseDocs/language/concurrency/README.md)
   仍保持 `planned`，但错误码、feature 位、锁访问和产物矩阵必须与最终契约一致；本轮不把
   页面改成 `verified`。
5. **门禁**：`check:docs`、`check:usedocs`、`check:layout`、`git diff --check`；本批只改
   文档，记录“未改产品代码”。
6. **推送与收口**：`35caa80`/`d6eee72`/`589dc92` 已推送，workspace gates
   `40104492341`/`40104894749`/`40106553383` 成功；本次证据提交只记录运行号，22A-FIX
   关闭后进入 22B。

## 六、不许

1. **不许**写产品代码、AST、类型规则、IR、Runtime、LLVM 或标准库实现；
2. **不许**绕过 22A-FIX 修正契约开始或实现 22B；
3. **不许**用“位集为超集”一句话绕过 `.xar` 的精确相等现状；必须同时写代码/回归落点；
4. **不许**把 Runtime ID 塞进 32B 对象头而不重新测量布局；
5. **不许**改 `.xiaoc` 既有 `XIAOC-*` 错误映射却不列迁移项；
6. **不许**在本批之外重新选择已确认的 §四 甲/乙/丙；
7. **不许**把 `planned` 的 UseDocs 页面写成已实现。

## 七、验收

1. **裁定完整**：F-1..F-8 每项都有唯一选择和唯一落点；F-9 的卫生项已同步；
2. **交叉一致**：22A、00、12、22、README、UseDocs 的名称/编号/位表完全一致；
3. **22B 输入齐**：类型表、CrossThread 事实/字段、API 所有权、锁访问、诊断映射和产物
   矩阵都能直接消费；
4. **证据**：文档门禁全绿；`35caa80`/`d6eee72`/`589dc92` 分别有
   `40104492341`/`40104894749`/`40106553383` 运行号；本次证据提交只记录运行号；
5. **边界**：diff 不含产品代码；工作区干净；
6. **交接**：22B 只能按 22A-FIX 后的契约实现类型/生命周期/IR，不再补语义。

## 八、落点

```text
docs/DevDocs/22a-fix-concurrency-freeze.md              本文
docs/DevDocs/22a-concurrency-freeze.md                  修正记录与契约表
docs/DevDocs/00-decisions.md                            F-1..F-8 最终选择
docs/DevDocs/12-tests-and-milestones.md                 P0 的 22A-FIX 前置
docs/DevDocs/22-concurrency-execution-and-shared-handles.md  下一道门
docs/DevDocs/README.md                                  阶段索引
docs/UseDocs/language/concurrency/README.md             planned 入口同步
```

不改 `core/`、`cli/`、`stdlib/`、`tests/research/concurrency/` 和产品 ABI；32B 探针与
`40093591873` 只作为输入，不在本批重测。

## 九、风险

1. **过早改位表**：`.xar` bit 0 已有 20E 的旧归档语义；改错会让旧归档静默或误拒。
2. **多 Runtime 入口设计过大**：显式 Runtime 参数和 `current_runtime` 已冻结为最小首版入口，
   22B 不得再扩展生命周期 API。
3. **锁守卫扩语言表面**：`lock_read`/`lock_write` 是新增模块函数，必须与 `std.concurrent`
   的导入边界一起冻结，不能顺手加方法式 API。
4. **只读容器判定过宽**：当前数组/集合/字典仍拒绝，未来新增只读类型时必须另立裁定。
5. **文档再次漂移**：22A、00、12、README、UseDocs 已同笔更新，后续契约变更必须继续同步。

## 相关页面

- [22A. 并发入口冻结](22a-concurrency-freeze.md)——被修正的冻结记录
- [22. 并发执行与共享句柄](22-concurrency-execution-and-shared-handles.md)——阶段分解
- [20E. 归档接线](20e-archive-wiring.md)——`.xar` bit 0 的既有语义
- [00. 决策基线](00-decisions.md)——最终裁定
- [12. 测试与开发里程碑](12-tests-and-milestones.md)——P0 出口条件
- [并发执行与共享句柄 UseDocs](../UseDocs/language/concurrency/README.md)——`planned` 入口

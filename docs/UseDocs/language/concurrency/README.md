---
id: language.concurrency
title: 并发执行与共享句柄
status: planned
audience: learner
module: stdlib.xiao-standard-library-concurrent
stage: "22E"
version: "0.1.0"
related:
  - ../README.md
  - ../../../DevDocs/22a-concurrency-freeze.md
  - ../../troubleshooting/README.md
  - ../../troubleshooting/errors-and-reports.md
---

# 并发执行与共享句柄

> 本页登记首版 `std.concurrent` 公共接口。22E 的示例和产物校验正在三平台验收，
> 完成前保持 `planned`；最新实现和验证范围见 [22E](../../../DevDocs/22e-standard-library-and-release.md)。

## 首版范围

并发入口统一位于 `std.concurrent`，不提供独立的 `std.thread` 模块，也不提供裸 `spawn`。
首版包含 OS 线程、固定容量任务池、多 Runtime、只读/共享可变句柄和版本化边界值；Actor、
`async`/`await`、冻结共享和官方并发容器后置。

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

`join`、`detach` 和 `wait` 消费句柄；`cancel` 不消费句柄，取消后仍可取得取消结果。`detach` 只改变失败
交付方式：Runtime 关闭或 VM 退出 drain 已提交活动后，已经完成回收回调的失败会在本次运行结果的
`DetachedFailure` 结构化事件中保留失败及其 cause/suppressed 链，尚未完成的报告留在进程级通道，且不改变
主退出码；进程级报告最多保留 4096 条，丢弃数量可通过 Rust API 查询。无限 detached 活动必须通过取消
或截止时间收束。规范入口
显式接收 `runtime`，`current_runtime()` 只是便捷别名。函数值必须显式声明参数和一个返回值，
最多 64 个边界参数；无参函数合法，隐式闭包捕获不合法，捕获值必须显式传入。`runtime` 是
可按值复制的共享句柄，别名共享关闭状态；`runtime_close` 消费调用方那一份，其他别名仍可
观察关闭，重复关闭幂等成功。Runtime ID 随任务和线程事实传播。

### Runtime 的直接归属

Runtime 的 owner 只按直接创建关系记录。若 Runtime A 的执行单元创建 Runtime B，B 再创建
Runtime C，A 可以操作 B，B 可以操作 C，A 不会自动获得 C 的关闭权限；关闭 B 也不会递归
关闭 C，C 必须由 B 的上下文、C 自身上下文或无 Runtime 上下文的外部调用方处理。

语言层的 `runtime_close(runtime) -> none` 在非 owner 的其他 Runtime 上下文中使用时报告
`X06-RUNTIME-031`；Rust
和 VM 内部使用 `try_close()` 观察同一拒绝，兼容旧 `close()` 入口在拒绝时保持 no-op。

## 值传递与能力边界

跨线程值按位置深拷贝，不额外包 tuple。候选 A 接受 `none`、固定宽度数值、`bool`、`str` 和
由这些值递归组成的 `tuple`；当前可变的 `array`、`set`、`dict`/字典表/字典列没有只读类型，
因此编译期拒绝。普通句柄、动态值和资源值也在编译期拒绝；显式共享句柄的 `T` 仍须满足
同一递归能力表。深拷贝失败时按参数逆序清理已完成
副本，主错误保留在调用点，清理错误进入 `suppressed`。

`shared_read(T)`/`shared_mut(T)` 消费输入 `T`。`read(shared<T>)` 返回递归值拷贝；`lock` 借用
共享句柄，`lock_read` 读取守卫载荷，`lock_write` 写入并消费新值，守卫离开作用域释放锁。
普通句柄不能隐式升级。只读 `shared<T>` 的句柄包装携带源 Runtime ID，可以跨 Runtime 读取；
`shared_mut<T>` 跨 Runtime 编译期拒绝。

## 任务池和取消

`pool_create(runtime, capacity)` 的容量必须在 1–256 之间；参数错误为 `X06-RUNTIME-032`，
关闭 Runtime 为 `X06-RUNTIME-031`。队列满时 `submit` 施加可取消背压。`pool` 是可复制共享
句柄；`shutdown` 消费调用方那一份，拒绝新任务并等待已提交任务，其他别名仍可观察关闭，重复
关闭幂等成功。池自身关闭后 `submit` 返回 `X06-RUNTIME-030`；Runtime 已关闭时优先返回
`X06-RUNTIME-031`。排队任务按提交顺序执行。未开始任务可以被 `cancel` 移除，运行中任务在
安全点退出；取消排队任务会立即释放队列容量，对应 `wait` 立即返回 `X06-RUNTIME-027`，不等
worker 空闲。取消运行中任务时，`wait` 会等任务到安全点并完成 finally/drop 清理，再返回
`X06-RUNTIME-027`；清理错误保留在报告的 suppressed 链中。首版要求真实的持锁等待/平台唤醒路径，
锁等待和线程/任务取消统一返回
`X06-RUNTIME-027`。

## 序列化与产物兼容

边界值使用 magic 为 ASCII `XCON`、版本 `1` 的 12 字节大端头：4 字节 magic、`u16` 版本、
`u32` 当前节点长度、`u8` 类型标签和 1 字节保留位。标签 `0–12` 表示正常标量/容器，`13`
和 `14` 分别预留给 resource/dynamic；它们不能绕过编译期拒绝。单值最多 16 MiB，整帧最多
64 MiB，嵌套深度最多 64、总节点最多 1,000,000，超限使用 `X06-RUNTIME-029`。

共享对象使用 Runtime ABI `1.17` 的统一 32B/8B 头，tag `1/2/3` 分别为
`ordinary`/`shared_read`/`shared_mut`；Runtime ID 位于共享句柄包装，不进入对象头。`.xar` 保留
20E 的 bit 0=`archive_requirements`，并发位从 bit 1 起：`threads`、`shared_handles`、
`task_pool`、`multi_runtime`、`serialization_v1`、`cancel_wait`；required 位按子集检查，
未知位拒绝。`.xiaoc` 使用独立 bit 0–5 表，未知位保持 `XIAOC-008`。`.xiaoc` 继续使用
`XIAOC-*` 版本/特性错误；`.xar` 的版本/ABI/缺失要求使用 `X17-XAR-007`，未知
required-feature/capability 位使用扩展后的 `X17-XAR-011`；原生可执行文件在只读节内保存同一位表、ABI 区间、目标、生成器和标准库身份，工具链校验不执行用户代码。
32 位目标以 `X20-TARGET-001` 拒绝。

## 并发错误

并发错误读取稳定 `code`，不要匹配本地化文本：

| 编号 | 含义 |
| --- | --- |
| `X03-TYPE-030` | 值不满足跨线程能力 |
| `X03-TYPE-031` | 普通句柄不能跨线程 |
| `X03-TYPE-032` | 共享/普通句柄身份误用或隐式升级 |
| `X06-LIFETIME-006..008` | 任务/线程边界事实或 Runtime ID 非法传播 |
| `X06-RUNTIME-027` | 线程/任务/锁等待取消；该并发取消不会被用户 `catch` 吞掉 |
| `X06-RUNTIME-028` | detached 线程失败，写入进程级结构化错误通道 |
| `X06-RUNTIME-029` | 序列化、反序列化或边界上限失败 |
| `X06-RUNTIME-030` | 池自身关闭后拒绝提交 |
| `X06-RUNTIME-031` | 已关闭 Runtime 或跨 Runtime 非法使用 |
| `X06-RUNTIME-032` | `pool_create` 容量或参数错误 |

detached 线程的失败不会重抛到不存在的 join 点，也不会改变进程主退出码；报告保留 Runtime ID、
线程身份、原因链和 `suppressed` 清理错误。`join`/`wait` 会重抛线程体或任务体的原始错误，
不会把它改写成 detached 错误。完整错误报告字段见[运行时错误报告](../../troubleshooting/errors-and-reports.md)。

## 原生产物校验

`xiao build` 的并发原生产物携带内嵌元数据：Linux 使用 `.note.xiao`、Windows 使用
`.xiaomta`、macOS 使用 `__XIAO,__meta`。元数据版本为 1，Runtime ABI 精确为 1.17，
包含规范目标/64 位宽度、生成器版本和标准库接口身份。构建后会重新读取实际产物校验，
`xiao verify <可执行文件> --detail` 也只读取节，不启动程序。

`.xiaoc`、`.xar` 和原生的 ABI 1.16 Runtime 均在执行前拒绝；1.17 仍须满足位集。
原生元数据缺失、损坏、版本/目标/标准库身份不符使用 `X17-XAR-007`，未知并发位或
错误包含归档 bit 0 使用 `X17-XAR-011`。旧的无元数据普通程序仍可运行，但不能被
校验器当作已经满足并发产物要求。元数据检查不会替代程序启动时已有的 ABI 检查。

## 可执行示例

下面每个代码块均作为独立程序进入真实前端、VM 和公开 NativeBuild 的测试。

### 线程：多值传参与直接 Runtime 关闭

```xiao
from std.concurrent import runtime_create, spawn, join, runtime_close
def work(int value, int extra) -> int
    return value + extra
rt = runtime_create()
thread = spawn(rt, work, 1, 6)
print(join(thread))
runtime_close(rt)
```

输出 `7`。值按参数位置复制，`join` 消费线程句柄并取得结果。

### 任务池：提交、等待与关闭

```xiao
from std.concurrent import current_runtime, pool_create, submit, wait, shutdown
def work(int value) -> int
    return value + 5
rt = current_runtime()
pool = pool_create(rt, 1)
task = submit(pool, work, 7)
print(wait(task))
shutdown(pool)
```

输出 `12`。关闭池会停止接收新任务并等待已提交活动。

### 共享值：守卫离开函数时释放

```xiao
from std.concurrent import shared_read, shared_mut, read, lock, lock_read, lock_write
def update(shared_mut<int> value) -> none
    guard = lock(value)
    lock_write(guard, 9)
def inspect(shared_mut<int> value) -> int
    guard = lock(value)
    return lock_read(guard)
constant = shared_read(4)
value = shared_mut(read(constant))
update(value)
print(inspect(value))
```

输出 `9`。不要在持有守卫的同一作用域再次锁定同一共享值。

### detached：入口正常收尾

```xiao
from std.concurrent import current_runtime, spawn, detach
def work() -> int
    return 1
rt = current_runtime()
thread = spawn(rt, work)
detach(thread)
print("main")
```

输出 `main`，主退出码为 0。detached 失败仍保留 cause/suppressed，原生入口在 drain 后
输出结构化诊断，VM 记录 `DetachedFailure`，两者都不改变主退出码。

### 取消已完成但尚未回收的线程

```xiao
from std.concurrent import current_runtime, spawn, runtime_close, cancel, join
def work() -> int
    return 3
rt = current_runtime()
thread = spawn(rt, work)
runtime_close(rt)
cancel(thread)
join(thread)
```

没有标准输出，报告 `X06-RUNTIME-027`，退出码为 3。关闭 Runtime 已等待线程完成，
之后取消仍在回收时生效；027 不会进入 Xiao 的普通 `catch`。首次取消在 finally 内到达
仍会投递；已经投递的取消不会再次打断同一执行单元的清理。

## 验证范围与兼容

三种产物的最小位集、缺位/未知位、旧 Runtime 和原生节校验由 `22e_release` 回归覆盖。
语言深度与 Runtime 限制须区分：XCON Runtime 接受 64 层；当前源码到原生的优化快照
对深递归类型还有 JSON 深度限制。该限制及宿主精确阻塞取消的证据层次见
[22D §9.3](../../../DevDocs/22d-llvm-concurrency-differential.md#93-22d-fix1-收尾矩阵与证据)。

标准库形态保持虚拟原生模块，签名由冻结表提供，行为由 Runtime 实现，没有同名 `.xiao`
包装或可替换的第二套签名。发布验收和三平台实际运行号见
[22E 实施记录](../../../DevDocs/22e-standard-library-and-release.md)。

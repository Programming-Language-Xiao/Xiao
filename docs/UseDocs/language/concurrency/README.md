---
id: language.concurrency
title: 并发执行与共享句柄
status: planned
audience: learner
module: stdlib.xiao-standard-library
stage: "22A/22C"
version: "0.1.0"
related:
  - ../README.md
  - ../../../DevDocs/22a-concurrency-freeze.md
  - ../../troubleshooting/README.md
  - ../../troubleshooting/errors-and-reports.md
---

# 并发执行与共享句柄

> 本页是 22A 的用户文档入口和冻结契约摘要，当前状态为 `planned`。22A 只冻结名称、边界、
> 错误和产物兼容规则；22B–22E 完成实现、测试和示例前，下面的形状不能作为可运行功能使用。

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

`join`、`detach` 和 `wait` 消费句柄；`cancel` 不消费句柄，取消后仍可取得取消结果。规范入口
显式接收 `runtime`，`current_runtime()` 只是便捷别名。函数值必须显式声明参数和一个返回值，
最多 64 个边界参数；无参函数合法，隐式闭包捕获不合法，捕获值必须显式传入。`runtime` 是
可按值复制的共享句柄，别名共享关闭状态；`runtime_close` 消费调用方那一份，其他别名仍可
观察关闭，重复关闭幂等成功。Runtime ID 随任务和线程事实传播。

> **22C 实现补充（本页仍为 `planned`）**

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
安全点退出。首版要求真实的持锁等待/平台唤醒路径，锁等待和线程/任务取消统一返回
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
required-feature/capability 位使用扩展后的 `X17-XAR-011`；原生并发产物后置到 22E。
32 位目标以 `X20-TARGET-001` 拒绝。

## 并发错误

并发错误读取稳定 `code`，不要匹配本地化文本：

| 编号 | 含义 |
| --- | --- |
| `X03-TYPE-030` | 值不满足跨线程能力 |
| `X03-TYPE-031` | 普通句柄不能跨线程 |
| `X03-TYPE-032` | 共享/普通句柄身份误用或隐式升级 |
| `X06-LIFETIME-006..008` | 任务/线程边界事实或 Runtime ID 非法传播 |
| `X06-RUNTIME-027` | 线程/任务/锁等待取消 |
| `X06-RUNTIME-028` | detached 线程失败，写入进程级结构化错误通道 |
| `X06-RUNTIME-029` | 序列化、反序列化或边界上限失败 |
| `X06-RUNTIME-030` | 池自身关闭后拒绝提交 |
| `X06-RUNTIME-031` | 已关闭 Runtime 或跨 Runtime 非法使用 |
| `X06-RUNTIME-032` | `pool_create` 容量或参数错误 |

detached 线程的失败不会重抛到不存在的 join 点，也不会改变进程主退出码；报告保留 Runtime ID、
线程身份、原因链和 `suppressed` 清理错误。`join`/`wait` 会重抛线程体或任务体的原始错误，
不会把它改写成 detached 错误。完整错误报告字段见[运行时错误报告](../../troubleshooting/errors-and-reports.md)。

## 当前状态与后续

代码、测试和可执行示例完成前，本页保持 `planned`，不能从这里推断当前工具链已经支持并发。
实现完成后，22E 必须补齐线程/共享/任务池教程、错误示例和 ABI/产物兼容回归，再将本页改为
`verified`。设计依据和四平台 32B 布局证据见[22A 冻结记录](../../../DevDocs/22a-concurrency-freeze.md)。

# B4-3 LLVM 实施证据

B4-3 将动态/静态 cmath 接到既有 C ABI，并使 Float `**` 共用 `cmath_pow`。ABI保持1.18，生成器21；`stdlib.cmath` 导入仍planned，B4-4、B5和公共发布尚未开始。本目录只记录 Windows x86_64-msvc 内部证据。

`record.py` 为每次执行记录固定源码摘要、DevDocs 原始与UTF-8/LF摘要、环境、PID、时间、命令、退出码和内容寻址日志。每种变体有独立 target、Runtime、缓存、Xiao Home 和临时目录。`runtime-identity`在冷构建后记录新Runtime静态库的路径、长度和SHA-256；日志对象同摘要只存一份，失败轮次不覆盖。


## 执行记录

| 变体 | 固定代码 | 结果 |
| --- | --- | --- |
| baseline | `0a9e68a7` | B4-2入口驱动回归通过；提供接线前比较基点 |
| fixed | `b77b0d5c` | 原生矩阵通过；合并回归发现两项既有 hex 隔离失败，原日志保留 |
| fixed-final | `6056fb94` | 移除无条件 cmath 声明后，冷构建通过；原生矩阵发现动态函数局部声明未汇总，失败回执保留 |
| fixed-final2 | `e0d5993e` | 冷构建、完整矩阵、跨阶段回归、B1 VM/原生、核心测试、Clippy、fmt和固定历史审计通过；定稿后文档门禁由独立回执记录 |

前两个 fixed 变体分别保留了 hex 模块污染和动态调用声明缺失这两轮失败；它们不作为最终通过结论。`fixed-final2`修复后，四组静态/动态 O0/O3 各核对18,318项，其中3,153项为预期错误结果，并覆盖全部1,075组Float pow历史差异。每个产物都通过位模式或错误码比较；状态0/1/2/3/6/8、动态坏标签out不变、挂起槽不变及七个语言退出/catch示例也通过。

第一轮六个历史审计和一次内容寻址自测并发运行时，互相可见的临时stdout暂存目录改变了DevDocs运行中摘要；这七条回执和日志保留，但不进入有效结论。采集器将`xiao-log-stage-*`识别为运行期文件后，六个历史审计已串行重跑并通过，内容寻址自测也以`collector-final`通过。另有一条摘要稳定的早期B4-2B审计被串行结果取代；三条早期文档门禁均通过，但因审计器后续更新而保留为前序记录。收口清单记录全部排除原因。

最终源码快照为`e0d5993e7c8214f2277d1bb81f9b05357d9305e0`，包含1,059个受保护产品文件。独立Runtime静态库`xiao_runtime.lib`为27,482,962字节，SHA-256 `27b460d2fde00c0e36f2799474e3a21310d5555530cbc8d7694caf6df220702d`；同一身份由`fixed-final2/runtime-identity.json`固定采集。B1 VM与原生精确入口通过；原生入口内含B2-A/B语料。B1、B2、B3、B3-4、B4-0/1和B4-2B固定历史审计均通过。workspace五个核心crate测试、B4跨阶段回归、workspace Clippy与fmt均通过。绑定最终审计器、Runtime身份回执、终稿README和当前源码快照的完整仓库门禁为`fixed-final2/final-docs-runtime.json`。

在仓库根复核已定稿的记录：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/record.py content-store-self-test
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/record.py run fixed-final2 native-matrix -- cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --release --test 20o_b2_b4_cmath_llvm -- --ignored --exact native_cmath_static_dynamic_o0_o3_and_shared_pow --nocapture
```

最终只读复核与反例：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/audit.py --self-negative
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/audit.py --live-runtime
```

默认审计读取固定Git对象、Runtime身份回执和运行回执，不要求当前HEAD或fixed任务目录；`--live-runtime`才要求仍在本机任务根的Runtime静态库存在，并与记录的长度和SHA-256逐字节比对。`--collect`是显式生成阶段收口与清单，默认审计不写文件；反例覆盖清单、属性文件、孤立日志、产品摘要、DevDocs绑定、任务路径逃逸和阶段状态。

Linux、macOS、远端CI、旧721 F10与性能预算未执行；B4-3是Windows本机内部退出，不是三平台/性能通过，也不表示cmath公开导入可用。ABI仍1.18、生成器21；B4-4、B4-5和B5未开始。

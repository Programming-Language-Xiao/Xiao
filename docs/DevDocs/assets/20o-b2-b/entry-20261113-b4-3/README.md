# B4-3 LLVM 实施证据

B4-3 将动态/静态 cmath 接到既有 C ABI，并使 Float `**` 共用 `cmath_pow`。ABI保持1.18，生成器21；`stdlib.cmath` 导入仍planned，B4-4、B5和公共发布尚未开始。本目录只记录 Windows x86_64-msvc 内部证据。

`record.py` 为每次执行记录固定源码摘要、DevDocs 原始与UTF-8/LF摘要、环境、PID、时间、命令、退出码和内容寻址日志。每种变体有独立 target、Runtime、缓存、Xiao Home 和临时目录。日志对象同摘要只存一份；失败轮次不覆盖。

## 执行记录

| 变体 | 固定代码 | 结果 |
| --- | --- | --- |
| baseline | `b77b0d5c` | 入口驱动回归通过；提供接线前后比较基点 |
| fixed | `b77b0d5c` | 原生矩阵通过；合并回归发现两项既有 hex 隔离失败，原日志保留 |
| fixed-final | `6056fb94` | 移除无条件 cmath 声明后，冷构建通过；原生矩阵发现动态函数局部声明未汇总，失败回执保留 |
| fixed-final2 | 待固定 | 按需提升实际动态调用声明的修复；完整原生矩阵、回归和门禁待运行 |

因此，前两个独立原生结果不作为最终通过结论。唯一最终结果应引用`fixed-final2`的成功回执；收口前还需完成 B4-3 固定回归、历史审计及定稿后的文档门禁。

在仓库根复核已定稿的记录：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/record.py content-store-self-test
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/record.py run fixed-final2 native-matrix -- cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --release --test 20o_b2_b4_cmath_llvm -- --ignored --exact native_cmath_static_dynamic_o0_o3_and_shared_pow --nocapture
```

原生矩阵应在四个静态/动态 O0/O3 产物各核对18,318项，包括全部1,075组历史pow输入、C ABI错误状态和语言级退出/catch。成功日志、runtime-build回执及源码/文档摘要必须都来自同一 fixed-final2 变体。

Linux、macOS、远端CI和性能预算未执行；B4-3不得据此写成三平台或性能通过，也不表示cmath公开导入可用。

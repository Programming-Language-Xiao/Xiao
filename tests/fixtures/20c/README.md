# 20C 能力探测夹具

本目录只保存 20C §3.1 的两个最小 Xiao 源码夹具，不是官方标准库，也不进入普通包依赖求解。

| 文件 | 探测范围 | 预期观察 |
| --- | --- | --- |
| `strings-path.xiao` | 字符串拼接形成相对路径 | VM 与 LLVM 原生输出 `lib/core` |
| `arithmetic-error.xiao` | 函数算术、运行期整数溢出与错误路径 | VM 保留 `X06-RUNTIME-009`；原生结果按实际后端/链接/进程状态记录 |

夹具由 `core/rust/crates/xiao-driver/tests/c20_capability_probe.rs` 读取。这里不添加
intrinsic、不绕过类型检查，也不把探测结果当作官方库接口承诺。

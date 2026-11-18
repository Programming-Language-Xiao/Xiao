# B6-1 macOS 专项补验

星崽授权在 macOS Runner 跑 B6-1。2026-11-18，[运行 44243357695](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44243357695) 的 native、resources 作业均通过。B6-2 据此开工。

| 运行 | 结论 |
| --- | --- |
| [44241879381](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44241879381) | 两作业在构建前失败，Windows 检出的归档换行导致源码摘要与 macOS 不同 |
| [44242377713](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44242377713) | 官方包装和 Runtime 参考向量通过，剩余作业因教程未导出和 Mach-O 测试汇编标识缺少下划线而失败 |
| [44243357695](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44243357695) | native `134596281404`、resources `134596281121` 均通过，attempt 1 |

成功作业使用 `macos-15`、`aarch64-apple-darwin`、Rust 1.96.0、LLVM 21 和独占任务根。Runner 提交 `4bc244c90b41da2aa314d3da05e2d30bd647cc5f`，固定源码 `ef456f17e29ca3955f2106847b85b018f946cb44`，源码清单 1,059 项。两份 Runtime 各自从固定源码新构建，命令、源码、工具、Runtime、日志和包装产物摘要见归档回执。

- 两个官方导入别名各自以原生 O0/O3 运行。四格 Mach-O 均为 `Complete`，组件声明与观察均为 `containers/rc/value/weak`，黄金输出及 009 均通过。
- Runtime 为 305 通过、0 失败、1 忽略。15 项正式参考共 17,243 条。VM 资源/归档目标为 42/0/3，其中 A6 资源 8/8；codegen 为 109/0/15，额外 driver 为 17/0/3。原生核心、011、显式除法、B1 精确入口、fmt 和 Clippy 均通过。
- Windows/Linux 既有证据见[限定重跑](../entry-20261118-b6-1-rerun/README.md)。Windows COFF 导出完整性仍为 `UnverifiedCoffExports`，Linux Clippy 未执行。

在仓库根运行 `python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-1-macos/audit.py`。清单记录三次运行下载的文件，以实际字节的 SHA-256 寻址。同一内容只存一份；审计按固定 Git 对象、CI 回执和日志交叉复核。

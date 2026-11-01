# B1-FIX-A Windows 本地日志

工程期：20O-B1-FIX-A；保存新增七项加原七项的精确 VM/native 执行日志。

两份日志各对应一个恰好 1 passed/0 failed/0 ignored 的 Rust 测试，包含原七项与新增七项的
逐场景记录。原生使用当前 ABI 1.17 的 Runtime staticlib 与显式 LLVM；不包含尚未实现的
intrinsic 32 / ABI 1.18。它们不是本轮 Linux/macOS 或远端运行证据，也不是产品性能成绩。

命令和未覆盖项见 [FIX-A 收口](../../20o-b1-fix-a-contract-closeout.md)。

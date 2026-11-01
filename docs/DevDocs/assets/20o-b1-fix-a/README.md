# B1-FIX-A Windows 本地日志

工程期：20O-B1-FIX-A；保存新增七项加原七项的精确 VM/native 执行日志。

两份日志各对应一个恰好 1 passed/0 failed/0 ignored 的 Rust 测试，包含原七项与新增七项的
逐场景记录。FIX-A 取证时使用 ABI 1.17 的 Runtime staticlib 与显式 LLVM；当时尚未实现
intrinsic 32 / ABI 1.18，因此这些日志不覆盖后续 FIX-B。它们不是 FIX-A 的 Linux/macOS
或远端运行证据，也不是产品性能成绩；当前实现状态见 [FIX-B 收口](../../20o-b1-fix-b-closeout.md)。

命令和未覆盖项见 [FIX-A 收口](../../20o-b1-fix-a-contract-closeout.md)。

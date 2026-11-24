# B3-1 time 开工基线

B3-0F 已签收冻结交接。Windows 本机在独立任务根从 `e79fab3c` 导出源码，使用 Rust 1.96.0 锁定构建 release Runtime。构建退出码为 0，回执登记 Runtime、两个 Cargo.lock、目标及本机 LLVM 工具摘要。锁文件同时保存导出字节和规范 LF 摘要，规范摘要与固定 Git 对象一致。[冻结交接](../entry-20261124-b3-0f/README.md) · [Windows 基线回执](baseline-windows.json)

Linux 和 macOS 的宿主探测和 Runtime 身份引用 B3-0A。B3-1 的三平台 time 执行和 Windows 产品级 COFF 验收待实施，开工裁定见规划 §3。[三平台入口](../entry-20261122-b3-0/entry.json) · [time 实施规划](../../../20o-b3-1-time-implementation-plan.md)

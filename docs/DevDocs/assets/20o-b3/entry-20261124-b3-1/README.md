# B3-1 time 实施证据

B3-0F 已签收冻结交接。Windows 本机在独立任务根从 `e79fab3c` 导出源码，使用 Rust 1.96.0 锁定构建 release Runtime。构建退出码为 0，回执登记 Runtime、两个 Cargo.lock、目标及本机 LLVM 工具摘要。锁文件同时保存导出字节和规范 LF 摘要，规范摘要与固定 Git 对象一致。[冻结交接](../entry-20261124-b3-0f/README.md) · [Windows 基线回执](baseline-windows.json)

开工裁定已登记。49～52 激活，53～63 为不可调用的 `reserved`，time 夹具已迁入正式目录，registry 状态为 `planned`。Windows time 原生入口由产品验证器核对同次 MSVC 链接生成的完整映射，保存映像和映射的 SHA-256。[裁定](decisions.json) · [time 实施规划](../../../20o-b3-1-time-implementation-plan.md)

`run.py` 从干净的固定提交执行 time Runtime 边界、VM 与原生精确入口，将命令和原始日志存入独立任务根。原生注入固定时钟使用 `time-native-test` 和独立的 `test-failpoints` Runtime。Linux 与 macOS Runner 使用本目录的工作流模板。三平台取证齐备后方可收口。[三平台入口基线](../entry-20261122-b3-0/entry.json) · [采集脚本](run.py) · [Runner 模板](time-runner.yml)

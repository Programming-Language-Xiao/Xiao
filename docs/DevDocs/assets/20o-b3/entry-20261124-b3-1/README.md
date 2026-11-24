# B3-1 time 实施证据

B3-0F 已签收冻结交接。开工基线仍固定为 `e79fab3c`、ABI 1.19、生成器 22。独立任务根使用 Rust 1.96.0 锁定构建 Windows release Runtime，回执登记 Runtime、两个 Cargo.lock、目标及本机 LLVM 工具摘要。锁文件同时保存导出字节和规范 LF 摘要，规范摘要与 Git 对象一致。[冻结交接](../entry-20261124-b3-0f/README.md) · [Windows 基线回执](baseline-windows.json)

开工裁定已登记。产品快照 `8d1c09f0` 激活 49～52，并将 53～63 保留为不可调用的 `reserved`。ABI 为 1.20，生成器为 23，标准库接口为 1；time 夹具迁入正式目录，registry 与教程仍为 `planned`。[裁定](decisions.json) · [time 实施规划](../../../20o-b3-1-time-implementation-plan.md)

`run.py` 在独立任务根执行 Runtime 边界、VM、双导入的 O0/O3 原生教程和时钟/能力错误样例，保存原始命令日志、工具和 Runtime 身份、睡眠/取消延迟及原生产物。Windows 的六份可执行文件分别绑定本次 MSVC 链接生成的完整 map，验证器按映像与 map SHA-256 取证并核对声明组件、观察组件和未分类符号。

Linux/macOS CI run `45428004405` 的两个作业分别通过。Windows 本机 time 专项绑定 `8d1c09f0` 并通过 COFF `Complete`。产品同一 Git 提交的 Linux workspace-gates CI run `45429820367` 通过，该作业在干净的 GitHub Runner 上执行完整的仓库门禁。[三平台冻结基线](../entry-20261122-b3-0/entry.json) · [执行入口](run.py) · [Linux/macOS 工作流模板](time-runner.yml)

首轮及修复轮日志保留于 `history.zip`，没有用后续通过覆盖失败。首轮 Linux 原生组件观察把 ELF 的 `STT_FILE` 编译单元名当成 Runtime 函数。修正符号类型过滤后继续发现实际输出槽对 `rc` 的静态链接依赖，当前产品组件表及逐导出 Linux 复验已覆盖该依赖。第一轮 Windows 专项还发现 `finally` 在同层 `catch` 前执行，黄金输出已据语言既有展开次序修正。

`audit.py` 只读验证 Git 对象、三平台结果、工具和 Runtime 摘要、原生文件与 COFF map 原始字节、CI 作业、workspace-gates、文档门禁与封存清单。Windows 的教程、边界和 COFF 标记在核对原始日志摘要后统一用 `/` 比较路径。`self_negative.py` 在临时副本运行一个基线和 8 个拒绝用例，逐条比对拒绝原因及审计前后摘要。路径反例用正斜杠重现日志格式差异，再验证错误的 COFF map 路径会被拒绝。`closeout.json` 派生 `b3_1_passed=true` 和 `b3_2_entry_ready=true`。B3-2 尚未开始。[审计](audit.py) · [负例](self_negative.py) · [收口脚本](seal.py) · [CI 采集](collect_ci.py) · [收口回执](closeout.json)

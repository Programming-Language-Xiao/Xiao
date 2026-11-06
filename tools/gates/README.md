# 工作区门禁

工程期：仓库维护与 20O-B1。存放工作区门禁、平台入口和 B1 精确执行点的计数守卫。

`b1-probe-guards.ps1` 直接从维护回归工作流提取默认 Bash、Linux/macOS 原生 Bash 和
Windows PowerShell 的判断代码，模拟七类 cargo 输出/退出码，共 28 个场景：单项通过、
零匹配、多匹配、ignored、失败、绿色文字但退出失败、重复汇总。它不运行产品或外部服务。

需要 PowerShell 7 和 Bash，不依赖 Python/PyYAML；Linux/macOS 可直接运行：

```powershell
pwsh -NoProfile -File tools/gates/b1-probe-guards.ps1
```

Windows 显式指定已安装的 MSYS Bash，可把机器报告写入任意已有输出目录：

```powershell
pwsh -NoProfile -File tools/gates/b1-probe-guards.ps1 -BashPath D:/msys64/usr/bin/bash.exe -ReportPath C:/Temp/b1-guards.json
```

报告含工作流 SHA-256、宿主/PowerShell 版本及逐场景退出码。维护回归默认作业执行此脚本，
脚本自身的改动也触发维护回归。脚本通过只证明计数判断能拒绝错误执行结果，不等于真实
VM/native 三平台通过；发布状态仍以 [B1 证据表](../../docs/DevDocs/20o-b1-freeze-draft.md) 为准。

FIX-B 将 Windows 的既有 B1 精确块提前到 Runtime 单测之后，仍在原作业内执行一次。
守卫只提取 B1 的闭合校验块；后续 22D 等门禁继续执行并决定作业结论，不能因 B1 已通过
而被省略。四条路径和 28 个正反例保持不变。

REM-C 在同一维护回归内增加手动输入 `b1_f10=true`，默认关闭。启用后在现有作业内串行运行
显式 release 测量工具：Linux 默认作业测 VM，三平台原生作业分别测 O0/O3；JSON 作为 artifact
保存。仅该模式的作业总看门狗为 60 分钟，函数预算仍是 VM 10 秒、原生 2 秒、翻倍比≤3、
增量内存≤64 MiB。它不增加 B1 精确目标、守卫路径或 ignored 测试，原有门禁全部继续执行。
测量窗口、校准和完整负载见 [工具说明](../../core/rust/crates/xiao-driver/examples/README.md)。

20O-B2-A1的controlled-cargo.py在CI完成Rust设置后安装任务级PATH代理。
构建/测试/检查/metadata等命令强制--locked，全部子进程清理继承的RUST_BIGDECIMAL_*；版本/帮助查询不注入无效参数。
原Cargo命令形态和stdout/退出码保留，B1模拟cargo函数仍优先于PATH代理，28/28守卫不变。
代理只写runner临时目录与GITHUB_PATH，不改系统环境、Cargo全局配置、测试目标或阈值。

A6增加默认关闭的手动输入`b2_a_perf`，与`b1_f10`互斥；两个开关同时开启时，
`b2-a6-selection.py`以退出码2明确拒绝。仅A6模式的VM/原生作业总时限为40分钟，
默认20分钟及B1 F10的60分钟保持原值。测量前执行资源8项和协议3项，随后继续原有门禁。
VM使用release，三平台原生分别记录O0/O3，逐批原始样本随报告上传，名称包含运行号和尝试号。
函数10秒/2秒、64MiB与翻倍比3/5不变，旧721条F10不在A6模式运行。

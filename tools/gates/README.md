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

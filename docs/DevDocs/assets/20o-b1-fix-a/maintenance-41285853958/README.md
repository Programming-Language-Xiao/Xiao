# FIX-A 维护回归 41285853958 证据摘录

工程期：20O-B1-FIX-A；来源为 GitHub Actions 官方 jobs/logs 接口，提交
4d347670cdfa05fac2f01888f9d5ff6e3a5db37e，attempt 1。

[机器证据清单](evidence.json) 记录实际运行号、作业链接、下载到本地的完整作业日志 SHA-256 与摘录行号。
2026-11-01 已按同一 `gh api repos/.../actions/jobs/<id>/logs` 方式重新下载四份完整日志并重算哈希；
Windows 哈希不变，默认 VM/Linux/macOS 三项已更正为可复算值。
四份 .log 从实际 20o_b1_probe 启动行截到紧随指定测试的结果汇总，保留原始时间戳。
摘录仅去除行尾空白并统一为 LF；JSON 中的 SHA-256 仍针对完整下载源日志，不是规范化后的摘录。
每份都有七条 B0 与七条 FIX-A 场景、一条目标成功记录及 1 passed/0 failed/0 ignored。

- [默认 VM（Linux）](default-linux.log)
- [Windows 原生](windows.log)
- [Linux 原生](linux.log)
- [macOS 原生](macos.log)

可用以下只读命令重新取得运行元数据及默认作业完整源日志；其余作业 ID 见 evidence.json：

```powershell
gh api repos/Programming-Language-Xiao/Xiao/actions/runs/41285853958
gh api repos/Programming-Language-Xiao/Xiao/actions/jobs/124786889391/logs
```

这只验证探测/CI 接线：没有验证 error_code 实现、ABI 1.18、七库产品或 CSV 全量性能。
FIX-A 与下一批边界见 [收口记录](../../../20o-b1-fix-a-contract-closeout.md)。

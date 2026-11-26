# B3-3 fs 实施记录

B3-3A 已复跑 B3-2 的固定对象审计和九项自反例。生产基线为 `879ce625`，维护提交 `2aae3a74` 的唯一产品路径差异位于 IR 单测内，生产前缀逐字一致。两个 Cargo.lock 同时记录固定 Git blob 与工作树字节。[基线](baseline.json) · [审计回执](b3-2-audit.json) · [自反例回执](b3-2-self-negative.json)

B1 守卫沿用 B3-1 的 4 项实际 Cargo 输出与 24 项派生负例。B3-2A 的 Windows 回放包含 28 项模拟和 0 项实际 Cargo 输出。Windows、Linux 和 macOS 的前批 Runtime 摘要来自 B3-2 收口。[B3-2 收口](../entry-20261125-b3-2-final/closeout.json)

首次复跑在 C 盘系统临时目录复制归档时遇到磁盘空间不足。重跑使用 D 盘独立任务根。Windows 原生专项的目标为 `x86_64-pc-windows-msvc`。[环境](environment.json)

固定产品 `5cb5a2c3` 已接入 Runtime、VM/TAC 和五个 C ABI。Windows 本机和 WSL Linux 各通过 289 项 Runtime 测试、10 项编号契约测试和 13 项 VM 回归。两种导入路径均与冻结黄金逐字一致，结束时 `demo` 为空。两平台 release Runtime 均存在五个 fs 导出。Windows 与 Linux 验证了末段符号链接，Linux 另外实测了跨卷重命名拒绝。Runtime 静态库字节留在本机任务根，归档绑定其 SHA-256 与导出工具输出。[状态](status.json) · [原始回执](runs.zip)

WSL 的早期采集分别遇到登录工具链路径缺失、任务临时目录缺失，以及两种 `nm` 长选项拼写差异。归档保留三个失败回执。修正采集器和任务环境后，以同一产品提交复跑。日志按 SHA-256 存储，每个摘要对应一个对象。[采集器](collect_local.py)

本机回执固定在生成器 24 阶段，当时原生接线和三平台取证待执行。星崽随后批准生成器 25 与原生接线。三平台专项由 `run_native.py` 采集。[补充裁定](generator-decision.json) · [实施进度](../../../20o-b3-3-fs-implementation-plan.md)

## 只读复核

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261125-b3-3/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261125-b3-3/self_negative.py --temp-root D:/project/Xiao/.b3-3/20261126-4bea2e43/temp
```

自反例核对原样审计、篡改清单、删除 `.gitattributes` 和伪造通过状态，逐项比较退出码、拒绝文本以及前后字节。[清单](evidence.json) · [封存入口](seal_local.py)

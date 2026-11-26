# B3-3 fs 实施记录

B3-3A 已复跑 B3-2 的固定对象审计和九项自反例。生产基线为 `879ce625`，维护提交 `2aae3a74` 的唯一产品路径差异位于 IR 单测内，生产前缀逐字一致。两个 Cargo.lock 同时记录固定 Git blob 与工作树字节。[基线](baseline.json) · [审计回执](b3-2-audit.json) · [自反例回执](b3-2-self-negative.json)

B1 守卫沿用 B3-1 的 4 项实际 Cargo 输出与 24 项派生负例。B3-2A 的 Windows 回放包含 28 项模拟和 0 项实际 Cargo 输出。Windows、Linux 和 macOS 的前批 Runtime 摘要来自 B3-2 收口。[B3-2 收口](../entry-20261125-b3-2-final/closeout.json)

首次复跑在 C 盘系统临时目录复制归档时遇到磁盘空间不足。重跑使用 D 盘独立任务根。Windows 原生专项的目标为 `x86_64-pc-windows-msvc`。[环境](environment.json)

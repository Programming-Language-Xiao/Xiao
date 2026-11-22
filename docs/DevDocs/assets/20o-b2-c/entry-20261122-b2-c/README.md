# B2-C 公共候选与发布证据

被测产品为 `e79fab3c`，树为 `80be41ee`，ABI 1.19、生成器 22、标准库接口 1。本轮 `cmath` 教程源码与独立黄金取自同一份候选，C4 将原字节登记到共享测试夹具。[B2-C 规划](../../../20o-b2-c-final-closeout-plan.md) · [B7 交接](../../20o-b2-b/entry-20261122-b7/README.md)

| 验收项 | 回执 |
| --- | --- |
| B1 七库 F10 | Linux VM 103 条，Windows、Linux、macOS 原生 O0/O3 各 206 条，共 721 条 |
| B2 F-3 | 每平台的 VM、O0、O3 各 183 项，合计 1,647 项与 8,235 个独立样本进程 |
| 语言与数值 | B1 20 入口、18 项资源用例、VM/原生精确回归、4 项实测输出与 24 项派生守卫。三平台核对 A1/A5、cmath、显式除法、浮点比较与 17,243 条 MPFR/GMP 向量 |
| 候选物 | 每平台 5 份程序、10 份 O0/O3 原生可执行文件及 10 份可移植文件。删除应用源码后重放 `.xiaoc` 与 `.xar` |
| 版本与缓存 | 当前验证器的 17 项接受/拒绝和 12 项缓存观察。生成器 22 接纳同 ABI 的 21 原生元数据，21 拒绝 22，缓存按生成器身份失配重生成 |

macOS 性能首轮保留三项失败。F10 的 `statistics.sum` O0 翻倍比 3.25487，F-3 的 VM `lfloat.to-integer` 为 3.12184，O3 `lint.power` 为 5.04523。只重跑三个失败作业，沿用相同源码与阈值。重跑原生 F10 的最大适用翻倍比为 2.96554。归档包含实际作业的 attempt、原始日志和上传对象。[性能运行](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45013071077) · [语言与候选运行](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45016234630)

Windows 的 10 份公共候选以同一次链接映射、PE 地址和导出表核对组件。Linux ELF 与 macOS Mach-O 使用产品的完整验证。Windows 产品观察为 `unverified-coff-exports`，B6 的 `windows_coff_verified=false` 按历史字段保留。B5 的 1.19/21 矩阵与 B1 的 1.18/20 重建来源分开引用。22D 本轮 Windows 通过，历史间歇性释放差分仍为开放。[B5 收口](../../20o-b2-b/entry-20261117-b5-closeout/README.md) · [B6 交接](../../20o-b2-b/entry-20261120-b6-5/README.md)

## 状态提交与工程门禁

C4 的最终状态提交为 `7753fd69`。它登记 `stdlib.cmath` 和高精度教程的已验证范围，将教程的相同源码与黄金接入精确目标，并同步既有状态断言。原有 registry 条目逐项相等。产品源码、两个 Cargo.lock、ABI、生成器和测量脚本按固定身份复核。

原工作流的失败和补验分别归档。取证分支 `3cf0c839` 仅配置运行环境并补采回执，实际执行检出 `7753fd69`。三平台原生维护、Linux 完整默认 workspace、Rust 格式与 Clippy、benchmark 和驱动自检、Bun 与 TypeScript、DevDocs、UseDocs、布局及覆盖率均通过。工作流前后同时核对 DevDocs、UseDocs、stdlib 树和锁文件。

前两轮 C4 补验保留选择导入断言、A5 旧状态断言和 Windows 取证参数的失败。最终仓库门禁 attempt 1 的 `queued_task_cancel_never_runs_body_and_wait_is_027` 在 22C 释放顺序断言失败。attempt 2 在同一固定提交上原样重跑通过，22C 继续独立追踪。[定稿维护回归](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45024200879) · [定稿仓库门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45024200820)

三平台的锁定 Cargo 解析确认 F-7 六包版本及特性。整数底座使用 `std/i128`，`libm` 启用 `arch/default`。收口回执同时核对 11 份上游许可证。

## 只读复跑与交接

`execution.zip` 用 SHA-256 保存 916 个路径引用和 653 个唯一对象。`gates.zip` 以相同方式保存工程门禁的原始运行、作业与输出。审计从固定 Git 对象和两个归档派生结论，并检查定稿清单。临时副本的反例要求拒绝文本相符及审计前后字节相等。[封存结论](closeout.json) · [文件清单](evidence.json) · [自反例回执](self-negative-result.json)

```powershell
cmd /c git fetch origin codex/b2-c-perf-runner codex/b2-c-candidate-runner
cmd /c git fetch origin codex/b2-c-gates-runner codex/b2-c-gates-final codex/b2-c-engineering
python -B -X utf8 docs/DevDocs/assets/20o-b2-c/entry-20261122-b2-c/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-c/entry-20261122-b2-c/self_negative.py
```

下一阶段为 B3 的简单宿主库规划。B1 历史原件与旧 721、22C、22D 及 19/11/22 继续按原归属记录。[B3 规划](../../../20o-b3-host-libraries-plan.md)

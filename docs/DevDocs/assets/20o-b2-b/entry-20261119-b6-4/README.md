# B6-4 三平台与 CI 工程门禁

**执行证据已归档，B6-4 验收未通过。** 产品基线为 `58176ff5`，ABI 1.19、生成器 22、标准库接口 1。正式复跑绑定 `97d5f288`。两轮执行与两轮 B1 补验的受保护产品、测试、工具及锁文件均与基线一致。B6-5 尚未启动。[B6-4 规划](../../../20o-b2-b6-4-ci-gates-plan.md) · [机器汇总](summary.json)

| 范围 | 实际结果 | 运行与作业 |
| --- | --- | --- |
| Linux 仓库门禁 | fmt、workspace Clippy、`check:lock`、`repo-check all` 通过。仓库检查报告 1,291 条既有 `A0-COVERAGE-002` 警告。research、benchmark、driver 和 Bun 门禁通过 | [44464422364 / 135358660716](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44464422364/job/135358660716) |
| Linux 默认 Rust workspace | 默认命令跳过 B1 VM 精确用例，1,758 通过、0 失败、55 忽略。随后单跑 B1 VM，1 通过、0 失败、0 忽略 | [44464422497 / 135358661595](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44464422497/job/135358661595) |
| Linux 原生维护 | `original-pow-eq-static-o0` 被产物校验拒绝。ELF 观察到 IR 未登记的 Runtime 组件 `rc` | [44464422497 / 135358661593](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44464422497/job/135358661593) |
| macOS 原生维护 | 同一用例在 Mach-O 产物校验被拒绝，原因同为 `rc` | [44464422497 / 135358661515](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44464422497/job/135358661515) |
| Windows 原生维护 | 浮点比较 62 例、248 次原生观测通过。随后 22D 的 `native-lock-wait-027` 释放顺序断言失败 | [44464422497 / 135358661620](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44464422497/job/135358661620) |

两个原生拒绝样本交 B4 LLVM Runtime 组件登记范围调查。Windows 样本交 22D，断言右侧有对象 0、1 的两组释放事件，左侧只有对象 0 的一组。B6-1 的 `restricted`、Windows COFF 导出完整性未验证、Linux Clippy 未执行属于前序阶段。B6-2 性能绑定 `79ea0375`，B6-3 的边界样本和组件等级绑定 `58176ff5`。B6-4 Linux Clippy 已实际通过。

## 运行与回放

首轮 `5758aa51` 推送命中两条工作流的路径过滤。Linux 没有为默认测试准备组件诊断所需的 LLVM 和 Runtime，三平台原生作业也未提供浮点比较要求的独占输出路径。复跑 `97d5f288` 补齐这些环境，原测试命令、跳过列表和时限沿用。两条复跑由手动派发，维护运行的 `b1_f10` 和 `b2_a_perf` 均为 false。仓库门禁命中 Bun 缓存，四个主要维护作业的 Rust 缓存均未命中。[首轮仓库门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44460712945) · [首轮维护回归](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44460712977)

维护作业原生阶段失败后，Linux、macOS 两条 B1 精确命令由独立 Runner 补跑。首次补验使用了 Runner 默认的 Rust 1.100.0 和 1.99.1，保留为历史记录。第二次补验显式选择 Rust 1.96.0，两项均通过。各运行的尝试编号均为 1，原生维护和补验各有独立的运行与作业编号。[首次补验](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44467531829) · [固定工具链补验](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44470256490)

Linux 默认维护作业通过了 28 项模拟输入守卫。另从同一作业的 Linux VM、Windows 维护作业，以及固定工具链补验的 Linux 和 macOS 作业提取四段实际 Cargo 输出。固定维护工作流的计数逻辑接受这四段输出，并按预期拒绝其 24 项派生负例。VM 核对九个夹具，三平台原生分别核对 O0 和 O3 的九个夹具标记。[守卫来源与回放汇总](summary.json)

## 只读复核

原始归档保存 6 次运行、18 个作业的 GitHub API 回执与日志，以及 28 次守卫回放的输入和进程回执。包内 `evidence.json` 将 198 个逻辑路径映射到 100 个 SHA-256 对象。作业回执记录 DevDocs 检出树、锁文件字节、Runtime、工具及适用的诊断组件摘要。[原始归档](ci.zip)

三平台临时测试可执行文件的字节身份为 `unverifiable`。原生日志保留用例结果与部分产物路径。维护复跑和固定工具链的 B1 补验均无上传产物。[维护复跑](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44464422497) · [固定工具链补验](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44470256490)

在仓库根运行复核。新检出先获取两个 Runner 分支的固定对象。审计逐字节对照 `0af1575b` 封存的 CI 归档与 `e154b9e7` 封存的文档门禁归档，并核对涉及 Runtime 的作业的 Runner 平台、目标三元组和构建前后记录。结果中的 `verified` 表示证据对账通过。阶段结论为 `b6_4_passed=false`。

```powershell
cmd /c git fetch origin codex/b6-4-gates-runner codex/b6-4-b1-runner
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261119-b6-4/evidence.py
```

可按 `gh api repos/Programming-Language-Xiao/Xiao/actions/runs/<run>/attempts/1` 和同路径下的 `jobs` 下载运行及作业回执，再按 `gh api repos/Programming-Language-Xiao/Xiao/actions/jobs/<job>/logs` 下载作业日志。归档里的 `ci/<run>/` 保留这三类响应的文件映射。文档门禁于 `0af1575b` 执行，回执包含命令、PID、原始输出和当时八份 DevDocs 文档的原始字节及 UTF-8/LF 摘要。[定稿门禁回执](docs-check.zip)

自审复跑了原样归档及七项反例，覆盖对象缺失、尝试编号与日志改写、构建前已有 Runtime、文档门禁执行提交、原始字节摘要与进程回执不符。复核逐项核对退出码、实际拒绝文本和输入字节。[首轮反例](negative-cases.json) · [本轮反例](review-negative-cases.json)

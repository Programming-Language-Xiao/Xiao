# B6-3 错误与资源边界验收

三平台边界验收通过。受测产品固定为 `58176ff5299441f921f3b54fbdc300e2e46b9bb0`，ABI 1.19、生成器 22、标准库接口 1。各任务独立构建 release Runtime 和核心，按同一份矩阵生成 Xiao 源码。矩阵登记在 `682beb6f`，SHA-256 为 `f6d7ac2a4e2098f55deb176448bcdcfddd7320a7e9570f448bb758181aa46d6e`。[固定矩阵](matrix.json)、[验收回执](summary.json)

| 平台 | VM / 原生 O0 / 原生 O3 | Runtime 通过 / 失败 / 忽略 | C ABI | 原生组件验证 |
| --- | --- | --- | --- | --- |
| Windows MSVC | 各 85/85 | 305 / 0 / 1 | 11/11 | `UnverifiedCoffExports` |
| Linux x86_64（WSL） | 各 85/85 | 305 / 0 / 1 | 11/11 | ELF `Complete` |
| macOS ARM64 | 各 85/85 | 305 / 0 / 1 | 11/11 | Mach-O `Complete` |

共 765 次语言执行，按黄金输出、错误身份、源码字节区间和适用的诊断细节逐项核对。非有限实参由既有 Runtime、VM 注入和原生位模式入口覆盖。每平台重跑 17,243 条正式参考，原生共享幂路径另核对 1,075 条。C 探针记录状态 0/1/2/3/6、输出槽和挂起槽，另由独立进程验证 Fatal 状态 8 和进程退出 4。受控分配失败 011 由既有故障注入验证。[原始回执与日志](boundaries.zip)

三平台 B1 的 20 个产品入口、18 项资源、VM 和原生两条精确入口均通过。精确入口覆盖七库及别名、字符串索引两个辅助夹具。B4 包装两个别名分别通过 O0/O3。维护工作流计数守卫在 Windows 回放通过 28/28，使用四条实际 Cargo 输出和 24 条派生负例。

## 执行记录

macOS 最终五项作业均通过。首轮回归重建了 Cargo 输出目录内的核心，触发产物身份检查。将 release 核心固定到独立目录后，第二轮的回归采集器漏计了 B1 的两个辅助夹具。Windows 和 Linux 的原始汇总也各保留两项同源误判。两条实际 Cargo 命令均退出 0，原始日志各有一项通过、零失败、零忽略。最终审计按夹具名和优化等级复算完整标记。[首轮 macOS](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44279794451)、[第二轮 macOS](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44281869728)、[最终 macOS](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44284131624)

本地预检的采集器导入错误、产物身份变化和命名实参用例的试跑回执一并归档。该用例的正式黄金按已冻结的 `finally → drop → catch` 展开顺序核对。

证据包内的 `evidence.json` 将原始路径映射到 SHA-256，包含 16,514 条记录和 7,041 个对象。原生执行文件、LLVM 文本、Rust 与 LLVM 工具和平台 Runtime 以实际摘要登记。进程回执记录原始内存单位。B6-3 验收资源接纳和错误契约，B6-2 性能结果仍绑定 `79ea0375`。

`repo-check.zip` 保存 `55c818fd` 收口时的完整仓库门禁回执和原始输出。回执记录 17 个文件的运行前后 SHA-256，文本亦记录 UTF-8/LF 摘要。审计用固定 Git 对象复核这些文件。自审定稿门禁另见 `review/repo-check.json`。[原门禁归档](repo-check.zip)

## 复核与交接

在仓库根运行只读审计。新检出若缺少远端 Runner 的 Git 对象，先获取 `codex/b6-boundary-runner` 分支。

```powershell
cmd /c git fetch origin codex/b6-boundary-runner
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-3/audit.py
```

同一审计也核对 `--archive` 和 `--repo-check` 指定的归档副本。原样副本通过，八个反例逐项核对实际异常文本、退出码和只读性。自审核对照固定汇总、门禁文件清单、构建目标、独占原生产物路径、F-12 消息标识和计数守卫的实际拒绝原因。[反例回执](review/negative-cases.json)

```text
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-3/review/self_negative.py
```

macOS 自审核重新下载最终运行的五份 ZIP。ZIP 摘要与 GitHub API 一致，包内 2,306 个文件与 `boundaries.zip` 逐字节对账。原件可用 `gh api repos/Programming-Language-Xiao/Xiao/actions/artifacts/<产物 ID>/zip` 下载。按 `<产物 ID>.zip` 命名后，可用审计的 `--macos-artifacts` 参数复核。[产物 ID 与摘要](review/macos-artifacts.json)

复跑使用新任务根与本机 LLVM 工具目录。Windows 和 Linux 的入口都使用冻结的 `58176ff5` 重建。

```text
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-3/prepare.py --task <新任务根> --llvm-bin <LLVM工具目录>
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-3/run.py --task <同一任务根> --mode all
```

B6-1 保持 `restricted/stage_passed=false`。三平台功能通过，Windows COFF 导出完整性未验证，Linux Clippy 未执行。B6-4 以本批源码身份、命令、原始日志和逐平台回执为入口，执行其工程门禁。[B6-4 入口](../../../20o-b2-b6-resource-performance-plan.md#b6-4三平台与-ci-证据)

# B7 内部收口与 B2-C 交接

最终产品为 `e79fab3c`，树 `80be41ee`，ABI 1.19、生成器 22、标准库接口 1。证据核对 1,222 个固定产品路径、两个 Cargo 锁文件、各执行点的 Runtime 与工具身份并复跑 30 个前序只读审计入口。B1 的 1.18/20 产物由 B5 按裁定重建，历史原件缺失。B6-2 的性能实测绑定 `79ea0375`，B6-3 的完整边界实测绑定 `58176ff5`。[B7 规划](../../../20o-b2-b7-closeout-plan.md)

## 回归结果

| 项目 | Windows | Linux | macOS |
| --- | --- | --- | --- |
| B1 契约和资源 | 9/9、18/18 | 9/9、18/18 | 9/9、18/18 |
| B1 VM、原生精确入口 | 各 1/0/0 | 各 1/0/0 | 各 1/0/0 |
| 新产物与黄金 | 七库 14 个原生产物、14 个可移植产物 | 七库 14 个原生产物、14 个可移植产物 | 七库 14 个原生产物、14 个可移植产物 |
| 完整默认 workspace，192 个目标 | 1,758/0/55 | 1,760/0/56 | 1,759/0/56 |
| workspace Clippy 和 fmt | 通过 | 通过 | 通过 |
| 数值、浮点与正式参考 | A1、A5、显式除法、64 位 Cmath、浮点比较和 17,243 条正式参考通过 | 同左 | 同左 |

B1 契约目标按固定表执行 20 项公开入口，覆盖双路径、导出、移除、普通包隔离与教程。原生精确目标核对七库及两个辅助夹具的 O0/O3 黄金。该目标会清理自己的七库可执行文件，因此另外从同一固定产品运行普通原生构建并登记七库 14 个实际执行文件摘要。可移植补采保留七库新生成的 `.xiaoc`、`.xar` 和删除应用源码后的回放结果。

计数守卫用固定工作流的四段判断回放，含本批四条真实 Cargo 输出及 24 条派生负例。22D 本机 Windows 与远端 Windows 的完整差分本轮通过，三个历史失败点仍为 `22d_open=true`，归 B7 继续追踪。

## CI 与工程门禁

原样检出的 [维护回归 44996207048](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44996207048) 和 [仓库门禁 44996207236](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44996207236) 失败。Linux 默认测试和仓库门禁缺 LLVM 与 Runtime 环境，三平台原生任务缺浮点测试的独占输出目录。

取证分支 `4e009503` 复用 B6-4 已运行的环境配置。[三平台维护回归](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44998495938)、[Linux 仓库门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44998495850)和 [macOS 三项专项](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44998495908)均通过，均为 attempt 1。工作流变动限于取证分支。Rust 1.96.0 实际执行，原始测试、预算和作业时限沿用冻结值。

macOS 在 `95c0f5b4` 对固定产品再取七库的原生产物与删源回放身份。该次专项与随推送触发的 Linux 仓库门禁均在 attempt 1 通过。B7 最终文档门禁另在 Linux 固定文档提交上执行，校验 DevDocs、UseDocs 的 Markdown 路径和字节，以及两项锁文件。回执保存间接 Cargo 实参、退出码、工具身份和原始输出。[macOS 产物补采](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45003168952) · [随推送执行的仓库门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45003168769)

## 交接与复跑

B6-5 的受限字段继续有效。Windows COFF 为 `UnverifiedCoffExports`，最终树 Windows/macOS 的公共产物验证、B6 新增数值负载、完整 721 条 F10 和最终教程及登记由 B2-C 接收。B6-1 的 Linux Clippy 历史缺项按原状引用，B7 Linux Clippy 另有新回执。四份语言/索引说明仅描述已验证的内部范围，`cmath` 及新增范围的公共发布仍为 planned。[机器交接清单](handoff.json) · [B2-C 规划](../../../20o-b2-c-final-closeout-plan.md)

证据归档以 SHA-256 引用原始字节。默认审计只读核对封存清单、固定 Git 对象、命令回执和阶段合取。收口输出以 `closeout.json` 为准。新检出若缺 CI 取证提交，可从对应取证分支取回 Git 对象。

```powershell
cmd /c git fetch origin codex/b7-evidence-runner codex/b7-artifacts-runner
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261122-b7/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261122-b7/self_negative.py
```

# B3-4E CI 平台回执

GitHub Actions 的两套门禁在 `18e7f4fa` 上通过。macOS `macos-15-arm64` 的
`aarch64-apple-darwin` 作业实际执行了 VM 教程、原生 O0/O3 教程、env 原生专项和
五批性能；Windows 与 Linux 的 env 原生专项、运行控制及原生教程也通过。

## 运行身份

| 工作流 | run / 尝试 | 结果 | 作业 |
| --- | --- | --- | --- |
| Xiao workspace gates | [46052336364](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/46052336364) / 1 | 成功 | Linux `140686475963` |
| Xiao maintenance regression | [46093774527](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/46093774527) / 2 | 成功 | Linux native `140827969917`、Linux default `140827943950`、Windows `140827943054`、macOS `140827942402`、Windows size `140827971690`、parser `140827968358` |

受测提交为 `18e7f4fa`。B3-4D/1D 的本地证据绑定 `1e42f501`；`939f2c34`/`87bfa31f` 与
`18e7f4fa` 只涉及门禁文档、clippy 和 CI 入口，不改变 env 语义。
[attempt 2 元数据](logs/1e-ci/maintenance-run-attempt2.json) ·
[workspace 元数据](logs/1e-ci/workspace-run.json)

## 平台结果

| 平台 | 镜像 / 目标 | env 原生专项 | VM 教程 | 原生教程 | 运行控制 |
| --- | --- | ---: | ---: | ---: | --- |
| Linux | `ubuntu-24.04` / `x86_64-unknown-linux-gnu` | 8 通过 | 1 通过 | 1 通过 | 8 通过，含 001/002/027 |
| Windows | `windows-2025` / `x86_64-pc-windows-msvc` | 8 通过 | 1 通过 | 1 通过 | 8 通过，含 001/002/027 |
| macOS | `macos-15-arm64` / `aarch64-apple-darwin` | 8 通过 | 1 通过 | 1 通过 | 8 通过，含 001/002/027 |

macOS 的 `denied-o0/o3` Mach-O 产物验证为 `complete`，未分类 Runtime 符号为空，
运行期退出 3、诊断 `X06-RUNTIME-026`。[macOS 原始日志](logs/1e-ci/macos-maintenance-attempt2.log) ·
[Windows 原始日志](logs/1e-ci/windows-maintenance.log) · [Linux 原始日志](logs/1e-ci/linux-maintenance.log)

## macOS 五批性能

`b3_env_performance` 以普通发布二进制运行，identity 绑定 `18e7f4fa` 和
`aarch64-apple-darwin`。42/42 组合通过，最大中位数 VM `1.273 ms`、原生 O0 `0.287 ms`、
原生 O3 `0.301 ms`，峰值增量 `20.562 MiB`，最大翻倍比 `2.763`；均在冻结阈值内。
[性能汇总](logs/1e-ci/macos-performance-attempt2/performance.json) ·
[identity](logs/1e-ci/macos-performance-attempt2/identity.json)

首次 attempt 的 `native-o0-snapshot-1024` 翻倍比为 `3.3616 > 3`，作业失败；保留失败
对照并重跑 attempt 2 后 42/42 全通过。[失败对照](logs/1e-ci/macos-performance-attempt1/performance.json) ·
[attempt 1 日志](logs/1e-ci/macos-maintenance.log)

## 门禁

- `workspace gates` 通过；`maintenance regression` attempt 2 的六个作业全部成功。
- 首次 `fdb929ba` 的 workspace gates 失败项已由 `939f2c34` 修复；debug 构建的
  `E0080` 已由 `87bfa31f` 修复。
- macOS VM 教程、原生 O0/O3、五批性能均已有 CI 回执。Windows 六份 COFF 仍由
  B3-4D 本地证据绑定，不声称 macOS 有 COFF 产物。
[workspace gates 原始日志](logs/1e-ci/workspace-gates.log)

## 结论

B3-4E 所需的三平台 env 功能与 macOS 五批性能已有 CI 回执。B3-4A～E 的本地和 CI
证据齐备，可进入最终状态同步与 B3-5 入口裁定。

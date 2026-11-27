# B3-4E CI 平台回执

GitHub Actions 的两套门禁都在 `87bfa31f` 上通过。macOS `macos-15-arm64` 的
`aarch64-apple-darwin` 作业实际执行了 env 原生 O0/O3 专项、原生教程和运行控制专项；
VM 教程与五批性能未在该作业执行，仍按缺项记录。

## 运行身份

| 工作流 | run | 结果 | 作业 |
| --- | ---: | --- | --- |
| Xiao workspace gates | [46052336364](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/46052336364) | 成功 | Linux `140686475963` |
| Xiao maintenance regression | [46052285074](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/46052285074) | 成功 | Linux `140686336461`、Windows `140686336674`、macOS `140686336783`、Windows size `140686336755`、parser `140686336788`、Linux default `140686336841` |

受测提交为 `87bfa31f`。B3-4D/1D 的本地证据绑定 `1e42f501`；两提交之间只有
`939f2c34` 的文档/布局/clippy 门禁修复和 `87bfa31f` 的性能入口 debug 断言改动，
不改变 env 语义。[maintenance 元数据](logs/1e-ci/maintenance-run.json) ·
[workspace 元数据](logs/1e-ci/workspace-run.json)

## 平台结果

| 平台 | 作业镜像 | env 专项 | 原生教程 | 运行控制 |
| --- | --- | --- | --- | --- |
| Linux | `ubuntu-24.04` | 8 通过、0 失败 | 1 通过 | 8 通过，含 001/002/027 |
| Windows | `windows-2025` | 8 通过、0 失败 | 1 通过 | 8 通过，含 001/002/027 |
| macOS | `macos-15-arm64` / `aarch64-apple-darwin` | 8 通过、0 失败 | 1 通过 | 8 通过，含 001/002/027 |

macOS 的 `denied-o0/o3` Mach-O 产物由驱动验证为 `complete`，未分类 Runtime 符号为空，
运行期退出 3、诊断 `X06-RUNTIME-026`。macOS 未执行 VM 教程和 `b3_env_performance`
五批采样，故不能把三平台 VM/性能矩阵写成完整通过。[macOS 原始日志](logs/1e-ci/macos-maintenance.log) ·
[Windows 原始日志](logs/1e-ci/windows-maintenance.log) · [Linux 原始日志](logs/1e-ci/linux-maintenance.log)

## 门禁

- `workspace gates` 的 Linux 作业通过；`maintenance regression` 的六个作业全部成功。
- 首次 `fdb929ba` 的 workspace gates 失败项已由 `939f2c34` 修复；首次 debug 构建的
  `E0080` 已由 `87bfa31f` 修复。
- 本批 CI 不证明 macOS VM 教程、macOS 五批性能或 macOS 六份 COFF；这些仍为缺项。
[workspace gates 原始日志](logs/1e-ci/workspace-gates.log)

## 结论

Windows、Linux、macOS 的 env 原生专项、教程原生入口和运行控制已在 CI 上通过。
B3-4E 仍缺 macOS VM 教程和 macOS 性能五批，不能置 `b3_4_passed=true`；需要新增
macOS 精确入口或按受限交接记录。

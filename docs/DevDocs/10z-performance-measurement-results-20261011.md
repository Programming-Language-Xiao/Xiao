# 10Z Linux 受控性能取数结果（2026-10-11）

本记录对应 [10Z 性能测量交接](10z-performance-measurement-handoff.md)，实际检出提交为 `07d29a67704d14972e6e22cff635a3b2ff20e5f3`。

本次使用 Ubuntu 26.04.1 x86_64 裸机、Ubuntu OpenJDK 21.0.12、Clang 21 和 Rust 1.96.0。Java 21 通过本地解压的 apt 包提供，未修改系统安装。

## 判读

- 顶层 `status`：`development-evidence`；
- `summary.total_cases / measured_cases / data_insufficient_cases`：`5 / 4 / 1`；
- `bootstrap_determinism.byte_identical`：`True`；
- 协议：预热 3、测量 11、bootstrap 10000、seed 19015、95% percentile-bootstrap；
- 本轮没有冻结阈值，也没有写达标或回归结论。

## 各基准

| 基准 | 状态 | Java median ns | Native median ns | VM median ns | Native/Java | VM/Java | 备注 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| `deep-expression-arithmetic` | `measured` | 31819627 | 1533467 | 4709814 | 0.0482 | 0.1480 | — |
| `scalar-overflow-and-bool-parity` | `data-insufficient` | — | — | — | — | — | java 返回 success=1983905792，但清单要求 error=X06-RUNTIME-009 |
| `named-local-loop` | `measured` | 31784110 | 2961335 | 4184843 | 0.0932 | 0.1317 | — |
| `deep-call-recursion` | `measured` | 32001508 | 761371 | 1124169 | 0.0238 | 0.0351 | — |
| `container-dense` | `measured` | 29518682 | 1982841 | 9213478 | 0.0672 | 0.3121 | — |

## 原始附件

- [驱动器报告 JSON](assets/10z-performance-20261011/performance-report.json)
- [完整驱动器日志](assets/10z-performance-20261011/performance-driver.log)
- [人工环境清单](assets/10z-performance-20261011/environment.txt)

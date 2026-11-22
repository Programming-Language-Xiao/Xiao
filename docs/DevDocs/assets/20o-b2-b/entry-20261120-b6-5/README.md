# B6-5 内部收口与交接

最终产品绑定 `e79fab3c`，核对 1,222 个受保护路径及两个 Cargo 锁文件。版本为 ABI 1.19、生成器 22、标准库接口 1，活动 intrinsic 为 1～48，ID 100 为墓碑，`stdlib.cmath` 仍为 planned。受保护树与 `780398e5` 一致。最终工程门禁及证据对账完成后，`b6_5_passed=true`、`b7_entry_ready=true`。B6 整体验收维持 `b6_accepted=false`。B7 与 B2-C 的执行另批开始。[实施规划](../../../20o-b2-b6-5-closeout-plan.md) · [机器可读交接](handoff.json)

## 阶段证据

| 阶段 | 复核结论与适用范围 |
| --- | --- |
| B5 | 版本、缓存和 C ABI 矩阵固定在 1.19/21，静态浮点 21→22 增量、URL 优化与 1,754/0/55 工作区回执分别核对。[联合收口](../entry-20261117-b5-closeout/README.md) |
| B6-1 | `restricted/stage_passed=false`。Windows、Linux 与 macOS 的功能验证齐备。Windows COFF 为 `UnverifiedCoffExports`，Linux Clippy 在 B6-1 当时未执行。macOS 补验使用固定源码 `ef456f17`，native/resources 两个作业通过。[限定重跑](../entry-20261118-b6-1-rerun/README.md) · [macOS 专项](../entry-20261118-b6-1-macos/README.md) |
| B6-2 | `perf_pass=true` 绑定 `79ea0375`。三平台 VM、原生 O0/O3 共九组，各 183 项，共 8,235 个测量进程。原审计核对黄金、全转义 URL 最大负载、时间、驻留增量及适用翻倍比。[实测](../entry-20261118-b6-2/README.md) · [源码及反例](../entry-20261118-b6-2/review/README.md) |
| B6-3 | `boundary_passed=true` 绑定 `58176ff5`。85 项跨平台和后端执行 765 次，C ABI 每平台 11 项，B1/B4 回归及 28 项守卫均通过。守卫含 4 条实际输出和 24 条派生负例。Windows COFF 导出完整性未验证。[边界归档](../entry-20261118-b6-3/README.md) |
| B6-4 | `b6_4_passed=false`、`original_passed=false`、`followup_passed=false`、`windows_coff_verified=false`、`22d_open=true`。既有后续功能复验、本机 Linux 164 导出与 Windows 定向 Clippy 各自保留执行快照。[原批](../entry-20261119-b6-4/README.md) · [修复和补验](../entry-20261119-b6-4-followup/README.md) |

## 最终树覆盖

归档 `identity/product.json` 保存两份逐文件差异，含变更前后 Git blob。测量入口和负载新增的 Rustdoc、VM 加载器原样移动，由 B6-2 源码复核证明。从 `58176ff5` 到最终树的四处受保护差异是边界采集工作流、组件登记、Linux 导出对账测试及浮点比较的组件观测。组件登记影响优化报告和普通原生产物构建。性能入口读取 LLVM 文本并用 Clang 直接链接，组件登记列表未进入编译参数或测量区间。数值 Runtime、生成的 LLVM 路径及负载语义在这段差异内一致。因此 `b6_2_final_tree_covered=true` 表示源码路径覆盖。正式测量仍绑定 `79ea0375`，B6-5 新增测量样本为 0。

普通原生构建使用组件登记选择 COFF 导出参数并核对产物。B6-5 在最终树上补验 Linux ELF。组件登记相关的 Windows、macOS 最终树上产物验证交 B2-C，故 `b6_3_final_tree_covered=false`。

## 最终 Linux 工程门禁

| 项 | 实测 |
| --- | --- |
| 默认 workspace `--locked` | 192 个测试结果小结，1,760 通过、0 失败、56 项原有忽略 |
| workspace Clippy `--all-targets --locked -- -D warnings`、fmt | 均退出 0。此处是当前 Linux 的新 Clippy 回执 |
| `bun run check:lock` | 退出 0。既有受控 Cargo 入口为两条间接 `cargo check` 带上 `--locked` |
| `repo-check all` | 原批首跑缺 Cargo 路径，修正后退出 0。这两次回执保存在 `closeout.zip`。自审补验在同一产品树上锁定间接执行的 `cargo metadata` 和 `cargo run`，另存各自的参数、进程标识与退出码 |
| Linux 定向补验 | release Runtime 的 164 个导出通过；cmath 两别名的 O0/O3 组件为 `Complete`，黄金和 009 通过；显式除法 O0/O3、回调三目标 IR、原生动态字符串通过。浮点比较 62 个 VM、248 次原生、248 次汇编，静态幂 eq/ne 四格组件均为 `{rc, value}` |
| 功能公开面 | 最终 `xiao-core` 的两种 cmath 别名正常执行。两个 Rust 加载器 API 的六个裸调用和导入样本按既定诊断拒绝 |

本批使用隔离的 Linux x86_64 WSL2 源码、target、Runtime、缓存和输出目录，工具为 Rust 1.96.0、LLVM 21.1.8、Bun 1.4.2。归档记录工具和新构建 Runtime 的 SHA-256、进程 PID、锁及源码前后身份、原始输出。B6-5 自身证据来自本机进程，历史 CI 见各自回执。

## 待办归属

| 项 | 归属 |
| --- | --- |
| Windows 22D 的 `native-lock-wait-027`、`xcon-bounded-string-success`、`spawn-join-str-multiargument` 间歇性释放差分 | B7 继续追踪，保留原失败轨迹和条件。[22D 记录](../../../22d-llvm-concurrency-differential.md#十一后续维护回归) |
| Windows COFF 完整性及 Windows/macOS 最终树上的原生产物验证 | B2-C 补验，逐平台保存组件声明和实际观察 |
| B6-1 Linux Clippy 历史 `not_run` | B7-4 保留历史缺项并登记最终 Clippy 状态 |

B7 从源码重生成 B1 20 入口，运行资源 18 项、双精确入口及 28 项守卫，并对账 A0～A7 与 B0～B6。B2-C 在最终新快照跑完整 721 条 F10，另验 B6 数值负载、Windows/Linux/macOS 公共产物及完整教程示例。教程和 registry 仅在对应证据齐备后升级。[B7 规划](../../../20o-b2-b7-closeout-plan.md) · [B2-C 清单](../../../20o-b2-c-final-closeout-plan.md)

## 只读复核

`closeout.zip` 绑定收口提交 `69d009e8`。归档的 `evidence.json` 按逻辑路径引用 SHA-256 对象，含 11 个前序只读回放。仓库门禁记录完整 DevDocs Markdown 树摘要，逐条比对本批文档和交接清单在固定 Git 树中的路径与字节。`status.json` 由归档派生，`review.json` 保存状态组合和反例。审计使用固定 Git 对象，当前 HEAD 和本机 Runtime 均可变化。

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261120-b6-5/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261120-b6-5/self_negative.py
```

自反例枚举 2,048 组布尔输入，覆盖伪造通过状态、数字代替布尔值、对象缺失、重算清单后的导出计数错误及重打包后同步更新状态中的摘要。审计核对退出码、拒绝原因和原件字节。

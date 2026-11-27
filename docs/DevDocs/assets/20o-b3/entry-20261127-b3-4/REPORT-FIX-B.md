# B3-4C-FIX-B 修复报告

工作区基于 `946513ac`，前一产品基线为 `00766ca3`。本批证据绑定工作区源码摘要、两份 Cargo 锁文件和教程夹具。受测组合为 Windows 11 10.0.26200、`x86_64-pc-windows-msvc`、Rust 1.96.0、LLVM 22.1.2、MSVC 链接器 14.51.36256.0 和 Windows SDK 10.0.26100.0。发布 Runtime、ABI 1.20、生成器 27、标准库接口 1 和各探针的文件摘要均有回执。[源码身份](logs/fix-b/source-identity.json) · [工具链](logs/fix-b/tools.log) · [原生进程与 Runtime](logs/fix-b/native-processes.json)

## FIX-B1 有界注入

- 标准输入握手与 `xiao_runtime_env_run_injected` 共用 `MAX_INJECTED_BYTES` 和同一 JSON 接纳器。原始上限为 6,299,650 字节，覆盖 1 MiB UTF-8 内容全部按六字节转义及 1,024 组的数组结构。C ABI 先校验能力，再检查载荷长度和 `isize` 上限，长度合格后才处理空指针并借用输入。标准输入逐块检查剩余容量，超限块在扩容和解码前拒绝。
- 解码器逐对验证名称、值、去重和配额，将合格项直接写入尚未发布的视图。达到第一处逻辑超限即停止。结构和尾部校验完成后才交付快照。原始载荷及快照逻辑超限均报 `X09-DRIVER-007`、退出 2、零用户输出。Runtime 和驱动回归覆盖恰好 1 MiB 的转义快照、恰好 6,299,650 字节的原始载荷、原始上限加一字节、逻辑上限加一字节，以及 C ABI 超长和空指针。
- 标准输入协议以 EOF 定界，因此要暂存至多 6,299,650 字节的已读前缀。C ABI 直接借用调用方缓冲。解码已接纳的字符串最多 1 MiB 和 1,024 个条目，加上当前一对字符串及解析器重用的暂存区。以原始上限为 `R`，结构有效的输入按 Rust 1.96.0 的缓冲增长机制计算，原始缓冲、单对解码和暂存重分配的合计上界为 `5R`，另加 1 MiB 快照与 96 KiB 条目数组及其重分配空间，合计小于 32 MiB。畸形类型的 Serde 临时错误字符串可能转义输入，按 `40R + 1 MiB + 96 KiB` 给出宽上界。这里计缓冲容量，不计分配器开销或调用方持有的输入，未测 RSS。[Runtime 原始日志](logs/fix-b/runtime.log) · [原生注入回执](logs/fix-b/driver-env-native-retry.log)

## FIX-B2 原生运行控制

- `run_application` 消费 `RunOptions.timeout_ms`。可选的 `run_application_with_control` 收到 `RunControl` 时合并截止，取较早的期限。驱动在调用入口、构建前、构建返回后及准备完成后的 `spawn` 前检查取消和截止。执行前返回 001 或 002、退出 2、标准输出为空。O0/O3 实测预取消和零超时都未产出可执行文件，构建完成后的参数迭代触发取消也未启动用户代码。
- 原生启动适配器借助原有随机数校验接纳私有控制字段。截止基于同一宿主的单调时钟，包含构建、启动及快照接纳所用时间。运行取消由父进程轮询令牌，在独占临时目录中写入通知。子进程在用户代码入口和 env 安全点检查控制。无显式快照时仍由子进程捕获一次宿主环境，私有字段从本次 env 快照中滤除。O0/O3 实测截止与取消都由用户代码中的 `get` 到达 Runtime，诊断分别为 `X09-DRIVER-002` 与 `X09-DRIVER-001`，退出均为 2。
- `get`、`set`、`unset` 在类型、授权、参数及预算后检查运行控制。运行级中断沿既有不可捕获 C ABI 通道传播，状态 8、退出 2。独立任务取消实测仍为 `X06-RUNTIME-027`，退出 3。Runtime 单测逐次核对失败后的旧值、条目数、累计字节及输出槽。链接相同 release Runtime 的 C 探针在 O0/O3 分别对 `set` 和 `unset` 观察 001 与 002。探针先读旧值，再用第 1,025 项插入、累计字节增加一和增加二验证提交后视图仍为 1,024 项、1 MiB 减一字节。增加一进入安全点并收到中断，增加二在预算检查报 `ENV_LIMIT_EXCEEDED`。[原生专项日志](logs/fix-b/driver-env-native-retry.log) · [逐用例退出和原始输出](logs/fix-b/native-processes.json)

## FIX-B3 生产性能身份

- 性能入口迁到普通二进制 `b3_env_performance`。发布构建只解析普通依赖。程序在处理 `--help`、控制端或 VM 子进程参数前，断言实际链接的 Runtime 关闭 `test-failpoints`。Cargo 的编译产物事件显示 Runtime 特性为 `[]`、优化级别 3、调试断言关闭。帮助命令通过同一条链接身份断言。现有负载、采样阈值、五批协议和身份字段随入口迁移。[入口源码](../../../../../core/rust/crates/xiao-driver/src/bin/b3_env_performance.rs) · [链接身份](logs/fix-b/performance-identity.json)

后续五批复跑使用下列入口，身份文件和两个任务目录沿用 B3-4E 协议。

```text
cargo +1.96.0 run --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --release --bin b3_env_performance -- --output-dir DIR --artifact-dir DIR --identity JSON
```

## 验证记录

| 实际命令 | 退出码 | 结果与原始日志 |
| --- | ---: | --- |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-runtime` | 0 | 340 项通过，1 项忽略。[日志](logs/fix-b/runtime.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env -- --nocapture` | 101 → 0 | 首次编译在合并截止时提前移动 `RunControl`，改为先计算时长后重试。最终 7 项通过、8 项原生用例另行执行。[首次](logs/fix-b/driver-env.log) · [最终](logs/fix-b/driver-env-verified.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env -- --ignored --nocapture --test-threads=1` | 0 → 101 → 0 | 初步验证通过。加严后的第二次运行，O3 探针在快照接纳阶段达到 900 ms 截止，只收到入口拒绝。重试给截止探针 3 秒接纳窗口，取消探针在读到旧值后同步发出通知。最终 8 项原生用例通过。[初测](logs/fix-b/driver-env-native.log) · [失败](logs/fix-b/driver-env-native-final.log) · [重试](logs/fix-b/driver-env-native-retry.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env_tutorial -- --nocapture` | 0 | VM 教程 1 项通过，双导入逐字核对；原生教程标记为忽略。[日志](logs/fix-b/driver-tutorial.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml --workspace` | 0 | 197 组，1,826 项通过、64 项忽略。[日志](logs/fix-b/workspace.log) |
| `cargo +1.96.0 fmt --manifest-path core/rust/Cargo.toml --all -- --check` | 0 | [日志](logs/fix-b/fmt-verified.log) |
| `cmd /d /c npm --prefix cli/ts test` | 1 → 0 | 首次缺 `string-width`，并继承了原生专项的 Runtime 覆盖路径。执行 `bun install --frozen-lockfile`（退出 0）后，在常规 CLI 环境重试，258 项通过、5 项跳过。[首次](logs/fix-b/npm-test.log) · [依赖安装](logs/fix-b/bun-install.log) · [重试](logs/fix-b/npm-test-retry.log) |
| `cargo +1.96.0 build --locked --manifest-path core/rust/Cargo.toml -p xiao-runtime --release` | 0 | 原生专项链接该静态库。[构建](logs/fix-b/runtime-release-final.log) · [摘要](logs/fix-b/native-processes.json) |
| `cargo +1.96.0 build --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --release --bin b3_env_performance --message-format=json` | 0 | [编译事件](logs/fix-b/perf-identity-build-final.log) |
| `cargo +1.96.0 run --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --release --bin b3_env_performance -- --help` | 0 | [启动断言与帮助](logs/fix-b/perf-identity-help-final.log) |

## 限制与复跑建议

- 构建工具继续完成已开始的编译或链接，驱动在调用返回时检查控制。原生协作式运行控制覆盖通过私有握手启动的 env 入口及继承同一运行上下文的任务。独立 C ABI 嵌入入口沿用自身快照，外部嵌入宿主的运行令牌缺少交付路径。未授予 env 的产物及不经过 env 安全点的代码由父进程监督。父进程给直接子进程 250 ms 协作退出窗口，随后可强制终止。该兜底跳过语言清理，子孙进程树和非 env 调用内部的协作中断仍未覆盖。受控模式通过临时文件采集输出，直接子进程退出后，持有继承句柄的进程仍可能写入本次回执截取点之后的内容。
- B3-4D 复跑需用本批产品重新构建各目标的发布 Runtime 并静态重链。Windows、Linux、macOS 应重验教程双导入、原生 O0/O3 和新增控制专项。Windows 的四份教程产物与两份权限产物需逐件重新绑定可执行文件、COFF 链接证据和验证回执。权限负例预期为 026、退出 3。历史 `00766ca3` 的六份 COFF 回执仍对应历史二进制。[B3-4D 历史证据](REPORT-1C-RERUN.md)
- B3-4E 复跑应使用本报告的发布二进制入口和固定运行身份，重新记录 VM 与原生 O0/O3 各五批及各平台证据。旧 VM 样本链接了启用 `test-failpoints` 的 Runtime，原生快照样本未包含本批解码改动。本批仅实测 Windows 本地功能和性能入口身份，Linux、macOS、CI 和新五批性能仍待复跑。[B3-4E 历史证据](REPORT-1D.md)

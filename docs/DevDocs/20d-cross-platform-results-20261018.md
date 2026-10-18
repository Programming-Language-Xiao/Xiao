# 20D Ubuntu x86_64 裸机验证结果（2026-10-18）

本轮按 [20D 跨平台交接](20d-cross-platform-handoff.md) §3.1–§3.5 执行，全部必跑命令退出码为 `0`。
逐条原始记录见 [执行日志](20d-cross-platform-log-20261018.md)，完整 stdout/stderr、命令、时间和环境已打包为 [原始证据](assets/20d-linux-20261018/raw-evidence.zip)。

## 提交与环境

- 实际检出：`acfcd4e965e04ab4e4927cafc9205c2048ef4642`；运行前后工作区均干净。
- 交接目标：`8a22c22`，代码提交 `29945b9`；实际 HEAD 比目标多出的提交只改交接/登记文档，核心代码与目标相同。
- Ubuntu 26.04.1 LTS，Linux `7.0.0-38-generic`，`systemd-detect-virt` 返回 `none`。
- CPU：Intel Core i5-12450H，12 个逻辑 CPU；内存 14 GiB。本机为 x86_64/AMD64 架构，CPU 厂商为 Intel，本轮没有提供 AMD 厂商硬件证据。
- Rust/Cargo 1.96.0，Bun 1.4.0，clang/LLVM 21.1.8，目标 `x86_64-unknown-linux-gnu`。
- Wayland 桌面，`DISPLAY=:0`、`WAYLAND_DISPLAY=wayland-0`；`x-terminal-emulator` 指向 `/usr/bin/lxterm`。
- `XIAO_USE_XVFB=0`；本轮没有使用 Xvfb。`XIAO_DIAGNOSTICS_HOLD_MS=0` 用于测试；终端测试通过只作为握手/执行证据，不是新一轮截图取证。

## 必跑结果

| 步骤 | 退出码 | 实际结果 |
| --- | ---: | --- |
| §3.1 仓库门禁 | 0 | fmt、Clippy、文档/覆盖率、锁文件、TypeScript、基准检查/测试/自检、Driver 测试、Bun 均通过 |
| §3.2 全量默认测试 | 0 | 1205 passed / 0 failed / 41 ignored |
| §3.3 重建 release Runtime 与诊断程序 | 0 / 0 | 当前提交重新构建并记录产物 SHA-256 |
| §3.3 显式原生用例 | 0 | 39 passed / 0 failed / 0 ignored；仅按交接命令过滤 DNS 和隔离平台异常子入口 |
| §3.4 `20d_first_api` | 0 | 15 passed / 0 failed / 2 ignored |
| §3.4 `20d_process_api` | 0 | 13 passed / 0 failed / 5 ignored |
| §3.5 完整平台复现 | 0 | ignored 阶段 41 passed / 0 failed，含 DNS/平台异常子入口；打包、原生运行、核心发现和协议拒绝回环均通过，调试原生产物退出码 0 |

Bun 两次均为 **294 pass / 6 skip / 0 fail**，与交接预期的 295/5 有一项环境差异：
`cli/ts/src/platform/toolchain.test.ts` 的真实 clang 探测仅在 `XIAO_TOOLCHAIN_SMOKE=1` 时执行，交接命令没有设置它。
其余跳过为本机缺少相应 Shell/平台的可选测试。未通过改参数重跑来改变计数。

§3.3 中 `real_terminal_session_is_environment_gated`、`native_table_methods_debug_execution_o0_o3` 均实际执行并通过；§3.5 再次实际执行并通过。
专项夹具的 ignored 数量对应默认选择；它们的原生部分已由显式原生命令覆盖。

## 20D 与 CI 逐字对账

参考 [CI run 39104020329 / Linux job 117605942603](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39104020329/job/117605942603)，代码提交为 `29945b9`。
从 CI 和裸机日志各抽出 19 条 `20D-NATIVE`、`20D-PROCESS`、`20D-PROCESS-REVIEW` 记录，逐行比较字符串和重复次数，**19/19 完全一致**。
只忽略并行测试造成的行顺序，不改输出或错误身份；原始记录与比较结果均在证据包中。

覆盖包括缓存等待 `resource<process>\n7\n7\n`、关闭后 `X06-RUNTIME-025`、程序不存在/非法 UTF-8 `X06-RUNTIME-012`、缺能力 `X06-RUNTIME-026`、动态种类错误 `X06-RUNTIME-002`。
Linux 信号退出输出为 `143\n`。隐式 shell 拒绝、有效解释器、继承环境、关闭/释放不等待且不杀子进程的测试均通过。

原生测试前后执行 `ps -eo pid,ppid,stat,comm`：两个快照都没有僵尸或 `process_child` 夹具条目。
此结论只适用于这两个采样时点；完整进程生命周期行为由对应 VM/原生夹具断言提供证据。

本轮提供 Ubuntu x86_64 裸机补充证据。Windows/macOS 证据仍引用既有 CI，未改动源码、未执行可选旧债或性能取数。

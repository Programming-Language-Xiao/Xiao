# 20D 跨平台专项交接：Ubuntu AMD x64 裸机验证

> **目标提交**：`8a22c22`（代码 `29945b9`，文档回写 `8a22c22`）。
> **已有证据**：维护回归 `39104020329` 六作业全绿、工作区门禁 `39104020386` 成功；
> 20D 的进程/文件 API 在 Windows、Linux x64、macOS arm64 上均通过 VM 与原生差分。
> **本文交接的是 CI 覆盖不到的那一维**：实体 Ubuntu AMD x64 裸机。

状态：**Ubuntu x86_64 裸机执行已完成（2026-10-18）**。结果见[裸机执行记录](20d-cross-platform-results-20261018.md)，原始输出见[执行日志](20d-cross-platform-log-20261018.md)。本轮实际 CPU 为 Intel i5-12450H，架构为 x86_64/AMD64。

## 一、这次裸机验证能补什么、不能补什么

| 维度 | CI 已覆盖 | 裸机要补 |
| --- | --- | --- |
| 三平台契约 | ✅（Win/Linux x64、macOS arm64） | — |
| 操作系统/架构 | ✅ 三种 | ❌ 与 CI Linux 同为 Ubuntu x64，不新增架构 |
| VM vs 裸机内核/硬件 | ❌ | ✅ 真实内核、调度、文件系统、硬件 |
| 真实终端（诊断窗口） | Linux 用 Xvfb；macOS 跳过 | ✅ 有桌面时可跑真实终端 |
| Unix 信号与僵尸回收 | 只在 VM 上 | ✅ 真实进程表、信号投递、回收 |
| 发行版/工具链差异 | 固定 runner | ✅ 机器自带 clang/LLVM/内核 |

**不能替代**：Windows 的 139 异常分类与 macOS arm64 的差分证据；这两块仍以维护回归为准。

## 二、前置

1. 在仓库根目录确认：
   ```bash
   git rev-parse HEAD          # 应为 8a22c22 或更新的已推送提交
   git status --porcelain      # 必须为空
   ```
2. **不要对未提交的工作树取证**——证据必须绑定提交。若 HEAD 已前进，记录实际 SHA 并说明。
3. 先记录环境（放进日志文件头）：
   ```bash
   uname -a
   lsb_release -a
   lscpu | head -20
   rustc -Vv
   cargo -V
   clang --version
   llvm-as --version
   echo "DISPLAY=${DISPLAY:-<none>}"
   command -v xterm || true
   command -v xvfb-run || true
   ```

## 三、执行序列

所有命令都在仓库根目录执行，建议 `2>&1 | tee logs/<name>.log` 并记录 `echo $?`。
先建 `logs/` 目录；**不要在裸机上改源码**。

### 3.1 仓库门禁

```bash
bash tools/gates/run.sh
```

预期 `exit 0`，包含 fmt、Clippy、文档/覆盖率、锁文件、TypeScript、基准自检、
驱动器测试与 Bun。Bun 预期 `295 pass / 5 skip / 0 fail`。

### 3.2 全量默认测试

```bash
cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast -j 2
```

预期 0 failed；记录实际的 passed/ignored 数字（Windows 本机为 1204 / 0 / 41，
Linux 可能因平台用例略有差异，**以实际值为准**）。

### 3.3 受控原生环境

```bash
# 若尚未安装：
sudo apt-get update
sudo apt-get install -y clang llvm xvfb xterm

export XIAO_CLANG="$(command -v clang)"
export XIAO_LLVM_AS="$(command -v llvm-as)"
export XIAO_LLC="$(command -v llc)"
export XIAO_STRIP="$(command -v llvm-strip || command -v llvm-strip-18)"
export XIAO_TARGET_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"

# 重新构建 Runtime staticlib——不要用旧产物
cargo build --manifest-path core/rust/Cargo.toml --release -p xiao-runtime
cargo build --manifest-path core/rust/Cargo.toml -p xiao-diagnostics

export XIAO_RUNTIME_LIBRARY="$PWD/core/rust/target/release/libxiao_runtime.a"
export XIAO_DIAGNOSTICS_PATH="$PWD/core/rust/target/debug/xiao-diagnostics"
export XIAO_DIAGNOSTICS_HOLD_MS=0
test -f "$XIAO_RUNTIME_LIBRARY"
test -x "$XIAO_DIAGNOSTICS_PATH"
```

被忽略的原生用例：

```bash
# 有桌面/真实终端（推荐）：
cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast -- --ignored --nocapture \
  --skip dns_resolution_failure_is_unavailable \
  --skip platform_failure_child

# 无桌面时再加两条终端依赖的跳过，或用 xvfb-run：
#   --skip real_terminal_session_is_environment_gated
#   --skip native_table_methods_debug_execution_o0_o3
```

### 3.4 20D 专门夹具

```bash
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20d_first_api
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20d_process_api
```

预期：`20d_first_api` 为 15 passed / 2 ignored；`20d_process_api` 为 13 passed / 5 ignored。

### 3.5 平台复现脚本

```bash
# 有真实终端：
XIAO_USE_XVFB=0 bash tools/platform-reproduction/reproduce.sh native
# 无桌面：
XIAO_USE_XVFB=1 bash tools/platform-reproduction/reproduce.sh native
```

脚本会自行构建 Runtime 并跑原生复现；若网络不可用，记录失败的 DNS 用例并注明
「外部网络条件」，不要改测试。

## 四、预期输出（用于逐字对账）

### 4.1 进程记录

| 记录 | 预期标准输出 | 错误身份 |
| --- | --- | --- |
| `20D-PROCESS cached` | `resource<process>\n7\n7\n` | 成功 |
| `20D-PROCESS closed` | 空 | `X06-RUNTIME-025` |
| `20D-PROCESS missing` | 空 | `X06-RUNTIME-012` |
| `20D-PROCESS denied` | 空 | `X06-RUNTIME-026` |
| `20D-PROCESS signal` | `143\n`（Linux 的 `128 + SIGTERM`） | 成功 |
| `20D-PROCESS-REVIEW declaration / assignment / parameter / return / inferred_parameter / filesystem_parent` | 空 | `X06-RUNTIME-002` |
| `20D-PROCESS-REVIEW valid` | `7\n` | 成功 |
| `20D-PROCESS-REVIEW valid_file_parent / valid_directory_parent` | `ok\n` | 成功 |

### 4.2 文件/目录原生记录

`20D-NATIVE relative` → `relative-ok\n`；`directory` → `resource<directory>\n`；
`missing` / `utf8` → `X06-RUNTIME-012`；`closed` → `X06-RUNTIME-025`。

把裸机的这些行与 CI Linux 作业 `117605942603` 的记录逐行对比；除平台专属的
signal 行外应完全一致。任何差异都要原样保留日志，不要自行「修正」。

## 五、失败协议

1. **不改源码、不跳过失败用例、不重跑掩盖**；保留完整日志；
2. 先确认不是旧产物：重新执行 §3.3 的 release Runtime 构建；
3. 记录：提交 SHA、命令、退出码、完整 stderr/stdout、环境清单；
4. 把日志发回；由审核侧判断是实现问题、环境问题还是文档问题。

## 六、回收方式

把 `logs/` 与该机器的环境清单发回即可。审核侧会：

- 把结果登记为本文的「裸机执行记录」；
- 与 `39104020329` 的三平台日志对账；
- 若通过，把它作为 **CI 之外的补充证据**（不替代 CI）；
- 若失败，进入审核附录，按失败项定位，不在裸机上顺手修。

### 本轮裸机执行记录（2026-10-18）

实际检出 `acfcd4e`，与目标 `8a22c22` 的代码相同。§3.1–§3.5 均退出 `0`，
全量默认测试 `1205 passed / 0 failed / 41 ignored`；显式原生选择 `39 passed / 0 failed`，
完整复现脚本的 ignored 阶段 `41 passed / 0 failed`。20D 的 19 条输出/错误身份记录
与 CI Linux 作业 `117605942603` 逐字一致，包括信号退出 `143`。

Bun 实际为 `294 pass / 6 skip / 0 fail`：六项跳过包括未开启的 clang 探测、三个 zsh/fish 用例和两个 Windows 专属用例；
其中 PowerShell 的 Windows/Linux 条件解释相较 Windows 本地记录多出的一项跳过。
`39/41 passed` 是框架统计，分别含 2/3 个直接返回的隔离子入口；对应父测试的实际验证和逐项说明见结果文档。
完整环境、命令、退出码与 CI 对照见[结果](20d-cross-platform-results-20261018.md)及[日志](20d-cross-platform-log-20261018.md)。

## 七、可顺手补的旧债（可选，不与本批验收绑定）

| 债务 | 对应动作 |
| --- | --- |
| 10Z-D Linux 受控复跑 | 按 [10Z-D](10z-dynamic-error-constructor-followup.md) 的命令在裸机重跑，记录运行号/日志 |
| T4 受控复跑 | 按 [10Z-收口](10z-19-closeout-declaration.md) 的 T4 条目补同机前后输出 |
| 19D 基准基线 | `cargo test --manifest-path tests/benchmarks/Cargo.toml` 与 performance driver 自检 |
| platform-reproduction | §3.5 已经覆盖；把它作为 Linux 裸机轮次的正式记录 |

这些旧债各自有归属文档；本文只提供执行点，不改它们的状态。

## 相关页面

- [20D-进程 API](20d-process-api.md) §十二——本轮 1.16 自审、三平台证据与失败记录
- [20D-首批 API](20d-first-api.md) §十四——上一批的三平台验收与复核格式
- [10D 受控原生环境](10d-environment-gated-test-spec.md)（§4 环境变量与工具链）
- [10Z-D 动态错误构造参数](10z-dynamic-error-constructor-followup.md)、[10Z 收口声明](10z-19-closeout-declaration.md)
- [11X0 平台复现](11x0-platform-reproduction.md)

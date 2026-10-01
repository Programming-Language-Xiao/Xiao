# 10J. 跨操作系统验证专项（Linux 侧交接）

> **这是一份独立交接档，写给 Ubuntu 服务器上的接手者。**
> 背景：CI 首次真跑（2026-10-01）四平台全红，根因是**一段 Unix 专属代码从未在 Unix 上编译过**。
> 本档任务是在 Linux 上复现、修复、验证；macOS 本机无法验证，只给排查线索，最终由 CI 说话。
>
> **⚠️ 本档与 N0-D（[10I](10i-n0d-artifact-verification-and-trimming.md)）是两条线**——
> 那是产物验证与裁剪，本档只做跨操作系统收口。**先把本档做完**，N0-D 才有验过的地基。

## 一、事故摘要

**运行**：`workflow_dispatch`，run id `36799534374`（2026-10-01 01:07 UTC，7m46s）

| 平台 | 结果 |
| --- | --- |
| `linux-amd64` | ❌ failure |
| `linux-arm64` | ❌ failure |
| `macos-arm64` | ❌ failure |
| `windows-amd64` | ❌ failure（job 总结论） |

**失败步骤**：`Check Rust Clippy` 与 `Run Unix reproduction`。

### 根因：`crates/xiao-runtime/src/crash.rs` 的 `#[cfg(unix)]` 分支

日志原文（`linux-arm64`）：

```
warning[E0133]: call to unsafe function `libc::sigaltstack` is unsafe and requires unsafe block
  --> crates/xiao-runtime/src/crash.rs:28   (install_alternate_signal_stack)

warning[E0133]: call to unsafe function `std::mem::zeroed` is unsafe ...
  --> crates/xiao-runtime/src/crash.rs:41   (install_unix_signal)

warning[E0133]: call to unsafe function `libc::sigemptyset` is unsafe ...
  --> crates/xiao-runtime/src/crash.rs:45

help: first cast to a pointer `as *const ()`
  --> crates/xiao-runtime/src/crash.rs:43   (function_casts_as_integer)
```

两类：

1. **`unsafe_op_in_unsafe_fn`**（Rust 2024 兼容性，默认 warn）——
   `unsafe fn` 体内调用 unsafe 函数**仍需显式 `unsafe {}` 块**。
   命中 `:28`（`libc::sigaltstack`）、`:41`（`std::mem::zeroed`）、
   `:45`（`libc::sigemptyset`）、`:43`（`libc::sigaction`）。
2. **`function_casts_as_integer`** —— `:43` 的 `signal_handler as usize` 应写成
   `signal_handler as *const () as usize`。

**在 `-D warnings` 下这些 warning 变成 error** → `Check Rust Clippy` 红 →
`Run Unix reproduction` 里的构建也红。

### 为什么一直没被发现

**这些代码全在 `#[cfg(unix)]` 里，Windows 上根本不编译。**
而 `crash.rs` 是 N0-C-2（`d95b6ae`）引入的——也就是说，
**那段平台异常处理器，从写出来到这次 CI 首跑，一次都没在 Unix 上编译过。**

Windows 本机至今四项门禁全绿，正是因为看不到它。

---

## 二、任务清单

| # | 任务 | 说明 |
| --- | --- | --- |
| 1 | **先复现** | 在 Linux 上看到 `-D warnings` 报 E0133，**再动手改** |
| 2 | **修复 `crash.rs`** | §四的要点 |
| 3 | **clippy 全绿** | `--workspace --all-targets -- -D warnings` |
| 4 | **门控全绿** | `cargo test --workspace -- --ignored` |
| 5 | **平台异常报告实测** | Linux 上真触发段错误，验退出码与机器字段 |
| 6 | **端到端回环** | `XIAO_USE_XVFB=1 ... reproduce.sh native` |
| 7 | **推 CI 验四平台** | **必须包含 macOS 的结果** |
| 8 | macOS | **本机不做**——看 CI |

---

## 三、环境准备（Ubuntu）

**⚠️ 以 [10D §4.2](10d-environment-gated-test-spec.md) 为权威，别另写一套。** 摘录：

```sh
sudo apt-get install clang llvm lld build-essential xvfb xterm
sudo update-alternatives --set x-terminal-emulator /usr/bin/xterm
rustup component add rustfmt --toolchain 1.96.0-$(rustc -vV | sed -n 's/^host: //p')
cargo build --manifest-path core/rust/Cargo.toml -p xiao-runtime --release

export XIAO_CLANG="$(command -v clang)"
export XIAO_LLVM_AS="$(command -v llvm-as)"
export XIAO_LLC="$(command -v llc)"
export XIAO_RUNTIME_LIBRARY="$PWD/core/rust/target/release/libxiao_runtime.a"
export XIAO_TARGET_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
```

**三个容易踩的点**：

- **Linux 上是 `libxiao_runtime.a`**（Windows 是 `xiao_runtime.lib`）——别沿用 Windows 的路径；
- **`XIAO_TARGET_TRIPLE` 必须等于 `rustc -vV` 的 `host`**，不能手工沿用 COFF 配置；
- **Rust 版本由 `core/rust/rust-toolchain.toml` 钉在 `1.96.0`**（含 `rustfmt`、`clippy`）——
  别用系统默认工具链，否则 lint 行为可能不同。

### 诊断相关测试要 `XIAO_DIAGNOSTICS_PATH`

**这是 10H 刚踩过的坑**：`find_renderer` 会选到 `target/debug/xiao-diagnostics`
这个**由 `cargo build` 才更新、`cargo test` 从不更新**的文件。

```sh
cargo build -p xiao-driver -p xiao-diagnostics          # 先构建 bin
export XIAO_DIAGNOSTICS_PATH="$PWD/core/rust/target/debug/xiao-diagnostics"
```

**不设会明确失败**（不再静默用旧产物），失败信息会指向 `10D §4.2`。

---

## 四、修复要点

### 4.1 `unsafe fn` 体内加显式 `unsafe {}` 块

```rust
unsafe fn install_unix_signal(signal: libc::c_int) {
    unsafe {                                   // ← 新增
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = signal_handler as *const () as usize;
        action.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
        libc::sigemptyset(&mut action.sa_mask);
        let _ = libc::sigaction(signal, &action, std::ptr::null_mut());
    }
}
```

`install_alternate_signal_stack` 同理。

### 4.2 ⚠️ **不要用 `#[allow(...)]` 压掉**

`unsafe_op_in_unsafe_fn` 是 **Rust 2024 的语义收紧**，不是噪音：
它要求"哪些操作需要 unsafe"在源码里**可见**。压掉它等于把
「这段为什么安全」的理由一并删了——**这正是 `crash.rs` 该留下的东西**
（那是信号处理器，写错的代价是崩溃时再崩一次）。

### 4.3 ⚠️ macOS 可能有**额外的**差异——**别凭"都是 Unix"推断**

**Linux 修完不等于 macOS 就绿。** 本机验不了 macOS，但以下是已知的风险点：

- **`libc::SIGSTKSZ`**：Linux(glibc 2.34+) 上它已经不是编译期常量，
  **macOS 的取值方式也不同**。`crash.rs:29` 的
  `(libc::SIGSTKSZ as usize).saturating_mul(4)` 在 macOS 上可能需要另写法。
- **`sigaction.sa_sigaction` 的类型与可用性**：Linux 与 macOS 的 `libc` 定义有差异。
- **`SA_ONSTACK` + `sigaltstack`**：两边都支持，但栈大小/行为细节不同。
- **`SIGBUS`**：两边都有，触发语义不同。

**要求**：Linux 修完**推 CI**，让 `macos-arm64` **自己说话**。
若 macOS 仍红，**照着它的报错再修一轮**，不要预设它和 Linux 同因。

---

## 五、验收

1. **Linux**：`cargo clippy --workspace --all-targets -- -D warnings` 绿；
2. **Linux**：`cargo test --workspace -- --ignored` 绿（含 `n0_c2_error_boundary` 的平台异常子进程测试）；
3. **Linux**：`platform_failure_report_is_machine_readable` 通过——
   真触发段错误、退出码 `139`、机器字段齐全（`class=fatal` / `code=X07-FATAL-005` /
   `message_id=fatal.hardware` / `exit_code=139`）；
4. **Linux**：`XIAO_USE_XVFB=1 bash tools/platform-reproduction/reproduce.sh native` 绿；
5. **CI 四平台全绿**——**含 `macos-arm64`**；
6. **不回归**：Windows 本机的四项门禁仍绿（`cargo test --workspace`、clippy、
   `bun run check`、门控 `--ignored`）。

### 5.1 本次执行记录（2026-10-01）

- 已按 §4.1 修复 `core/rust/crates/xiao-runtime/src/crash.rs`：Unix 两个 `unsafe fn` 的不安全操作均放入显式 `unsafe {}`，`SIGSTKSZ` 使用平台原生 `usize`，并将 `signal_handler` 转换改为 `as *const () as usize`；未使用 `#[allow(...)]`。
- 为使 `-D warnings` 在 Linux 通过，修正 `xiao-package/src/cache.rs` 的 Unix 条件编译可变性；为使跨平台 CLI 测试向量与宿主一致，修正 Git 缺失、激活路径和环境布局测试的宿主路径假设。
- Rust 1.96.0：`cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings` 通过。
- Linux 门控：`cargo test --manifest-path core/rust/Cargo.toml --workspace -- --ignored` 通过；`platform_failure_report_is_machine_readable` 单独运行通过，真实子进程退出码为 139 且机器字段齐全。
- Linux 原生回环：`XIAO_USE_XVFB=1 bash tools/platform-reproduction/reproduce.sh native` 通过（Rust 1.96.0、LLVM/Clang 21.1.8、Xvfb；Bun 缓存使用独立临时目录）。普通 native、`-debug` native、同目录/PATH/开发回环和协议失配均通过。
- 四平台 CI 已通过：[run 36805985940](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/36805985940) 的 `linux-amd64`、`linux-arm64`、`macos-arm64`、`windows-amd64` 均为 `success`；macOS Unix 分支未出现额外的 `libc`/`sigaction` 问题。

---

## 六、本次不负责

- **不做 N0-D**（三目标固定宽度、Runtime 裁剪验证）——见 [10I](10i-n0d-artifact-verification-and-trimming.md)，那是另一条线。
- **不重开** N0-C 的设计（错误路径、释放追踪、差分、`-debug` 三不）。
- **不改** macOS 的启动器逻辑——`toolchain.rs` 的 `__APPLE__` 分支已在 `58deb01` 落地，
  本档只验它**能不能编译、能不能跑**，不改它的行为。
- **不做交叉编译**（`10:57` 明确留到第 11 阶段）。
- **不改** `10D §4.2` 的准备流程——若发现它不对，**改它并说明原因**，别在别处另写一份。

---

## 相关页面

- [10D. 环境门控测试规范](10d-environment-gated-test-spec.md) **§4.2** —— **Linux/macOS 准备的权威**
- [10H. N0-C-3 `-debug` 产物与欠账清算](10h-n0c3-debug-window-and-debt.md) **§2.2/§2.3** —— CI 首跑与 `XIAO_DIAGNOSTICS_PATH` 的由来
- [10I. N0-D 产物验证与裁剪](10i-n0d-artifact-verification-and-trimming.md) —— **另一条线**，本档做完再做
- [10. LLVM 原生后端](10-native-backend.md) —— 权威规范
- [10G. N0-C-2 错误边界与字节码差分](10g-n0c2-error-boundary-and-differential.md) —— `crash.rs` 的引入批次

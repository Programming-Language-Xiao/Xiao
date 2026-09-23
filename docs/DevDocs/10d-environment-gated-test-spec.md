# 10D. 环境依赖测试专项规范

> **本文定义一类测试怎么写。** 起因是 [10C](10c-native-runtime-link-fix.md) 那个缺陷：
> N0-B 的主功能路径链不出来，而**门禁一直是绿的**——因为 7 个需要外部工具链的测试
> 在默认环境里**静默跳过**，在汇总里显示为 `ok`。
>
> 本文不是给那一次事故写的反思，是给**这一类测试**定规矩。

## 一、问题：四种真实状态塌缩成两种

一个依赖外部环境的测试，真实状态有四种：

| 状态 | 含义 | 应该怎么显示 |
| --- | --- | --- |
| **通过** | 环境齐备，断言成立 | `ok` |
| **失败** | 环境齐备，断言不成立 | `FAILED` |
| **没跑** | 环境缺失（没装 clang / 没设变量） | **必须是"没跑"** |
| **跑不了** | 环境在但不可用（clang 版本不对、链接器缺失） | **必须是"失败"** |

**现在的写法把后两种都变成了 `ok`**：

```rust
let Some(clang) = std::env::var_os("XIAO_CLANG") else {
    return;                 // ← 既不算失败，也不算"没跑"——它算通过
};
```

**后果**：`cargo test` 输出「647 passed; 0 failed」时，
**其中有 7 个在这台机器上从来没执行过**。门禁绿 ≠ 跑过。

10C 的缺陷就是这么活下来的：那条测试从写出来那天起就没真跑过，
**第一次真跑就失败了**。

## 二、现状清单（2026-09-23 实测）

**8 个测试**在默认环境里静默跳过，涉及 **4 个环境变量**和一个真实终端能力：

| 文件 | 测试 |
| --- | --- |
| `xiao-codegen-llvm/tests/n0_a.rs` | `optional_real_llvm_round_trip` |
| `xiao-codegen-llvm/tests/n0_a.rs` | `optional_corrupt_llvm_is_rejected` |
| `xiao-codegen-llvm/tests/n0_a.rs` | `optional_entry_observation_tracks_runtime_branch` |
| `xiao-codegen-llvm/tests/n0_b_dynamic.rs` | `optional_llvm_accepts_dynamic_table_module` |
| `xiao-driver/tests/n0_a_native_driver.rs` | `optional_real_frontend_to_native_round_trip` |
| `xiao-driver/tests/n0_a_native_driver.rs` | `optional_frontend_artifact_differential_round_trip` |
| `xiao-driver/tests/n0_b_dynamic_native.rs` | `optional_dynamic_string_native_round_trip` |
| `xiao-driver/src/diagnostics.rs` | `real_terminal_session_is_environment_gated` |

环境变量：`XIAO_CLANG`、`XIAO_LLVM_AS`、`XIAO_RUNTIME_LIBRARY`、`XIAO_TARGET_TRIPLE`。

**齐备环境下的实测结果**（10C 发现时跑的）：

```text
n0_a                     16 passed    （3 个 optional 全部真跑并通过）
n0_b_dynamic             13 passed    （1 个 optional 真跑并通过）
n0_a_native_driver        6 passed    （2 个 optional 真跑并通过）
n0_b_dynamic_native       6 passed, 1 FAILED   ← 唯一的失败，正是 10C
```

**8 个里 1 个是坏的**。这个比例说明：静默跳过不是"省事"，是**在积累未验证的代码**。

## 三、规范

### 3.1 需要外部环境的测试**必须标 `#[ignore]`**

```rust
#[test]
#[ignore = "需要 XIAO_CLANG / XIAO_LLVM_AS；准备方式见 10D §4"]
fn optional_real_llvm_round_trip() { ... }
```

**为什么是 `#[ignore]` 而不是继续条件 `return`**：

`#[ignore]` 的跳过**在汇总里天然可见**：

```text
test result: ok. 40 passed; 0 failed; 0 ignored    ← 一眼看到有没有跳过
test result: ok. 33 passed; 0 failed; 8 ignored    ← 这 8 个去哪了，写在文档里
```

它把"没跑"变成了**第四种状态**，而不是伪装成"通过"。
`--nocapture`、额外的日志、grep 计数**都不需要**——数字就在汇总行里。

### 3.2 显式跑的时候，环境缺失**必须失败**

标了 `#[ignore]` 的测试，跑 `cargo test -- --ignored` 时就是**显式要求执行**。
此时若 `XIAO_CLANG` 还是没设，**应该 panic，而不是继续 return**：

```rust
let clang = std::env::var_os("XIAO_CLANG")
    .expect("显式运行 --ignored 时 XIAO_CLANG 必须已设置；准备方式见 10D §4");
```

理由：**"没跑"已经由 `ignored` 表达了**。既然显式跑了，缺环境就是**配置错误**，
必须暴露成失败——否则又回到"静默跳过"。

### 3.3 每条 `#[ignore]` 的理由要能定位到准备方式

`#[ignore = "..."]` 的字符串里写清**需要哪些变量**，并指向本文 §4。
否则接手者只知道"它被跳过了"，不知道"怎么让它跑起来"。

## 四、怎么准备齐备环境

**这一节是 10C 摸索出来的，照抄即可。**

前置：安装 Visual Studio 的 C/C++ 桌面开发负载（提供 MSVC 的 `link.exe`），
以及 MSYS2 的 clang / llvm-as（提供能编译 LLVM IR 的 clang）。

准备一个 `.bat`（**必须全 ASCII，见 §4.3**）：

```bat
@echo off
call "<VS_ROOT>\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
set PATH=<MSYS2_ROOT>\ucrt64\bin;%PATH%
set XIAO_CLANG=<MSYS2_ROOT>\ucrt64\bin\clang.exe
set XIAO_LLVM_AS=<MSYS2_ROOT>\ucrt64\bin\llvm-as.exe
set XIAO_RUNTIME_LIBRARY=<REPO>\core\rust\target\release\xiao_runtime.lib
set XIAO_TARGET_TRIPLE=x86_64-pc-windows-msvc
cd /d <REPO>\core\rust
cargo test -p xiao-codegen-llvm -p xiao-driver -- --ignored
```

**先决条件**：`XIAO_RUNTIME_LIBRARY` 指向的 staticlib 要先构建出来
（`cargo build --release -p xiao-runtime`；它的 `crate-type` 含 `staticlib`）。

### 4.2 Linux、WSL 与 macOS

Linux 与 WSL 使用发行版提供的 LLVM，macOS 使用 Homebrew LLVM；三者都必须把目标三元组
设成 `rustc -vV` 的 `host` 行，不能手工沿用 Windows 的 COFF 配置。复现脚本
`tools/platform-reproduction/reproduce.sh native` 会执行同一套准备、门控测试和端到端回环。

Ubuntu/Arch WSL 的最小准备：

```sh
sudo apt-get install clang llvm lld build-essential xvfb xterm   # Ubuntu
sudo pacman -S --needed base-devel clang llvm lld                 # Arch
cargo build --manifest-path core/rust/Cargo.toml -p xiao-runtime --release
export XIAO_CLANG="$(command -v clang)"
export XIAO_LLVM_AS="$(command -v llvm-as)"
export XIAO_LLC="$(command -v llc)"
export XIAO_RUNTIME_LIBRARY="$PWD/core/rust/target/release/libxiao_runtime.a"
export XIAO_TARGET_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --manifest-path core/rust/Cargo.toml --workspace -- --ignored
```

macOS 需要先把 Homebrew LLVM 放入 PATH，并保留 `osascript` 与 `open`：

```sh
brew install llvm
export PATH="$(brew --prefix llvm)/bin:$PATH"
export XIAO_CLANG="$(command -v clang)"
export XIAO_LLVM_AS="$(command -v llvm-as)"
export XIAO_LLC="$(command -v llc)"
export XIAO_RUNTIME_LIBRARY="$PWD/core/rust/target/release/libxiao_runtime.a"
export XIAO_TARGET_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
cargo test --manifest-path core/rust/Cargo.toml --workspace -- --ignored
```

`real_terminal_session_is_environment_gated` 还要求真实终端能力：Linux/WSL 要有可用的
`DISPLAY`（无桌面 runner 使用 `xvfb-run`），macOS 要能执行 `osascript`/`open -a Terminal`。
没有这些能力时，显式 `--ignored` 必须失败，不能把终端测试重新改成条件 `return`。

### 4.3 三个必须知道的坑

1. **`vcvars64.bat` 会改 PATH**，但 clang 需要**同时**能看到 MSVC 的工具——
   所以顺序是**先 vcvars，再把 MSYS 加进 PATH**，不能反过来。
2. **Git 自带一个 `link.exe`**（`<Git>\usr\bin\link.exe`）。如果它在 MSVC 的
   `link.exe` **之前**，clang 会调错链接器。挂好 vcvars 后可以用 `where link` 确认顺序：
   MSVC 的**必须在前**。
3. **`.bat` 里不要写中文**。cmd 默认代码页是 GBK，UTF-8 的中文注释会让它**语法解析崩溃**
   （报 `'xxx' is not recognized` 之类），而且错误信息会误导排查方向——
   10C 的排查在这上面绕了好几轮。

### 4.4 关于 `XIAO_RUNTIME_LIBRARY` 的路径

它必须是**已经构建好的** staticlib。若路径不存在，测试应当**失败**（按 §3.2），
而不是退回跳过——**"库没构建"是配置错误，不是环境缺失**。`XIAO_TARGET_TRIPLE` 也必须
    明确设为与 Runtime staticlib 相同的目标；Windows 使用 `x86_64-pc-windows-msvc`，
    Linux 使用 `x86_64-unknown-linux-gnu` 或 `aarch64-unknown-linux-gnu`，macOS 使用
    `x86_64-apple-darwin` 或 `aarch64-apple-darwin`。

## 五、门禁要求

1. **默认门禁**：`cargo test` 的汇总里**记录 `ignored` 的数量**。
   与本文 §2 的清单（当前 8 个）不一致时，要么是新增了环境依赖测试（好事，更新清单），
   要么是有人把测试从 `ignored` 改回了条件 `return`（**要拒绝**）。
2. **每批至少一次齐备环境跑**：把对应平台的 §4 脚本跑一次，结果写进该批的交接记录。
   **这是唯一能证明那些测试还活着的动作。**
3. **`ignored` 数归零不一定是好事**：它可能意味着有人为了"门禁好看"把它们改成了
   条件 `return`——那正是本文要防的形态。

## 六、验收

1. §2 清单里的 8 个测试**全部标了 `#[ignore]`**，且理由字符串能定位到本文 §4；
2. 默认 `cargo test` 的汇总里能看到 `8 ignored`；
3. `cargo test -- --ignored` 在齐备环境里**全部执行**（10C 修复前允许 1 个失败，
   修复后必须全绿）；
4. 缺环境时跑 `--ignored` 会**失败**，不是静默跳过（§3.2）；
5. 本批交接记录里有**一次齐备环境的完整结果**。

## 七、落地记录（2026-09-23）

§2 列出的 8 条测试现已全部使用 `#[ignore = "...；准备方式见 10D §4"]`：
`xiao-codegen-llvm` 的 4 条、`xiao-driver` 的 3 条和诊断窗口的 1 条。默认运行
`cargo test -p xiao-codegen-llvm -p xiao-driver` 另加诊断单元测试时的汇总为 8 ignored（分别为 3、1、2、1、1），
没有把缺环境伪装成通过；显式运行 `--ignored` 时会用 `expect` 检查变量，Runtime 库路径
不存在也会直接断言失败。

在 Windows 原生环境按 §4 准备后，以下命令已完整执行 7 条门控测试并全部通过：

```text
n0_a                         3 passed
n0_b_dynamic                 1 passed
n0_a_native_driver           2 passed
n0_b_dynamic_native          1 passed
合计                         7 passed, 0 failed
```

动态 Runtime 测试使用 `x86_64-pc-windows-msvc`，通过 `Toolchain::probe_native_static_libraries`
查询 Rust 清单后链接 `xiao_runtime.lib`。VS Community 18.9.2、MSVC linker 14.51.36256.0、
MSYS2 clang/llvm-as 22.1.2 和 Rust 1.96.0 均为本次记录的实际工具；缺少 `XIAO_CLANG` 时
单独执行 `optional_real_llvm_round_trip --ignored` 已确认会失败并给出配置错误。

Windows 的 7 条工具链门控结果已有历史记录；新增的真实终端测试和 Linux/WSL/macOS 的
`--ignored` 结果由 11X0-P 的平台记录逐项补齐。WSL 和容器共享宿主调度或多一层文件系统，
不能与原生数字并列；未运行的 macOS 不能被宣称为已验证。

## 八、不负责

- **不改变这些测试的断言内容**——本文只管"它们有没有跑"，不管"它们断得对不对"；
- **不修改测试断言**——本批只补跨平台准备与证据记录；
- **不采性能数字**——Linux/WSL/容器的功能证据不进入 09R3 冻结报告。

## 相关页面

- [10C. 原生 Runtime 链接缺陷修复交接](10c-native-runtime-link-fix.md) —— 本文的起因
- [10B. N0-B Runtime ABI](10b-n0-runtime-abi.md) —— 那 7 个测试所在的批次
- [09-B0-D. 退出码冻结与 Linux 容器实测](09b0d-exit-codes-and-linux-verification.md) —— 容器侧的门禁记录
- [00E. 单文件行数门禁交接](00e-file-size-gate.md) —— 另一条门禁的阅读提醒

# 11X0-P. 跨平台复现（Linux 原生 / WSL / macOS）

> **这不是一个新阶段，而是把 X0 积压的平台债清掉。** X0 的八条退出条件里，
> **第 2、4、5、6 条都要求「三平台」**，而目前只有 Windows 原生完成。
> 这个债横跨 09R3 与六个 11X0 子批，已经挂了多轮。
>
> **本批的产出不是功能，是证据**：让「Linux 与 macOS 待复现」这句话变成「已复现」
> 或者「明确记录为什么不能复现」。**禁止在没跑之前改状态**——
> `09r3:182` 把这条列为「最容易做假」的一条。

## 一、Agent 交接上下文

### 接手前提

1. [11X0-E. `xiao build` 与主机工具链发现](11x0e-build-and-toolchain.md) **§5.5**（`:232-245`）
   —— **平台待复现清单的权威原文**。**逐项更新，不要沿用旧表述。**
2. [11X0-C. 独立可执行与平台矩阵](11x0c-packaging-and-platforms.md) **§三**（`:150-179`）
   —— **「验证 ≠ 验收」的完整口径**（见 §五.1）。**本批的证据分级照这一节走。**
3. [09R3. 跨平台基准与冻结](09r3-benchmarks-and-freeze.md) **§六**（`:126-141`）
   —— WSL 与容器的原始约束；**`:199-220` 是「带平台债关门」的先例**，本批的落地记录形态照它写。
4. [10D. 环境依赖测试规范](10d-environment-gated-test-spec.md) —— `#[ignore]` 写法规格与
   环境准备方式。**注意它的清单已经漂移**（§三.4）。
5. [11X0-D. `-debug` 与诊断窗口](11x0d-debug-diagnostics-window.md) **§3.2**（`:106-124`）
   —— **平台终端启动**：三平台没有统一做法，Linux 是四个候选。
6. [10C. 原生运行时链接修复](10c-native-runtime-link-fix.md) —— **`:171-173` 明说
   「不做 Linux/macOS 的原生依赖库适配」**，那是本批要接的。
7. [10A. LLVM 原生构建闭环](10a-n0-native-closure.md) **`:131-138`** ——
   **容器镜像需要 C/LLVM 工具链**，且**不要在 N0 顺手改镜像**。这批就是那一批。

### 现状盘点（2026-09-24 实测）

```text
已完成的证据   Windows 原生：xiao build / run / test / debug 端到端
              Linux Docker amd64：工具链、构建、独立运行、xiao test 与 8 条门控测试全绿
              Linux ARM：目标描述修复与架构断言已落地；镜像构建被主机 QEMU 执行层阻塞
              macOS：仅建立 CI 复现工作流，当前环境未运行
待补证据      Linux 真机、WSL（Ubuntu/Arch）工具链复现、Docker arm64、macOS runner
平台分支      31 处（Rust cfg + TS process.platform）；Linux amd64 已有端到端证据，
              ARM/WSL/macOS 仍不得写成已验证
```

**两个关键事实要先说清**：

1. **现有容器验不了 `xiao build`**：`Dockerfile.dev` 只装了 Rust 1.96.0 + Bun 1.4.0，
   **没有 clang/LLVM**（`10a:35`）。而 Linux 原生要复现的核心正是 `xiao build`
   （`11x0e:240`：需 LLVM ≥18、目标/链接探测、真实源码独立运行）。
   **所以本批第一步是补镜像，不是跑。**
2. **Rust 核心不能交叉编译**：`11x0c:111-112` 明写「bun 能交叉产出，但 Rust 核心不能
   （Windows 上的 macOS 交叉需要额外工具链）。所以打包能与不能，跟随核心能构建的平台」。
   **这是 macOS 必须用 macOS 主机的技术原因**，不是偷懒。

### 本批交付与不负责

**交付**：Linux 原生（Docker 多架构 + 真机/WSL）与 macOS（CI runner）的**功能证据**、
补齐缺失的前置设施（容器工具链、复现脚本）、X0 第 2/4/5/6 条的收口判定。

**不负责**：**X0 第 3 条**（它**不是平台债**，见 §八.2）；09R3 的性能数字（本批不采）。

---

## 二、范围：四项手段，四种证据等级

| # | 手段 | 覆盖 | 能证明什么 | 不能证明什么 |
| --- | --- | --- | --- | --- |
| **1** | **Docker `linux/amd64`** | Linux x86_64 | 构建/运行/发现/退出码/协议字段 | 真机安装路径与用户环境 |
| **2** | **Docker `linux/arm64`** | Linux ARM | 同上，**且能抓架构硬编码**（§四.3） | 同上；QEMU 下编译慢 |
| **3** | **WSL（Ubuntu + Arch）** | 真 Linux 内核 | 更强的功能证据；**Arch 是廉价反例探测**（glibc/路径/`lib` 布局不同） | 仍共享宿主 CPU 调度（数字不可与 Windows 并列） |
| **4** | **CI macOS runner** | 真机 macOS | **Mach-O 工具链、`osascript` 开窗、Rust 核心原生构建** | 云 runner 性能波动大（但本批只要功能） |

⚠️ **手段 1/2/3 都不是「真机 Linux 验收」**——`11x0c:166-168` 说得很清楚：
容器「内核版本、cgroup、文件系统仍然与裸机有差异，**而且容器里没有真实的安装路径与用户环境**」。
**§五.1 给出本批的替代方案与必须写出的判断。**

### 2.1 ⚠️ Arch WSL 已存在，但工具链仍未齐备

`11x0c:76,80-82` 把 Docker(Arch) 列为「发行版差异验证」环境，
`11x0d:214` 的翻车点 8 更是依赖「本仓有 Ubuntu 与 Arch 两个验证环境」。
本机现在确实有 `Ubuntu-26.04` 与 `archlinux` 两个 WSL 发行版，但
2026-09-24 的探测显示两者都没有 `rustc`、Bun、`clang`、`llvm-as` 或 `llc`。
因此它们只能记录为**环境存在、工具链缺失**，不能冒充 WSL 功能通过；Docker
镜像仍是 Debian 13，不把不存在的 Arch Docker 结果写进证据。

---

## 三、必须先做的前置（**否则后面一行都验不了**）

### 3.1 补 Docker 镜像的 C/LLVM 工具链

`10a:131-138` 早就点名：「N0 需要 C/LLVM，但不要在 N0 顺手改镜像」——
**这一批就是该改的时候**。

镜像至少要加：`clang` / `llvm`（`llvm-as`）**≥18**（`11x0e:240`、
`core/rust/llvm-toolchain.toml` 的 `minimum_major = 18`）、以及构建
`libxiao_runtime.a` 所需的环境。

⚠️ **镜像改动会影响 `09b0d` 与 `11x0c` 两次实测的可比性**——
那两次都记了镜像指纹，改镜像后新结果**不能与旧记录直接比较**，
要在记录里写明「镜像已变更」。

### 3.2 补复现脚本

`09r-bytecode-machine-research.md:462`（R1-AE）要求
「Linux 与 macOS 写成明确的待复现清单**与复现脚本**」。本批已补齐
`tools/platform-reproduction/reproduce.sh`、`reproduce.ps1`、Dockerfile 和
`.github/workflows/platform-reproduction.yml`；Linux amd64 已实际执行脚本，
ARM64、WSL 和 macOS 的未完成项保留在 §5.6。

### 3.3 更新 `10D` 的 `#[ignore]` 清单（7 → 8）

`10d §二`（`:34-48`）列 **7 条**，仓内实际是 **8 条**——
X0-D 新增的 `xiao-driver/src/diagnostics.rs:844`（真实终端模拟器）没登记。
`10d §五 门槛 1` 明确要求「与 §2 清单不一致时…更新清单」。**本批顺手补上。**

---

## 四、每项要复现什么（**逐项清单**）

### 4.1 三平台都要走的四条链路

1. **`xiao build`**：工具链探测（clang 版本、`Target:` 行、真实链接探测）→ 原生产物；
2. **产物独立运行**：复制出仓库、清空 `XIAO_CORE_PATH`、PATH 只留 `/usr/bin:/bin`，
   仍要能跑（`11x0c:202` 的 Docker 版已做过，本批在原生与 macOS 重做）；
3. **`xiao run` 与退出码**：B0-D 五值的映射；
4. **`-debug` 开窗**：见 §4.2。

### 4.2 `-debug` 的平台终端启动（**Linux 的难点**）

`xiao-driver/src/diagnostics.rs:378-463` 有三个分支：

- Windows（`:378-412`）：`wt.exe` → `cmd.exe /c start` → PowerShell `Start-Process`；
- macOS（`:413-441`）：`osascript` → `open -a Terminal`；
- **Linux（`:442-463`）：`x-terminal-emulator` → `gnome-terminal` → `konsole` → `xterm`**。

⚠️ **Linux 没有统一标准**（`11x0d:113`）。**必须逐条写明"本次只覆盖了哪一个"**——
`11x0d:244` 的验收要求就是这个。Ubuntu 与 Arch 的默认终端不同（`11x0d:214`）。

### 4.3 ★ 顺带要抓的架构缺陷（**本批的高价值产出**）

`core/rust/crates/xiao-codegen-llvm/src/target.rs:100` 的 `TargetDescription::host()`
曾经按操作系统直接返回 x86_64 目标：

```rust
#[cfg(target_os = "macos")]
{ Self::macos_x86_64() }     // ← 架构写死，无 target_arch 判断
#[cfg(target_os = "linux")]
{ Self::linux_x86_64() }     // ← 同上
```

而同一条规则在 TS 侧
（`cli/ts/src/platform/core.ts:77-91` 的 `hostTarget`）**是按 `process.arch` 动态推导的**
（`arm64` → `aarch64`）。

**这是本仓头号病（同一规则两层各写一份然后漂移）的又一实例**：
TS 侧写对了、Rust 侧写死了。它已在本批修复为按 `target_arch` 返回
`x86_64`/`aarch64` 的 Windows、Linux 和 macOS 目标，并增加
`host_target_architecture_matches_compile_time_target` 断言。

**影响面是单点**：`ProtocolTarget::host()`（`protocol/request.rs:38`）直接
`from_target(TargetDescription::host())`，**修一处即可覆盖协议侧**。

**在 arm64 上它会直接错**：Apple Silicon macOS 与 ARM Linux 都会拿到
`x86_64-*` 三元组。而 **GitHub Actions 的 `macos-14`+ runner 全是 arm64**，
`ubuntu-24.04-arm` 也可用——**手段 2 与手段 4 都会踩到它**。

ARM64 镜像原本应当负责把这条断言真正编译并执行；但本机在安装了
`qemu-aarch64` binfmt 后，`docker buildx build --platform linux/arm64` 仍在
Dockerfile 的 `RUN apt-get ...` 阶段报 `exec /bin/sh: exec format error`，所以
§5.6 只记录为**仿真执行阻塞**，不宣称 ARM64 运行时证据完成。

### 4.4 环境依赖测试

按 `10d §三` 的规范，在**每个目标平台**各跑一次 `cargo test -- --ignored`，
记录 8 条（不是 7 条）的通过/失败。
`10d §五 门槛 2` 说这是「唯一能证明那些测试还活着的动作」。

⚠️ `10d §四` 的**环境准备方式只有 Windows**（`:101-141`），
且 `§八`（`:192`）明写「不做非 Windows 平台的准备方式」。本批已为
Linux/macOS 补上脚本路径：把 `XIAO_CLANG` / `XIAO_LLVM_AS` / `XIAO_LLC`
指向 LLVM ≥18，把 `XIAO_RUNTIME_LIBRARY` 指向 `libxiao_runtime.a`，并把
`XIAO_TARGET_TRIPLE` 设成与 `rustc -vV` 一致的三元组。Linux Docker amd64
按此方式跑通；WSL 因工具缺失未进入测试，macOS 尚未执行。

---

## 五、硬约束

### 5.1 「验证 ≠ 验收」的**分级口径**（本批的核心判据）

沿用 [11X0-C §3](11x0c-packaging-and-platforms.md) 的细分口径，**并显式声明这是对
`09R3:135` 的解读**——因为 `09R3` 的原句「WSL 与容器只当开发环境，**不进验收**」
**字面上比「只是性能数字」更宽**，三份文档的措辞粒度不同（`09r3` 最笼统、
`11x0c` 最精细、`09b0d:220` 居中）。

| 问题 | 结论 |
| --- | --- |
| 容器/WSL 的功能结果**可以**算证据吗？ | **可以**——构建成功、`xiao run` 跑通、退出码、协议机器字段、门禁绿 |
| **不可以**算什么？ | ① 性能/基准数字（**不得进 `tests/benchmarks/reports/`**）；② 不等同真机 Linux 验收；③ **不得据此宣称「跨平台已验证」**（`09b0d:220` 的硬红线） |

**「X0 第 2 条完整达成」的两种合法收法**（`11x0c:178-179` 要求**必须写出判断**）：

- **(a)** 在真实 Linux 主机上复现一次；**或**
- **(b)** 写明为什么容器/WSL 证据足以替代。

**本批的实际立场**：当前没有真实 Linux 主机或已执行的 macOS runner 结果，
而 WSL 两个发行版也缺少工具链，因此本批**不采用 (b)**。Docker amd64 只能
作为可复现的 Linux 功能证据；X0 第 2 条仍保持未收口，具体原因见 §5.6 与 §8.1。

### 5.2 禁止在没跑之前改状态

`09r3:182` 点名「第 6 条最容易做假：未复现前不得宣称已验证」。
本批**每完成一项才能改一项**，且改动必须附**环境、命令、结果**三要素
（`11x0c:183-191` 的五项表格式）。

### 5.3 不要产生 `tests/benchmarks/reports/` 的 diff

`09b0d:300-301`、`10a:137-138` 双重约束：容器/WSL 的数字**不得**写进那个目录。
`tests/benchmarks/src/main.rs:307-311` 会**硬拒非 Windows**——这是设计，不要去改它。

### 5.4 平台分支：**31 处，逐处要有对应证据**

本批要复现的是这 31 处的 Linux/macOS 路径。重点几处：

| 位置 | 内容 |
| --- | --- |
| `cli/ts/src/platform/core.ts:160-217` | `xiao-core` 发现：`where.exe` vs `which`、`;` vs `:`、`.exe` 后缀 |
| `cli/ts/src/platform/toolchain.ts:418-437` | **命令执行**：win32 经 `cmd.exe /c call` + `windowsVerbatimArguments`；其他直接 `execFile` |
| 同上 `:317`、`:359` | Runtime 库名、诊断组件名 |
| `core/rust/.../toolchain.rs:568-680` | POSIX startup shim：`__APPLE__` → `_NSGetExecutablePath`，否则 `/proc/self/exe` |
| `core/rust/.../build.rs:176-180`、`protocol/build.rs:581-587` | **Windows 上先 `remove_file` 再 `rename`**（Unix 语义差异） |
| `cli/ts/src/platform/packaging.ts:101-119` | `bun-{windows,linux,darwin}-{x64,arm64}` 六目标 |
| `cli/ts/src/config/editor.ts:131-133` | 配置路径：`%APPDATA%` / `~/Library/...` / `$XDG_CONFIG_HOME` |

⚠️ **`toolchain.rs:909-937` 的现有测试只断言生成的源码文本包含
`CreateProcessW` / `posix_spawn` / `/proc/self/exe` / `_NSGetExecutablePath`**——
**那是源码字符串断言，不是功能验证**。本批必须用真机端到端替换它，
或至少明确它证明不了什么。

### 5.5 门禁与规范

- 新增的环境依赖测试按 **10D** 写：`#[ignore]` 而非条件 `return`；
  显式 `--ignored` 时缺环境**必须 panic**。
- 不新增 Rust 依赖前先想清楚 `check:lock`（`tests/benchmarks` 是独立 crate）。
- 提交标题带规范前缀、**正文说明为什么**（已由 `.githooks/commit-msg` 兜住）。

### 5.6 2026-09-24 实测记录

#### Linux Docker `linux/amd64`

环境与镜像：

| 项 | 结果 |
| --- | --- |
| 镜像 | `xiao-platform-reproduction:linux-amd64`，镜像摘要 `sha256:196c4439891e619cf1909023027732c5c1c3eb64c98e33482aa96fe02dde9e6c` |
| 系统 | Debian 13（trixie），Linux `x86_64`，运行在 Docker Desktop 的 WSL2 内核上 |
| 工具链 | Rust `1.96.0`；Bun `1.4.1`；clang / `llvm-as` / `llc` `19.1.7` |
| 目标 | `x86_64-unknown-linux-gnu` |

完整命令：

```text
docker build --platform linux/amd64 -f tools/platform-reproduction/Dockerfile -t xiao-platform-reproduction:linux-amd64 .
docker run --rm --platform linux/amd64 -e XIAO_USE_XVFB=1 -v "${PWD}:/workspace" -w /workspace xiao-platform-reproduction:linux-amd64 bash tools/platform-reproduction/reproduce.sh native
```

结果：

- `cargo test --manifest-path core/rust/Cargo.toml --workspace -- --ignored` 在
  `xvfb-run` 下 8 条门控测试全部通过：`n0_a` 3 条、`n0_b_dynamic` 1 条、
  `n0_a_native_driver` 2 条、`n0_b_dynamic_native` 1 条、真实终端测试 1 条；
  合计 `8 passed, 0 failed`。
- `cargo test -p xiao-driver` 通过（默认汇总保留 1 条 ignored）；`bun test` 为
  `82 pass / 1 skip / 0 fail`；`bunx tsc --noEmit`、基准 crate `cargo check`、
  `bun run check`、`cargo fmt --all -- --check` 均通过。
- `bun run check:coverage` 为 `5916/5916`，公共 API 为 `3052/3052`；
  `tests/benchmarks/reports/` 没有性能数字或文件差异。
- 独立 `bun-linux-x64` CLI 打包成功；`xiao build`（含 LLVM 输出）、`-debug build`、
  仓库外 `xiao run` 和 `xiao test` 全部成功。产物为 ELF x86-64；`xiao run` 返回
  `exit_code=0`、`exit_name=success`，`xiao test` 返回 `total=1`、`passed=1`、
  `failed=0`、`exit_code=0`。

这证明 Debian 容器内的 Linux amd64 功能链路和当前架构断言可运行；它不证明裸机
Linux 安装路径，也不把容器结果写入 09R3 性能验收。

#### Linux Docker `linux/arm64`

已执行：

```text
docker run --privileged --rm tonistiigi/binfmt --install arm64
docker buildx build --platform linux/arm64 -f tools/platform-reproduction/Dockerfile -t xiao-platform-reproduction:linux-arm64 --load .
```

`binfmt` 报告 `qemu-aarch64` 已注册，BuildKit 也列出 `linux/arm64`；但镜像在
Dockerfile 的 `RUN apt-get ...` 阶段仍失败：

```text
exec /bin/sh: exec format error
```

因此 ARM64 镜像没有生成，`cargo test` 和 `reproduce.sh` 未开始。该项状态是
**主机 Docker Desktop 的 QEMU/binfmt 执行阻塞**，不是仓库代码失败；`host()` 的
`target_arch` 修复和断言已经提交，但 ARM64 的运行时证据仍待能执行 ARM64 `RUN`
步骤的主机或 CI runner。

#### WSL（Ubuntu 与 Arch）

两个发行版均能启动并挂载工作树，探测命令为：

```text
wsl.exe -d Ubuntu-26.04 -- bash -lc "cd /mnt/d/project/Xiao/Xiao && uname -a && cat /etc/os-release && ldd --version | head -1 && rustc -vV && bun --version && clang --version | head -1 && llvm-as --version | head -1 && llc --version | head -1"
wsl.exe -d archlinux -- bash -lc "cd /mnt/d/project/Xiao/Xiao && uname -a && cat /etc/os-release && ldd --version | head -1 && rustc -vV && bun --version && clang --version | head -1 && llvm-as --version | head -1 && llc --version | head -1"
```

Ubuntu 结果为 Ubuntu 26.04 LTS、WSL2 `6.6.114.1-microsoft-standard-WSL2`、
`x86_64`、glibc `2.43`；Arch 结果为 Arch Linux、同一 WSL2 内核、`x86_64`、
glibc `2.43`。两者都报告 `rustc`、Bun、`clang`、`llvm-as`、`llc` 未安装，
所以没有执行 `reproduce.sh native`，状态记为**环境缺失**而非通过或失败。

#### macOS CI

`.github/workflows/platform-reproduction.yml` 已加入 `macos-14`、Bun `1.4.1`、
Rust `1.96.0` 和 Homebrew LLVM；Linux 安装步骤也补上了 `xauth`。当前没有执行
GitHub Actions runner，因此没有 Mach-O、`osascript` 或 `open -a Terminal` 证据，
macOS 必须继续保持**待复现**。

---

## 六、分步执行记录

| 步 | 实际结果 | 状态 |
| --- | --- | --- |
| **1** | Dockerfile、Linux/macOS 复现脚本、PowerShell 入口和 CI 矩阵已落地；镜像含 C/LLVM、`xauth`、Xvfb 与 xterm | ✅ |
| **2** | Docker `linux/amd64` 全链路复现，含 `xiao build`、独立运行、`xiao test`、`-debug` 和 8 条门控测试 | ✅ |
| **3** | ARM64 目标缺陷已修复并加断言；镜像构建被主机 QEMU 的 `exec format error` 阻塞 | ⚠️ 待 ARM64 runner |
| **4** | Ubuntu/Arch WSL 可启动，但 Rust/Bun/LLVM 全部缺失，未进入复现脚本 | ⚠️ 待准备环境 |
| **5** | 多平台 GitHub Actions 工作流已提交；macOS runner 尚未实际运行 | ⚠️ 待 CI |
| **6** | `10D` 清单已从 7 条修正为 8 条；平台记录、机器状态和 X0 判定在本批同步 | ✅ |

**每一步的验收**：既有测试不改、`tests/benchmarks/reports/` 无 diff、门禁全绿。

---

## 七、最可能翻车的地方

1. **镜像没补 C/LLVM 就去跑 `xiao build`**（§3.1）——会得到一个"工具链缺失"的错误，
   然后被误记成"Linux 不支持 build"。**本批头号翻车点。**
2. **把容器结果写成"跨平台已验证"**（§5.1）——`09b0d:220` 的硬红线，
   `09r3:182` 点名"最容易做假"。
3. **沿用旧表述**（`11x0e:245` 明令「逐项更新，不要沿用旧表述」）。
4. **忘了 arm64**（§4.3）——那正是能抓到真缺陷的那条路径，只跑 amd64 等于白跑。
5. **把 X0 第 3 条混进来记成"平台债"**（§八.2）——它不是平台债，会掩盖一个独立缺口。
6. **碰 `tests/benchmarks/reports/`**（§5.3）。
7. **`-debug` 只验了四个终端候选中的一个却写成"Linux 已验证"**（§4.2）。
8. **镜像变更后与旧记录直接比较**（§3.1 末尾）——`09b0d`/`11x0c` 的两次实测记了镜像指纹。

---

## 八、验收

**关键验收不是「跑通了」**：

1. **每项都有「环境 + 命令 + 结果」三要素**，且**镜像/工具链版本写全**；
2. **`tests/benchmarks/reports/` 无 diff**；
3. **§4.3 的架构缺陷被暴露并修复**，且有「`host()` 与 `target_arch` 一致」的断言
   （撤掉修复该断言必须失败）；
4. **`cargo test -- --ignored` 在每个目标平台各跑一次**，8 条结果逐条记录；
5. **`10D` 的清单从 7 更新到 8**，并说明新增来源；
6. **复现脚本存在且可执行**（§3.2）；
7. **平台记录逐项更新**（`11x0e:245`），**macOS 若未跑就不得写成已验证**；
8. **X0 第 2/4/5/6 条给出明确收口判定**，第 3 条**单独**给出判定或转交（§八.2）；
9. **门禁全绿**，含 `check:lock`、`bunx tsc`。

### 8.1 X0 八条退出条件的现状判定

| # | 条件 | 现状 |
| --- | --- | --- |
| 1 | CLI 接成 `run`/快捷运行/`build`/`test`/`config` | ✅ 已满足（X0-T 补齐了 `xiao test`） |
| 2 | **三平台最小构建矩阵通过** | ⚠️ 未收口：Linux amd64 Docker 功能证据通过，ARM64 被主机 QEMU 阻塞，WSL/macOS 未完成 |
| 3 | **01 至本阶段的"已确定"规则都有自动化规格测试** | ⚠️ 独立规格测试债，登记为 `X0-SPEC-001`，不属于本批平台债 |
| 4 | CLI 用 TS 构建并通过静态检查；**三平台入口行为一致** | ⚠️ Linux amd64 静态检查与入口回环通过；ARM64、WSL、macOS 仍缺证据 |
| 5 | **三平台独立 `xiao` 可执行并能调用内核** | ⚠️ Linux amd64 ELF 独立产物与核心发现通过；ARM64/macOS 未完成，不能收口 |
| 6 | **三平台均能发现并调用兼容版本核心** | ⚠️ Linux amd64 同目录发现与开发回环通过；WSL 工具链缺失，macOS 未运行 |
| 7 | `xiao config` 布尔结构化写入 | ✅ 已满足 |
| 8 | `-debug` 传递与独立窗口 | ⚠️ Windows 原生满足；POSIX 只有源码文本断言 |

### 8.2 ⚠️ X0 第 3 条**不是平台债**，要单独处理

`12-tests:695` 要求「从阶段 01 到本阶段已经实现的所有'已确定'规则都有自动化规格测试」。
实测：

- `tests/spec/` 只有 `01-lexical`、`02-parser`、`03-expression`、`04-types`、
  `05-containers`、`06-modules`、`09-bytecode`、`11x0-protocol`；
- **没有 07（错误模型/并发）、08（前端流水线）、10（原生后端）、11/11X0 的 spec 目录**；
- `tests/unit/` 与 `tests/integration/` **只有 README**；
- **全仓没有任何文档宣称这条已关闭**。

**本批不补这组规格测试，也不把它记成平台债。**现状登记为后续具名债项
`X0-SPEC-001`：为阶段 07、08、10、11/11X0 已确定规则建立对应的自动化规格
测试目录与执行入口，并在补齐前保持 X0 第 3 条未满足。后续批次接手时不能以
本批 Linux/Windows 功能证据替代这项规格覆盖。

### 8.3 落地记录的形态（照 `09R3` 的先例）

`09R3` 带平台债关门时做了五件事，**本批照抄**：

1. **债进产物不只进文档**：`09r3-freeze.json:4-7` 的 `platform_status` 机器字段；
2. **退出条件里点名**并解释"最容易做假"的那条（`09r3:182`）；
3. **落地记录做双重否定**：Linux Docker amd64 的功能证据已落地，但 ARM64、WSL
   和 macOS 仍明确标为阻塞或待复现；
4. **下游批次引用这笔债**而不是假装已清（`09b0:116`）；
5. **环境矩阵逐项表**（`11x0c:183-191` 的五项）。

---

## 九、不负责与不要重复做的事

- **不采 09R3 的性能数字**——本批只要功能证据（§5.1、§5.3）。
- **不改 `tests/benchmarks`**——`src/main.rs:307-311` 硬拒非 Windows 是**设计**。
- **不做 `.app` / 签名 / 公证 / Apple SDK 检测**——那些归 `11:249` 的后续工作。
- **不做 X0 第 3 条以外的规格测试扩张**（§8.2 单独处置）。
- **不顺手改 X0-T 之外的功能语义**——本批是**证据批次**，没有新功能。
- **不把 X0-T 的平台债漏掉**：`11x0t-project-test-semantics.md` 已补入三平台
  `xiao test` 复现清单，但不改变 X0-T 已冻结的测试发现、排序、退出码和结果协议语义。
- **不改 `Dockerfile.dev` 的验收地位注释**：它写明「这不是验收路径的一部分」，
  补了 C/LLVM 之后**这条仍然成立**。

---

## 相关页面

- [11X0-E. `xiao build` 与主机工具链发现](11x0e-build-and-toolchain.md) §5.5 —— **平台清单权威原文**
- [11X0-C. 独立可执行与平台矩阵](11x0c-packaging-and-platforms.md) §三 —— **验证 ≠ 验收的分级口径**
- [09R3. 跨平台基准与冻结](09r3-benchmarks-and-freeze.md) §六 / §十一 —— 容器约束与带债关门先例
- [11X0-D. `-debug` 与诊断窗口](11x0d-debug-diagnostics-window.md) §3.2 —— 平台终端启动
- [10D. 环境依赖测试规范](10d-environment-gated-test-spec.md) —— `#[ignore]` 规范（清单待更新为 8）
- [10C. 原生运行时链接修复](10c-native-runtime-link-fix.md) —— Linux/macOS 依赖库适配的来源
- [10A. LLVM 原生构建闭环](10a-n0-native-closure.md) `:131-138` —— 容器镜像需要 C/LLVM
- [12. 测试与开发里程碑](12-tests-and-milestones.md) `:689-700` —— X0 八条退出条件
- [11X0-T. 项目测试语义与结果协议](11x0t-project-test-semantics.md) —— 其平台债由本批补录

# 11X0-P1. 跨平台复现收口（Linux 原生 / WSL / macOS）

> **本批是 [11X0-P](11x0-platform-reproduction.md) 的续批。** 上一批完成了
> Linux Docker amd64 的完整功能复现、修掉了 `target.rs` 的架构硬编码、
> 补齐了复现脚本与 CI 工作流。启动本批时仍有三项未完成：ARM64（被主机 QEMU 阻塞）、
> WSL（工具链缺失）、macOS（CI 建好但从未运行）。目前 Windows PowerShell 5.1、
> Ubuntu/Arch WSL 和 GitHub Actions 四平台工作流均已通过；本机 Docker arm64 仿真仍受阻，
> macOS 真实终端测试仍按无 GUI 规则显式跳过。
>
> **本批的目标是让 X0 第 2/4/5/6 条真正收口**——或者按 `09R3` 先例明确带债关门。
> **两条路都行，但不许含糊**（`11x0c:178-179`）。

## 一、Agent 交接上下文

### 接手前提

1. [11X0-P. 跨平台复现](11x0-platform-reproduction.md) —— **本批的直接前置**。
   §2 的四项手段与证据等级、§5.1 的「验证 ≠ 验收」分级、§5.6 的实测记录、
   §8.1 的 X0 收口判定表，**本批全部沿用**。
2. [11X0-C. 独立可执行与平台矩阵](11x0c-packaging-and-platforms.md) **§三**（`:150-179`）
   —— X0 第 2 条「完整达成」的两种合法收法（真机复现 **或** 写明容器证据为何足够）。
3. [09R3. 跨平台基准与冻结](09r3-benchmarks-and-freeze.md) **§六 / §十一**
   —— 容器约束原文；**「带平台债关门」的模板**。
4. [10D. 环境依赖测试规范](10d-environment-gated-test-spec.md) **§四**（`:101-144`）
   —— **Linux 侧环境变量的准备方式已经补上了**（上一批做的），本批直接用。
5. [11X0-E. `xiao build` 与主机工具链发现](11x0e-build-and-toolchain.md) **§5.5**（`:232-245`）
   —— 平台清单权威原文。**`:245` 明令「逐项更新，不要沿用旧表述」。**
6. [12. 测试与开发里程碑](12-tests-and-milestones.md) `:689-700` —— X0 八条退出条件。

### 现状盘点（2026-09-24 实测）

```text
本地状态   main 分支，Windows PowerShell 5.1 与 Ubuntu/Arch WSL 本地复现已通过
CI 状态    运行 35955788547 已完成，linux-amd64/linux-arm64/macos-arm64/windows-amd64 全部成功
WSL 状态   Ubuntu-26.04 与 archlinux 均已具备 Rust 1.96.0、Bun 1.4.2、clang/LLVM、xterm、Xvfb、rustfmt
未收口     X0 第 8 条的 macOS 真实终端握手；本机 Docker arm64 镜像仍受 QEMU 执行层阻塞
```

### 本批交付与不负责

**交付**：CI 多平台证据（含**原生 arm64**）、WSL 工具链与功能证据、
X0 第 2/4/5/6 条的**明确收口判定**。

**不负责**：**`X0-SPEC-001`**（`12-tests:720` 已登记为独立债项，见 §六.3）；
**X0 第 8 条的 POSIX 端到端**；09R3 的性能数字。

---

## 二、第一道闸：推送（**已满足；后续新提交仍需推送**）

`workflow_dispatch` 的工作流**必须在默认分支存在**，才会出现在 Actions UI 里。
用户已将前置提交推送到默认分支，GitHub Actions 页面已经能看到
`Xiao platform reproduction`，因此本批可以继续修复并提交工作流。

```
当前前置状态：origin/main 与本地 `15c8b06` 同步；工作流已出现在 Actions UI，尚无运行记录。
```

本批修复完成后仍需把新提交推送到默认分支，才能运行更新后的工作流；推送前继续确认
工作树、提交正文和差异范围，禁止强制推送。

---

## 三、⚠️ 动手前必须修的 CI 缺陷

**这些是「看起来能跑但实际会失败」的地方**——不先修，跑出来的失败会被误记成"平台不支持"。

### 3.1 `reproduce.ps1:79-80` 的 `utf8NoBOM` **已修复**

```powershell
Set-Content -Encoding utf8NoBOM ...
```

工作流 `:69` 用 **`powershell -ExecutionPolicy Bypass -File`** 调用——
Windows 上 `powershell.exe` 固定是 **Windows PowerShell 5.1**，
而 **`utf8NoBOM` 是 PowerShell 6+ 才有的枚举值**。
配合脚本开头的 `$ErrorActionPreference = 'Stop'`（`:8`），会直接终止。

**实际修法**：脚本改用兼容 Windows PowerShell 5.1/PowerShell 7 的
`System.IO.File.WriteAllText` + 无 BOM UTF-8；CI 同时改用 `pwsh -File`，并将脚本诊断文本
保持 ASCII，避免 PS5.1 把无 BOM UTF-8 按本地代码页误解析。

Windows PowerShell 5.1 已在本机完整执行入口并通过；这项风险已由真实运行关闭，
不能再把 Windows 入口记成未验证。

### 3.2 `reproduce.ps1` 缺少三项 **已补齐**

| 缺失项 | `reproduce.sh` 的位置 |
| --- | --- |
| `--emit-llvm` 产物检查 | `:161-166` |
| `-debug build` | `:168-173` |
| `file` 输出 | `:190-192` |

Windows 侧现在同步检查 `--emit-llvm` 输出、`-debug` 产物和 PE/COFF 格式；
本机 Windows PowerShell 5.1 已完整执行并通过。

### 3.3 macOS runner 的**真实终端测试**采用显式降级

`.github/workflows/platform-reproduction.yml:19-21` 的 `macos-14` runner **无 GUI 会话**，
而第 8 条门控测试 `real_terminal_session_is_environment_gated`
（`core/rust/crates/xiao-driver/src/diagnostics.rs:844`）走 `osascript` / `open -a Terminal`。
`reproduce.sh` 支持 `XIAO_SKIP_REAL_TERMINAL_TEST=1`；macOS CI 设置该变量并通过
`cargo test -- --ignored --skip real_terminal_session_is_environment_gated` 明确跳过真实
终端测试。它会在日志中写出跳过原因，**不计为通过**；其余 7 条环境门控测试仍照常执行。
工作流的 `timeout-minutes` 作为最后防线。

### 3.4 Windows 侧的 LLVM 来源已明确

Windows runner 的官方 LLVM 包不可靠地提供 `llvm-as` / `llc`，因此 CI 改为使用
`msys2/setup-msys2@v2` 的 UCRT64 环境，安装 `mingw-w64-ucrt-x86_64-clang` 与
`mingw-w64-ucrt-x86_64-llvm`，再把 `ucrt64\bin` 加入当前 job 的 PATH。工作流通过
`vswhere.exe` 自动导入 Visual Studio 的 `vcvars64.bat`，确保 MSYS2 clang 使用 MSVC 的
`link.exe` 和头文件；`35955788547` 的 Windows job 实测 clang `22.1.8`、目标
`x86_64-pc-windows-msvc`，LLVM 输出、PE/COFF 和完整复现均通过。10D 的 MSYS2 准备方式
与 CI 现在保持同一工具链来源，Chocolatey 只负责提供 `file` 命令。

### 3.5 `ubuntu-24.04-arm` 的可用性前提

该 runner **仅对公开仓库免费**；私有仓库需要 arm64 larger runner，否则作业会排队或失败。
`Programming-Language-Xiao/Xiao` 是公开仓库（`git remote -v` 可查），**当前没问题**，
但要在记录里写明这个前提。

### 3.6 证据留存已补齐

工作流现已加入 `actions/upload-artifact`、`timeout-minutes` 和 `concurrency`。
而 `reproduce.sh` 的 stdout（`:94-100` 的环境七项、`:161-188` 的协议 JSON）
是**唯一证据载体**。
「环境 + 命令 + 结果」三要素会随日志过期丢失。

每个平台的日志写入 `artifacts/<platform>.log` 并上传 30 天，收口判定可以追溯。

---

### 3.7 `actions/checkout` 已升级

用 `actionista` 技能的当日版本索引（`actions-index.json`，264 个动作）核对：

| Action | 工作流用的 | 索引最新 | 状态 |
| --- | --- | --- | --- |
| **`actions/checkout`** | **`@v7`** | **`v7.0.1`** | ✅ 已离开弃用区间 |
| `oven-sh/setup-bun` | `@v2` | `v2.2.0` | ✅ 大版本正确 |
| `dtolnay/rust-toolchain` | `@1.96.0` | `v1` | ✅ **用版本号正是该 action 的推荐用法**，不要改成 SHA |

工作流已按当日索引升级到 `@v7`；Rust toolchain action 仍保留其推荐的
`@1.96.0` 写法，Bun 仍使用 `@v2`。

### 3.8 三个顶层字段已补齐

```yaml
permissions:
  contents: read            # ← 当前没写，token 权限偏宽

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true  # ← 当前没写

jobs:
  reproduce:
    timeout-minutes: 45
```

macOS 真实终端测试通过显式变量跳过，`timeout-minutes: 45` 仍保留为 job 级最后防线。

### 3.9 Rust 缓存已加入

四个平台各编一次整个 workspace。两条路：

- **社区标准**：`Swatinem/rust-cache@v2.9.2`（索引确认**未弃用**）——一行搞定；
- **手写**：`actions/cache@v6.1.0` 缓存 `~/.cargo/bin/`、`~/.cargo/registry/`、
  `~/.cargo/git/`、`target/`，key 用 `cargo-${{ hashFiles('**/Cargo.lock') }}`。

⚠️ **本仓有两份 `Cargo.lock`**（`core/rust/Cargo.lock` 与 `tests/benchmarks/Cargo.lock`，
后者是独立 crate）——用 `hashFiles('**/Cargo.lock')` 一并覆盖，
**别只写 `core/rust/Cargo.lock`**，否则 benchmarks 改了不会失效缓存。

工作流使用 `Swatinem/rust-cache@v2`，同时覆盖 `core/rust` 和独立的
`tests/benchmarks` workspace；缓存不是验收证据，只用于降低四平台重复编译成本。

---

## 四、三条路径

### 4.1 CI（**macOS 与原生 ARM64 的正解**）

`linux-arm64` 用 **`ubuntu-24.04-arm`——原生 arm64 runner，不经 QEMU**，
**直接绕开上一批 `exec /bin/sh: exec format error` 的阻塞**。
这正是 `target.rs` 架构断言真正被编译执行的地方
（`tools/platform-reproduction/README.md:18-19` 明说 `linux/arm64` 不能省略）。

顺序建议：先 `linux-amd64` + `linux-arm64`（快、确定性高），
再加 `macos-arm64`（有 §3.3 的不确定性），最后 `windows-amd64`（有 §3.1 的确定性失败，修完再说）。

**证据要求**：四平台的日志/输出**落 artifact**，并在记录里写明
runner label、镜像版本、工具链版本。

### 4.2 WSL（真内核 Linux 证据）

两个发行版已存在（`Ubuntu-26.04`、`archlinux`，WSL2 `6.6.114.1`、`x86_64`、glibc `2.43`），
已按 `10d:132-144` 补齐工具链与真实终端依赖：

**Ubuntu-26.04**
```
sudo apt-get update && sudo apt-get install --yes clang llvm lld build-essential xvfb xterm
sudo update-alternatives --set x-terminal-emulator /usr/bin/xterm
```
**archlinux**
```
sudo pacman -S --needed base-devel clang llvm lld xterm xorg-xauth xorg-server-xvfb
```

Arch 使用 `xterm` 作为真实终端候选，并使用 `xorg-server-xvfb` 提供 Xvfb；
这两项已在两套 WSL 的复现中实际生效。

**Rust 与 Bun 两边都要装，且版本要钉**：

- Rust：**用 `rustup` 装 1.96.0**（Arch 的 `rust` 包版本不受控，不满足"钉 1.96.0"）；
- Bun：`curl -fsSL https://bun.sh/install | bash` → **必须 ≥ 1.4.1**
  （1.4.0 在 `--compile` 有 ELF 临时文件权限回归，`README.md:40`）。
- Rust 工具链还必须安装 `rustfmt`：`rustup component add rustfmt --toolchain 1.96.0-<host>`。

⚠️ **一个容易踩的点**：`reproduce.sh` 从**仓库根**调用 cargo，而 `rust-toolchain.toml` 在
`core/rust/` 下。**rustup 按 CWD 向上找工具链文件，从仓库根找不到** →
不设 `rustup default 1.96.0` 就会用错版本，**"Rust 1.96.0"这条证据就不成立**。

装完按 `10d:132-144` 导五个环境变量（`XIAO_CLANG`/`XIAO_LLVM_AS`/`XIAO_LLC`/
`XIAO_RUNTIME_LIBRARY`/`XIAO_TARGET_TRIPLE`），并以 `XIAO_USE_XVFB=1 bash
tools/platform-reproduction/reproduce.sh native` 执行完整复现。

⚠️ `XIAO_TARGET_TRIPLE` **必须等于 `rustc -vV` 的 `host` 行**，不得沿用 Windows 的
`x86_64-pc-windows-msvc`——`reproduce.sh:50-54` 把这条做成了硬断言。

⚠️ **措辞不一致顺手统一**：`10d:49` 说"4 个环境变量"，实际用 **5 个**（多 `XIAO_LLC`）。

### 4.3 收口判定

**共同前提（四条都适用）**：

1. **证据三要素**：环境 + 完整命令 + 结果（字段模板见 `tools/platform-reproduction/README.md:53-61`）；
2. **与 Windows 原生的逐项差异**——**没有差异也要写"无差异"**；
3. **禁止未跑先改状态**（`09r3:182` 点名"最容易做假"）；
4. **不得写"跨平台已验证"**（`09b0d:210-220` 的硬红线）；
5. `tests/benchmarks/reports/` **不产生数字 diff**（**状态字符串可以改，见 §五.3**）。

| 条 | 完成标志 |
| --- | --- |
| **第 2 条** | 三平台各自跑通最小构建矩阵。**要么真实主机复现，要么写明容器/WSL 证据为何足够**——这个判断必须写出来 |
| **第 4 条** | 三平台：`bunx tsc --noEmit` + `bun run check` + **命令入口行为一致**（`xiao test` 的 `total/passed/failed/exit_code` 机器字段逐平台一致） |
| **第 5 条** | 三平台各自产出**不依赖 Node.js 的独立 `xiao`**，且**复制到仓库外 + 清 `XIAO_CORE_PATH` + 最小 PATH** 仍能 `run`。注意还要"正确调用编译器内核"，**不能只验 `--version`** |
| **第 6 条** | 三平台各验**三条发现路径**（同目录 / PATH / 开发回环）**+ 版本协商失败路径**（失配必须拒绝、结构化错误码为 `X11-PROTOCOL-004`，**不解析本地化文本**）。上一批只验了同目录与开发回环两项 |

---

## 五、硬约束

### 5.1 沿用 [11X0-P §5.1](11x0-platform-reproduction.md) 的证据分级

功能结果有效；**性能数字不得进 `tests/benchmarks/reports/`**；
**不得据此宣称"跨平台已验证"**。

### 5.2 未跑完的**原样保留**

`platform_status` 里没跑到的平台字符串**原样不动**（`11x0:208-212`）。
**`excluded-from-acceptance` 这半句去掉即违约**（`09r3:135`、`09b0d:210-220`）。

### 5.3 ⚠️ `09r3-freeze.json` 的**状态字符串可以改，数字不能改**

上一批的实际做法是：`d073ae0` 只改了 `platform_status` 三行状态串，
**冻结七项（`selected_machine: "stack"`、`format_version`、`opcode_range` 等）一个字没动**。

**本批沿用这条解释**——因为 `11x0e:245` 明令"逐项更新"，
而 `09r3:214` 把这个文件定义为「冻结七项、**平台状态**和机型选择」。
⚠️ [11X0-P §5.3](11x0-platform-reproduction.md) 的措辞「不要产生 diff」**划得过宽**，
正确口径是「**数字不得写入；状态更新是必要的**」。

ARM64 已由 `ubuntu-24.04-arm` 原生 runner 完成，`linux` 状态串已写明原生 amd64/arm64
功能证据；本机 Docker arm64 的 QEMU 阻塞只保留在复现记录，不作为 Linux 平台结论。

### 5.4 更正上一批的一处措辞

`11x0-platform-reproduction.md:393` 写「没有 07、08、10、**11/11X0** 的 spec 目录」——
**与盘上事实不符**：`tests/spec/11x0-protocol/` 存在且已被双侧测试读取
（`xiao-driver/tests/x0_a_protocol.rs:10-22`、`cli/ts/src/protocol/protocol.test.ts:8`）。

**应更正为**：缺 **07、08、10、11** 四个 + **`06-modules/` 有目录无执行入口**这一条。

### 5.5 环境依赖测试按 [10D](10d-environment-gated-test-spec.md) 写

`--ignored` 时缺环境**必须 panic**，不许条件 `return`。
WSL 证据的边界：共享宿主调度，**性能数字不得与 Windows 原生并列**。

---

## 六、分步提交与实际结果

| 步 | 内容 | 为什么这个顺序 |
| --- | --- | --- |
| **1** | 修 `reproduce.ps1` 的 `utf8NoBOM`（§3.1）+ 补三项（§3.2） | ✅ Windows PowerShell 5.1 已实测通过 |
| **2** | 给 CI 加 artifact 上传 / 超时（§3.6、§3.3） | ✅ 工作流已补齐证据留存、并发和超时 |
| **3** | **推送**（§二），触发 `linux-amd64` + `linux-arm64` | ✅ 运行 `35955788547` 成功；原生 arm64 runner 绕开主机 QEMU |
| **4** | 触发 `macos-arm64`、`windows-amd64` | ✅ 同一运行成功；macOS 真实终端测试按显式跳过记录 |
| **5** | WSL：装工具链（§4.2）并 `reproduce.sh native` | ✅ Ubuntu/Arch 两套 WSL 各自完整通过 |
| **6** | 逐项更新 `11x0e §5.5`、`11x0p §5.6/§8.1`、`09r3-freeze.json` 状态串；更正 §5.4 | ✅ CI 结果与四平台证据已写回；性能数字未改 |

---

## 七、最可能翻车的地方

1. **没修 `utf8NoBOM` 就跑 Windows**（§3.1）——得到一个 PowerShell 参数错误，
   却被记成"Windows 复现失败"。
2. **把 CI 的失败当成"平台不支持"**——先分清是**脚本缺陷**还是**平台缺陷**（§3 全部）。
3. **macOS 的终端测试挂起**（§3.3）——没有超时会一直等到 job 超时，
   而且**不算通过**。
4. **WSL 用错 Rust 版本**（§4.2 末尾）——从仓库根跑 cargo 找不到 `rust-toolchain.toml`，
   "Rust 1.96.0"的证据不成立。
5. **未跑先改状态**（§5.2）——`09r3:182` 点名的"最容易做假"。
6. **把 `09r3-freeze.json` 的状态更新当成违规而不敢改**——§5.3 说明了两者边界；
   反过来，**往里面写数字**才是违规。
7. **Arch 漏装终端**（§4.2）——第 8 条门控测试必失败，且容易被误读成"Arch 不支持"。
8. **推送未获授权**（§二）——这是外向动作。

---

## 八、验收

1. **CI 四平台的日志/产物已留存**，记录里有 runner label、镜像版本、工具链版本；
2. **`reproduce.ps1` 的 `utf8NoBOM` 已修**，且 Windows 侧三项补齐（§3.2）；
3. **原生 arm64 结果存在**，且 `target.rs` 的架构断言在 arm64 上**真正被编译执行**；
4. **WSL 两个发行版各有功能证据**，且明确说明共享宿主调度边界；
5. **X0 第 2/4/5/6 条各有明确判定**——收口或带债关门，**理由写出来**；
6. **`09r3-freeze.json` 只改状态字符串，无数字改动**；
7. **`11x0e §5.5` 逐项更新**（`:245` 明令不沿用旧表述）；
8. **`11x0p:393` 的 11X0 措辞已更正**（§5.4）；
9. **门禁全绿**，含 `check:lock`、`bunx tsc`。

### 8.1 落盘形态（照 `09R3` 先例）

1. **债进产物**：`09r3-freeze.json` 的 `platform_status` 机器字段；
2. **退出条件点名**并解释"最容易做假"的那条（`09r3:182`）；
3. **落地记录做双重否定**：GitHub Actions 原生 Linux amd64/arm64 与 macOS arm64 功能证据
   已落地，但本机 Docker arm64 仿真仍受阻，macOS 真实终端测试仍明确标为未验证；
4. **下游批次引用这笔债**而不是假装已清（`09b0:116`）；
5. **环境矩阵逐项表**（`11x0c:183-191` 的五项）。

---

## 九、不负责与不要重复做的事

### 9.1 `X0-SPEC-001` **不在本批**

`12-tests:720` 与 `11x0p:397-400` 已把它登记为**独立具名债项**：
为阶段 **07、08、10、11/11X0** 的已确定规则建立自动化规格测试**目录与执行入口**。

⚠️ **真正的门槛不是"建目录"而是"有执行入口"**：`tests/spec/06-modules/` 就是现成反例——
目录和夹具都在，**但没有任何测试代码读取它**。
**若只建 07/08/10/11 的 JSON 而不接测试，等于复制同一笔债。**

**本批不补它，但也不得把它记成平台债。**

### 9.2 其余

- **不采 09R3 的性能数字**（§5.1）。
- **不改 `tests/benchmarks/src/main.rs:307-311`** 的非 Windows 硬拒——那是设计。
- **不动 X0 第 8 条的 POSIX 端到端**（`toolchain.rs:909-937` 的源码字符串断言
  要等真机端到端替换，属独立工作）。
- **不把 Docker amd64 的既有结论重跑一遍**——上一批已完成，本批只补缺口。

---

## 相关页面

- [11X0-P. 跨平台复现](11x0-platform-reproduction.md) —— **本批的直接前置**
- [11X0-C. 独立可执行与平台矩阵](11x0c-packaging-and-platforms.md) §三 —— 验证 ≠ 验收
- [09R3. 跨平台基准与冻结](09r3-benchmarks-and-freeze.md) §十一 —— 带债关门的模板
- [10D. 环境依赖测试规范](10d-environment-gated-test-spec.md) §四 —— Linux 环境准备方式
- [11X0-E. `xiao build` 与主机工具链发现](11x0e-build-and-toolchain.md) §5.5 —— 平台清单权威
- [12. 测试与开发里程碑](12-tests-and-milestones.md) `:689-700` —— X0 八条退出条件

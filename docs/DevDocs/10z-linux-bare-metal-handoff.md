# 10Z-Linux. Z-2 裸机轮次交接：`19.14` C 档窗口取证

> **写给谁**：在 Ubuntu 机器上执行本轮的操作者（人，不是 Agent）。
> **这一轮要什么**：`19.14` 的 **C 档**——诊断窗口在 Linux 桌面上**确实出现并被截图**。
> **这一轮不做什么**：**不做性能取数**（原因见 §〇，这是本轮最重要的前提）。
>
> 状态：**待执行**。上级文档 [10Z-收尾](10z-closeout-execution.md) §2.3；`19.14` 原文见 [19](19-optimization-release.md)。

## 〇、先读这一节：本轮能做什么、不能做什么

**能做：C 档窗口取证。** 10V 的窗口保持已落地，窗口在会话结束后会保留 5 秒——这正是前两轮
「闪现即关、拿不到截图」的成因，现在条件具备了。

**不能做：性能取数（O6 条件 5）。** 审核 2026-10-10 实测：**Java 侧与 LLVM 原生侧的计时驱动器都不存在**。

证据（都可复核）：

| 事实 | 复核方式 |
| --- | --- |
| LLVM 原生侧只有**功能探针** | `xiao-driver/tests/native_benchmark_probe.rs` 只输出 `{id}:built` 或 `{id}:native-rejected:{error}`；container-dense 路径自述「**非性能取数**」 |
| Java 侧只输出**值、不计时** | `tests/benchmarks/java/README.md` 的契约是 `success<TAB><value>` |
| **全仓库没有跑 `javac`/`java` 的驱动器** | `grep -rln "javac\|java -Xms\|Benchmark.class"` 在 `*.sh`/`*.ps1`/`*.rs`/`*.ts`/`*.yml` 中**零命中** |
| **没有对 `.exe` 重复计时的地方** | `grep -rln "Measure-Command\|perf_counter"` 零命中 |
| 冻结的协议**只被声明、没有被实现** | `tests/benchmarks/baseline.json` 写了 `warmup_iterations:3` / `measurement_iterations:11` / `percentile-bootstrap` / `10000` / `seed 19015`，但**没有对应的驱动器**；唯一计时的 `tests/benchmarks/src/main.rs` 测的是**三种字节码机载体（VM 侧）**，其 README 明说「没有……LLVM 原生对照」 |

**所以：不要在计时驱动器就绪之前把这一轮当成「C 档 + 取数」一起跑**——那会**第三次白跑**。
本轮的方案见 §七（三选一），需要星崽先定。

## 一、这一轮要产出的东西

| 产物 | 判据 | 归属 |
| --- | --- | --- |
| **窗口出现的截图** | 肉眼可见的窗口 + 截图 + 原始输出 | `19.14` C 档（`19` 的三平台条目里** Linux 那一格**） |
| 窗口相关的环境记录 | 见 §五 清单 | 若仍无窗口，用于定位 |
| results + log 双文件 | 见 §六 | 归档 |

**`19.14` 是三平台条目**：Windows 已有证据；**macOS 已定为「不可验证」**（无 Mac 宿主、CI 显式跳过真实终端）；
本轮补的是 **Linux**。所以本轮**不涉及 macOS**，也不要试图在 macOS 上做。

## 二、检出提交

```bash
git fetch origin
git checkout main && git pull --ff-only
git log --oneline -1        # 回传里要写明实际 sha
```

参考：截至本文撰写时为 `d23751b`。**以实际检出为准，并把 sha 写进回传。**

## 三、前置环境

裸机或正常安装的 Ubuntu（**不是 WSL，不是容器**）。

```bash
sudo apt-get update
sudo apt-get install --yes clang lld llvm build-essential xauth xterm file
```

工具链版本要与仓库一致：

```bash
rustup toolchain install 1.96.0
rustup component add rustfmt clippy --toolchain 1.96.0
bun --version          # 需要 1.4.x
```

**本轮必须记录的环境**（前两轮没记全，导致无法定位）：

```bash
which x-terminal-emulator && readlink -f "$(command -v x-terminal-emulator)"
which gnome-terminal konsole xterm
echo "DISPLAY=$DISPLAY  WAYLAND_DISPLAY=$WAYLAND_DISPLAY  XDG_SESSION_TYPE=$XDG_SESSION_TYPE"
```

诊断窗口按 **`x-terminal-emulator` → `gnome-terminal` → `konsole` → `xterm`** 的顺序尝试启动
（见 `xiao-driver` 的 `terminal_candidates`）。**上一轮「测试 ok 但无窗口」很可能就出在第一个候选上**——
例如 `x-terminal-emulator` 指向 GNOME 终端时，它在 Wayland 下需要 D-Bus 会话，可能启动即退出而不显示。
这些路径信息是本轮定位的关键。

## 四、C 档：窗口取证（本轮重点）

### 4.1 ⚠ 不要设 `XIAO_DIAGNOSTICS_HOLD_MS=0`

**这是前两轮失败的成因，务必注意。**

`XIAO_DIAGNOSTICS_HOLD_MS` 控制窗口在会话结束后**保留多久**（默认 **5000 ms**）。
**设为 `0` 就是关掉保持**——窗口会立刻关闭，**截图必然失败**。

**注意一个陷阱**：仓库里的 `tools/platform-reproduction/reproduce.sh` **第 121 行会
`export XIAO_DIAGNOSTICS_HOLD_MS=0`**。所以：

- **本步不要用 `reproduce.sh` 跑**，用手工命令（下面给了）；
- 如果你先跑过 `reproduce.sh`，**开一个新终端**再做本步（export 只在那个脚本进程内，但换个终端最保险）；
- **不要**出于「脚本这么设我也这么设」的联想，自己 `export XIAO_DIAGNOSTICS_HOLD_MS=0`。

想留更久可以**临时调大**（上限 3600000 ms），例如 `export XIAO_DIAGNOSTICS_HOLD_MS=15000`。
**这是取证用法，不是改默认值。**

### 4.2 先记录现状（在跑之前）

把 §五 的环境清单**先落盘**。**不要跑完再补**——事后回忆出来的环境值不算受控记录。

### 4.3 跑并观察

准备一个短程序（窗口结束后会保留 5 秒，所以短程序也能取证）：

```bash
cd <仓库根>
cat > /tmp/window-probe.xiao <<'EOF'
print("window-probe")
EOF
xiao run -debug /tmp/window-probe.xiao
```

（若仓库里没有名为 `xiao` 的独立 shim，用同一入口：
`bun cli/ts/src/main.ts run -debug /tmp/window-probe.xiao`。）

**在程序运行以及结束后的保持期间观察**，并记录窗口是否显示最终状态、关窗后进程的退出行为。

### 4.4 若窗口仍未出现，做一次对照

```bash
# 另开一个终端，在程序跑到诊断阶段时执行
pgrep -a xiao-diagnostics
pgrep -a xterm
pgrep -a x-terminal-emulator
```

再按 §三 记的候选顺序**强制换一个渲染器**重试，各记一次结果（例如把 `x-terminal-emulator`
临时指向 `/usr/bin/xterm` 再跑一遍）。

### 4.5 什么算通过，什么算未通过

| 观察到 | 判定 |
| --- | --- |
| **窗口确实出现**，且**有截图** | **通过** |
| 仍无窗口 / 无截图 | **未通过**——写下当时的观察与下一步假设 |

**「测试报 ok」不算通过。** 前两轮就是这么记成未通过的：测试断言通过了，但操作者没看到窗口。
**本轮要的是肉眼可见 + 截图。**

## 五、要记录的环境清单（跑之前落盘）

```text
CPU 型号与核数：
内存：
Ubuntu 版本与内核（uname -a）：
桌面环境与显示协议（X11 / Wayland）：
rustc 版本（rustc -vV 的完整 host 行）：
clang / llvm 版本：
bun 版本：
x-terminal-emulator 指向：
gnome-terminal / konsole / xterm 各自是否存在：
DISPLAY / WAYLAND_DISPLAY / XDG_SESSION_TYPE：
执行时间点（含时区）：
```

## 六、要回传的东西

**沿用前两轮的双文件格式**，一起回传：

1. **`10z-…-results-<日期>.md`**——判据与结论并列；
2. **`10z-…-log-<日期>.md`**——**原始输出**（命令与输出原样贴，不要只给摘要），含：
   - §五 的环境清单；
   - `xiao run -debug` 的完整输出；
   - `pgrep` 的输出；
   - **截图文件**；
   - 强制换渲染器那次对照的输出（若做了）。

**只要结论不要原始输出的回传会被退回**——本项目此前多次栽在「只写摘要」上。

## 七、先别做的事（本轮方案待定）

**本轮的取数部分**有三种处置，**需要星崽先定**：

- **(A) 先补计时驱动器，再开轮次**——推荐。驱动器要做的事：把 `manifest.json` 的每个程序
  用 `xiao build` 建成产物、按协议跑（预热 3 / 测量 11）、同时跑 Java 侧、按
  `baseline.json` 的 percentile-bootstrap 统计。**属实现工作**，建议单独立项或并入 10Z 后续。
- **(B) 本轮只做 C 档 + 环境清单**，取数等驱动器就绪后另开一轮——代价是多一次机器往返。
- **(C) 由操作者手工计时**——**不建议**：协议要 percentile-bootstrap 与固定 seed，
  手工产不出可复核的数字；那会让「取数」变成「非受控数字」，与 [10Y](10y-b-series-triage-rework.md) §2.7
  第 4 条（沿用 19D 既定统计口径）冲突。

**在方案定下来之前**：

- **不要**用 `time` / 秒表凑一组数字回传；
- **不要**用本机或 CI 的数字充当受控取数；
- **不要**在 macOS 上做 `19.14`（已定「不可验证」）；
- **不要**用 `reproduce.sh` 跑 C 档那一步（§4.1 的陷阱）；
- **不要**装或使用 Xvfb——真实 X 会话是这一步的前提。

## 八、回传模板

```text
检出提交（git log --oneline -1）：
本轮目标：19.14 C 档（Linux）

== 环境（跑之前落盘）==
<粘贴 §五 的清单>

== C 档结果 ==
窗口是否出现：是 / 否
截图文件名：
xiao run -debug 的完整输出：
<粘贴>
（若未出现）pgrep 输出：
<粘贴>
（若未出现）换渲染器的对照与结果：
<粘贴>
判定：通过 / 未通过
若未通过，你的观察与下一步假设：

== 其他 ==
遇到的问题：
不确定的地方：
```

## 相关页面

- [10Z-收尾. 10 系列收束](10z-closeout-execution.md) §2.3 —— 本轮的口径来源（C 档判据、保持用法、不设 Xvfb）
- [10T-Linux. 上一轮裸机交接](10t-linux-bare-metal-handoff.md) —— 上一轮（**勿按它执行**，它没有保持窗口的用法）
- [10V. 诊断窗口会话结束后的保持](10v-diagnostic-window-hold.md) —— 窗口会保留 5 秒的原因；`XIAO_DIAGNOSTICS_HOLD_MS` 的语义
- [19. 优化、兼容性与发布验收](19-optimization-release.md) `19.14` —— 被验证的条目原文
- [19D. 性能对照](19d-performance-comparison.md) —— O6 条件 5 与 `baseline.json` 的协议（**本轮不做，见 §〇**）

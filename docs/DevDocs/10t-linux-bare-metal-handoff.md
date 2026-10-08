# 10T-Linux：裸机 Linux 复跑与诊断窗口定位交接

> **这是新一轮的独立交接档，写给那台 Ubuntu 机器的操作者。** 由 [10T](10t-table-method-abi-implementation.md) 派生。
> 上一轮见 [10S-Linux 交接](10s-linux-bare-metal-handoff.md) 与[上一轮回传](10s-linux-bare-metal-results-20261007.md)。
>
> **这一轮只做两件事**：确认修复后的提交在裸机上**跑通**；把上一轮「测试报 ok 但窗口没出现」的原因**定位到可复现的事实**。
> **不要改任何源码**——发现缺陷就记录现象回传，修复由实现 Agent 做。

## 一、这一轮和上一轮有什么不同

| | 上一轮（10S，检出 `0ae4850`） | 这一轮（10T） |
| --- | --- | --- |
| 结果 | **失败**：13 个差分用例因「产物观察到未由 IR 登记的 Runtime 组件：`weak`」构建失败 | 该问题已修（弱组件依赖改为按 main 入口可达调用图登记），预期跑通 |
| 窗口 | 测试 `real_terminal_session_is_environment_gated` 报 **ok**，但操作者**没看到窗口**、无截图 → 记为**未通过** C 档 | 本轮的重点之一：把「为什么没窗口」查清楚 |
| 其他 | 另有表方法 ABI 未实现等 | 表方法已接通，五个基准程序在 CI 上均可构建 |

**这一轮仍然不覆盖 macOS**（CI 里显式跳过真实终端），也**不是性能证据**；它给的是 19 O6 条件 3 的裸机 Linux 那一格。

## 二、检出提交

```text
0735846
```

说明：这是本档写作时的 `main`，代码与 10T 的最终功能提交 `58e8ecc` **完全等同**（`0735846` 只改文档）。
该提交的完整平台复现（Windows / Linux amd64 / Linux arm64 / macOS + 汇总）与维护门控均为绿。
**如果实施 Agent 另有指定，以它给的提交号为准**，并在回传里写明实际检出的提交。

## 三、前置环境

裸机或正常安装的 Ubuntu（**不是 WSL，不是容器**）。安装包：

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

**这一轮额外要记录窗口相关的环境**（上一轮没记，导致没法定位）：

```bash
which x-terminal-emulator && readlink -f "$(command -v x-terminal-emulator)"
which gnome-terminal konsole xterm
echo "DISPLAY=$DISPLAY  WAYLAND_DISPLAY=$WAYLAND_DISPLAY  XDG_SESSION_TYPE=$XDG_SESSION_TYPE"
```

诊断窗口会按 **`x-terminal-emulator` → `gnome-terminal` → `konsole` → `xterm`** 的顺序尝试启动终端模拟器
（见 `xiao-driver` 的 `terminal_candidates`）。上一轮「测试 ok 但无窗口」很可能就出在第一个候选上
——例如 `x-terminal-emulator` 指向 GNOME 终端时，它在 Wayland 下需要 D-Bus 会话，可能启动即退出而不显示窗口。
这些路径信息是本轮定位的关键。

## 四、要跑的命令

```bash
git clone https://github.com/Programming-Language-Xiao/Xiao.git
cd Xiao
git checkout 0735846

# 不要设置 XIAO_USE_XVFB：本机有真实图形会话，正是本轮要利用的条件
RUSTUP_TOOLCHAIN=1.96.0 bash tools/platform-reproduction/reproduce.sh native > /tmp/xiao-10t-native.log 2>&1
echo "exit=$?"
```

脚本会自己构建 Runtime 与诊断程序、跑门控测试、打包 CLI 并做协议检查。
**脚本跑的时候请盯着屏幕**——窗口那一步就在其中（见 §五）。

## 五、窗口那一步（本轮重点）

### 5.1 先记录现状

按 §三 的 `which` / `readlink` / `echo` 采集，**先不要改任何 alternatives 设置**。

### 5.2 跑并观察

脚本完成后另取窗口证据；不要用门控测试充当窗口证据。先固定终端：

```bash
sudo update-alternatives --set x-terminal-emulator /usr/bin/xterm
```

用真实调试入口运行程序（窗口结束后默认会保留 5 秒，因此短程序也可取证）：

```bash
xiao run -debug ./window-probe.xiao
```

程序运行及结束后的保持期间观察并截图，记录窗口是否显示最终状态，以及关窗后进程的退出行为。
若窗口仍未出现，同时记录：

```bash
# 另开一个终端，在脚本跑到门控测试阶段时执行
pgrep -a xiao-diagnostics
pgrep -a xterm
```

看到窗口就**截图**；没看到就把上面几条的输出连同**当时的时间点**记下来。

### 5.3 如果窗口没出现，做一次对照

把终端候选固定为 CI 使用的 xterm，再运行上面的真实 `-debug` 程序并记录差别：

```bash
sudo update-alternatives --set x-terminal-emulator /usr/bin/xterm   # 与 CI 的 Linux 作业一致
# 然后重复真实 `xiao run -debug` 观察
```

**两次的结果都要回传**：默认设置下的表现，与换成 `xterm` 之后的表现。这能区分
「候选选择问题」和「更深的启动问题」。

### 5.4 什么算通过

- **窗口肉眼可见**（大致是滚动区域 + 状态栏的 TUI）；
- **一张截图**；
- 关掉窗口后进程能正常结束，不卡住。

**只有测试 ok 不算通过**——上一轮已经明确过这一点，本轮沿用。

## 六、要记录并回传的内容

```bash
cat /etc/os-release | head -3
uname -m
ldd --version | head -1
rustc -vV
bun --version
clang --version | head -1
llvm-as --version | head -1
which clang llvm-as llc llvm-strip xterm
git rev-parse HEAD
```

外加：

1. §三 的窗口环境那几条（`x-terminal-emulator` 指向、各终端模拟器是否安装、`DISPLAY`/`WAYLAND_DISPLAY`/`XDG_SESSION_TYPE`）；
2. **完整命令**与**退出码**；
3. `/tmp/xiao-10t-native.log` 的尾部（约最后 60 行）以及其中任何 `FAILED` / `panicked` / `显式跳过` 行；
4. §五 的窗口观察结果：`pgrep` 输出、是否截图、换 `xterm` 前后的差别；
5. `file` 对打包产物的输出。

## 七、先别做的事

- **不要改源码**（包括"顺手修一下"）；
- **不要用 WSL 或 Docker 代替**，也不要在结果里省略"这是裸机"；
- **不要设置 `XIAO_USE_XVFB=1`**；
- **不要跳过失败的用例**然后报"全绿"；
- **不要把「测试报 ok」当成窗口已验证**；
- **不要把这份结果写成「三平台一致」**——macOS 仍未验证。

## 八、可能遇到的问题

| 现象 | 常见原因 | 处理 |
| --- | --- | --- |
| 脚本在门控测试阶段失败 | 见 §六 第 3 条，把失败原文回传 | 不要跳过，照实回传 |
| 测试 ok 但无窗口 | `x-terminal-emulator` 指向的模拟器在 Wayland 下启动即退出 | 做 §5.3 的对照，两次都记录 |
| `pgrep` 找不到任何诊断进程 | 诊断会话没起来或已退出 | 连同日志尾部一起回传 |
| 找不到 `llvm-as` / `clang` | 未安装或不在 PATH | 按 §三 安装；回传 `which` 输出 |
| `bun install --frozen-lockfile` 失败 | bun 版本不符 | 用 1.4.x；回传完整错误 |
| 纯 `ssh` 会话 | 通常没有 `DISPLAY` | 换到本机图形会话或 VNC；写明会话方式 |

## 九、回传模板

```text
检出提交：
发行版与版本：
架构 / glibc：
rustc / bun / clang / llvm-as：
XDG_SESSION_TYPE / DISPLAY / WAYLAND_DISPLAY：
x-terminal-emulator 指向：
已安装的终端模拟器（gnome-terminal / konsole / xterm）：
完整命令：
退出码：
real_terminal_session_is_environment_gated：ok / failed / 被跳过（附原因）
窗口肉眼可见：是 / 否
截图：已附 / 无法提供
pgrep 输出（xiao-diagnostics / 各终端模拟器）：
换成 xterm 之后的表现：
日志尾部：
打包产物 file 输出：
其他异常：
```

## 相关页面

- [10T. 表方法 ABI 实现](10t-table-method-abi-implementation.md) —— 本档的父批次
- [10S-Linux 交接](10s-linux-bare-metal-handoff.md) —— 上一轮；其回传见[结果](10s-linux-bare-metal-results-20261007.md)与[日志](10s-linux-bare-metal-log-20261007.md)
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) §4 —— 门控环境准备
- [19C. 维护策略与调试三平台](19c-maintenance-and-debug.md) §2.5 —— `19.14` 的 A/B/C 三档证据口径
- `tools/platform-reproduction/README.md` —— 裸机与 Docker/WSL 证据的边界

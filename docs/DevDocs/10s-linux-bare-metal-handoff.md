# 10S-Linux：裸机 Linux 证据采集交接

> **这是一份独立交接档，写给那台 Ubuntu 机器的操作者。** 由 [10S](10s-cross-platform-evidence-and-release-closeout.md) 派生。
> 需要这台机器是因为：19 的出口条件 3 要求三平台一致，而 `tools/platform-reproduction/README.md` 明确写
> 「Docker/WSL 结果是功能证据，**不能冒充裸机验收**」；除 Windows 外，目前只有这台机器能提供裸机 Linux。
>
> **这台机器要做的只有两件事**：跑一次完整的平台复现并记录环境；在真实图形会话下确认诊断窗口能开。
> **不需要改任何源码**——如果发现缺陷，**记录现象并回传**，修复由实现 Agent 在别的批次做。

## 一、这次要拿到什么

| 目标 | 对应条目 | 需要什么证据 |
| --- | --- | --- |
| 裸机 Linux 的平台复现 | 19 的 O6 出口条件 3 | 完整命令、退出码、四个环境的版本、日志尾部、产物格式 |
| 诊断窗口在 Linux 桌面打开 | 19C 的 `19.14` C 档（此前记录为「无宿主可验证」） | `real_terminal_session_is_environment_gated` 在真实 X 会话下**通过**，且**肉眼确认窗口出现**（截图） |

**这份证据不能证明的事**（回传时不要顺手扩大结论）：

- 它**不覆盖 macOS**——macOS 只有 CI，且 CI 对 macos-arm64 会显式跳过真实终端用例；
- 它**不是性能证据**——`reproduce.sh` 只验证功能链路，不采集性能数字；
- 一台机器的一次通过，不构成「三平台一致」，只是把 Linux 那一格从「未验证」变成「有裸机证据」。

## 二、前置环境

**操作系统**：要求是**裸机或正常安装的 Ubuntu**（不是 WSL，不是容器）。WSL 与 Docker 的结果在这条线上不算数。

**安装包**（与 CI 的 Linux 矩阵一致）：

```bash
sudo apt-get update
sudo apt-get install --yes clang lld llvm build-essential xauth xterm file
```

- `xterm` 是诊断窗口的渲染器，**必须装**——缺了它，19.14 C 档那一步会以「找不到渲染器」失败；
- `xvfb` 不需要（本机有真实显示，见 §五 的注意事项）。

**工具链版本**（必须与仓库一致，否则证据不可比）：

```bash
rustup toolchain install 1.96.0          # 仓库 core/rust/rust-toolchain.toml 固定 1.96.0
rustup component add rustfmt clippy --toolchain 1.96.0
bun --version                            # 需要 1.4.x（CI 用 1.4.1）
clang --version | head -1
llvm-as --version | head -1
```

## 三、要跑的命令

**检出提交**：本档发布时的 `main`（写作时 HEAD 为 `48d4a3f`）。**如果实施 Agent 另有指定，以它给的提交号为准**，
并在回传里写明实际检出的提交。

```bash
git clone https://github.com/Programming-Language-Xiao/Xiao.git
cd Xiao
git checkout <上面确定的提交>

# 一次跑完：门控测试 + CLI 打包 + 协议检查；脚本会自己构建 Runtime 与诊断程序
set -o pipefail
bash tools/platform-reproduction/reproduce.sh native 2>&1 | tee /tmp/xiao-native.log
status=${PIPESTATUS[0]}
echo "exit=$status"
```

**关于 `XIAO_USE_XVFB`**：**不要设置它**。脚本只在 `XIAO_USE_XVFB=1` 时才用 `xvfb-run` 包一层，
那会变成虚拟显示；本机有真实图形会话，正是这次要利用的条件。

**如果脚本中途失败**：把失败的完整输出保留下来照实回传，**不要**为了跑完而删测试、改参数或跳步骤。
失败的证据同样有价值——它正是这套流程存在的理由。

## 四、必须记录并回传的内容

请按这个清单采集，缺项会让证据不可复现：

```bash
cat /etc/os-release | head -3          # 发行版与版本
uname -m                               # 架构（预期 x86_64）
ldd --version | head -1                # glibc 版本
rustc -vV                    # rustc 版本与 host 三元组
bun --version
clang --version | head -1
llvm-as --version | head -1
which clang llvm-as llc llvm-strip xterm
```

外加：

1. **实际检出的提交号**（`git rev-parse HEAD`）；
2. **完整命令**（原样复制，含所有环境变量）；
3. **退出码**；
4. **日志**：`/tmp/xiao-native.log` 的**尾部**（最后 60 行左右）与其中出现的任何 `FAILED` / `panicked` /
   `显式跳过` 行；
5. `file` 命令对打包产物的输出（脚本末尾会打印）。

## 五、诊断窗口那一步（19.14 C 档）

`reproduce.sh native` 会跑门控测试，其中 `real_terminal_session_is_environment_gated` 会启动一个诊断会话。
在**有真实显示的会话**里跑，并且 `xterm` 已安装时，它会真的开出一个窗口。

**什么算行为证据**：

- 测试**通过**（日志里该用例是 `ok`，不是 `ignored`，也不是被 `--skip` 跳过）；
- 运行时**肉眼看到窗口真的出现了**（内容大致是滚动区域加状态栏的 TUI）；
- **一张截图**（窗口与终端同框最好）；
- 关了窗口之后，测试/进程能正常结束，没有被卡住。

**注意事项**：

- 必须是**本机图形会话**（坐在机器前，或 VNC/远程桌面等有真实 `DISPLAY` 的会话）。
  **纯 `ssh` 登录通常没有 `DISPLAY`**，那种情况下这个用例会失败——那不是产品缺陷，
  请把会话方式写清楚再回传；
- 若窗口一闪而过或测试报「找不到渲染器」，先确认 `xterm` 已安装、`DISPLAY` 有值，
  把 `echo $DISPLAY`、`which xterm` 和错误原文一起回传。

## 六、先别做的事

- **不要改源码**，包括「顺手修一下看起来不对的地方」——本档只采集证据；
- **不要用 WSL 或 Docker 代替**，也不要在结果里省略「这是裸机」这一事实；
- **不要设置 `XIAO_USE_XVFB=1`**，哪怕是为了「让它跑起来」；
- **不要跳过失败的用例**然后报「全绿」；
- **不要把这份结果写成「三平台一致」**——macOS 仍然是未验证。

## 七、可能遇到的问题

| 现象 | 常见原因 | 处理 |
| --- | --- | --- |
| `clang: command not found` 或找不到 `llvm-as` | `llvm`/`clang` 未装或在非标准路径 | 按 §二 安装；把 `which` 的输出回传 |
| `real_terminal_session_is_environment_gated` 失败 | 无 `DISPLAY`（ssh 会话）、缺 `xterm` | 换到图形会话、装 `xterm`；回传 `echo $DISPLAY` |
| `bun install --frozen-lockfile` 失败 | bun 版本不符或网络问题 | 用 1.4.x；回传完整错误 |
| `cargo` 相关失败且提到 target/triple | 工具链版本不符 | 确认 `rustup toolchain install 1.96.0` 已执行 |
| 脚本很久没输出 | 首次要构建整个 workspace | 耐心等待；若超过 30 分钟把最后输出回传 |

## 八、回传模板

把下面这段填好回传即可（表格不要删列）：

```text
检出提交：
发行版与版本：
架构 / glibc：
rustc / bun / clang / llvm-as：
完整命令：
退出码：
是否设置 XIAO_USE_XVFB：否（应当为否）
real_terminal_session_is_environment_gated：通过 / 失败 / 跳过（附原因）
窗口是否肉眼可见：是 / 否
DISPLAY 值：
截图：已附 / 无法提供
日志尾部：
打包产物 file 输出：
其他异常：
```

## 相关页面

- [10S. 跨平台证据刷新与释放账目收口](10s-cross-platform-evidence-and-release-closeout.md) —— 本档的父批次
- [10J. 跨操作系统验证专项（Linux 侧交接）](10j-cross-os-verification.md) —— 同类型交接档的先例
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) §4 —— 门控环境的准备方式
- [19C. 维护策略与调试三平台](19c-maintenance-and-debug.md) §2.5 —— `19.14` 的 A/B/C 三档证据口径
- `tools/platform-reproduction/README.md` —— 各平台证据的定位与边界（裸机 vs Docker/WSL）

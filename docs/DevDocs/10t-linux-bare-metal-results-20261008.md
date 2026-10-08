# 10T Linux 裸机复跑结果（2026-10-08）

本记录对应 [10T Linux 交接](10t-linux-bare-metal-handoff.md)，实际检出提交为 `74b272d70bde3eaeb9f8619426a5bc0594f6cfcd`。
机器为 Ubuntu 裸机桌面环境，未使用 WSL、Docker 或 Xvfb。

## 复现结果

完整命令：

```bash
RUSTUP_TOOLCHAIN=1.96.0 bash tools/platform-reproduction/reproduce.sh native > /tmp/xiao-10t-native.log 2>&1
```

退出码：`0`。

- `real_terminal_session_is_environment_gated`：`ok`，没有被跳过；
- Rust/TypeScript 门禁、CLI 打包、原生构建、协议回环和 `file` 产物检查全部通过；
- 日志没有 `FAILED` 或 `panicked`；
- `XIAO_USE_XVFB` 未设置。

## 诊断窗口定位

窗口候选环境：

- `x-terminal-emulator` → `/usr/bin/terminator`；
- 已安装 `gnome-terminal` 和 `xterm`；
- `DISPLAY=:0`，`WAYLAND_DISPLAY=wayland-0`，`XDG_SESSION_TYPE=wayland`。

操作者观察到终端窗口确实闪现，但马上关闭。完整复现日志同时出现 Terminator 配置提示：

```text
Unable to open '/etc/xdg/terminator/config' for reading and/or writing.
ConfigBase::load: Unable to open /etc/xdg/terminator/config
```

只针对 `real_terminal_session_is_environment_gated` 的默认候选和临时强制 xterm 对照均返回 `ok`；强制 xterm 也会快速关闭窗口。由于窗口无法持续显示且没有截图，19.14 C 档仍记为未验证通过。该现象表明候选终端启动并完成握手后，测试立即释放诊断会话。

完整原始日志保留在本机 `/tmp/xiao-10t-evidence/native.log`；仓库只提交日志尾部：[10t-linux-bare-metal-log-20261008.md](10t-linux-bare-metal-log-20261008.md)。

## 环境

```text
commit=74b272d70bde3eaeb9f8619426a5bc0594f6cfcd
PRETTY_NAME="Ubuntu 26.04.1 LTS"
NAME="Ubuntu"
VERSION_ID="26.04"
x86_64
virt=none
ldd (Ubuntu GLIBC 2.43-2ubuntu2.4) 2.43
rustc 1.96.0 (ac68faa20 2026-05-25)
binary: rustc
commit-hash: ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96
commit-date: 2026-05-25
host: x86_64-unknown-linux-gnu
release: 1.96.0
LLVM version: 22.1.2
1.4.0
Ubuntu clang version 21.1.8 (6ubuntu1)
Ubuntu LLVM version 21.1.8
/usr/bin/clang
/usr/bin/llvm-as
/usr/bin/llc
/usr/bin/llvm-strip
/usr/bin/xterm
x-terminal-emulator=/usr/bin/x-terminal-emulator
/usr/bin/terminator
/usr/bin/gnome-terminal
/usr/bin/xterm
DISPLAY=:0 WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland
```

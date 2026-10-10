# 10Z Linux 裸机窗口取证结果（2026-10-10）

本记录对应 [10Z Linux 交接](10z-linux-bare-metal-handoff.md)，实际检出提交为
`69f6ff57f7bcafe87817eb61498735fbad89fbdd`。机器是 Ubuntu 裸机桌面环境，未使用
WSL、Docker 或 Xvfb。

原始命令输出见：[10Z Linux 窗口取证日志](10z-linux-bare-metal-log-20261010.md)。

## C 档判定

判定：**通过**。

操作者在真实桌面终端执行 `xiao run -debug` 后，诊断窗口实际出现并保持可见，窗口中显示
模块加载、函数进入、栈帧、作用域和输出事件；程序退出码为 `0`。本次使用
`XIAO_DIAGNOSTICS_HOLD_MS=60000`，并设置 UTF-8 locale；窗口截图已归档：

- [终端与窗口同框](assets/10z-linux-20261010/diagnostics-terminal.png)
- [独立诊断窗口](assets/10z-linux-20261010/xiao-diagnostics.png)

截图中两个终端的字体表现不同：一个窗口的 CJK 字符出现乱码或缺字，另一个窗口能显示完整
事件结构。窗口本身确实出现，窗口保持和退出行为均符合取证要求；字符渲染问题单独记录，
不把它写成终端显示完全正常。窗口结束后，命令行正常返回。

## 实际命令

```bash
cd /tmp/xiao-10z-probe2
export XIAO_CORE_PATH=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/debug/xiao-core
export XIAO_DIAGNOSTICS_PATH=/home/xiaocz/Projsct/Xiao/Xiao/core/rust/target/debug/xiao-diagnostics
export XIAO_DIAGNOSTICS_HOLD_MS=60000
export LANG=zh_CN.UTF-8
export LC_ALL=zh_CN.UTF-8
export TERM=xterm-256color
bun /home/xiaocz/Projsct/Xiao/Xiao/cli/ts/src/main.ts run -debug main.xiao
```

命令输出中的关键结果：

```text
window-probe
状态  退出码
----  ------
成功  0
缓存 hit  xiaoc  摘要 c50aef82928645cad4d9239403243b7d53864220ed992de2ea5383d7b7c99920  校验 verified
优化 O0：已执行 0/5 个 Pass
```

## 环境

```text
commit: 69f6ff57f7bcafe87817eb61498735fbad89fbdd
Ubuntu: 26.04.1 LTS
CPU: 12th Gen Intel(R) Core(TM) i5-12450H, 12 CPUs
memory: 14 GiB
kernel: Linux 7.0.0-31-generic x86_64
virtualization: none
rustc: 1.96.0, host x86_64-unknown-linux-gnu
bun: 1.4.0
clang: Ubuntu clang version 21.1.8
llvm-as: Ubuntu LLVM version 21.1.8
x-terminal-emulator: /usr/bin/terminator
gnome-terminal: /usr/bin/gnome-terminal
xterm: /usr/bin/xterm
DISPLAY=:0
WAYLAND_DISPLAY=wayland-0
XDG_SESSION_TYPE=wayland
```

本轮只补 `19.14` 的 Linux C 档窗口证据，不涉及性能取数或 macOS。

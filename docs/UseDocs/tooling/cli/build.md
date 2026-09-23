---
id: tooling.cli.build
title: xiao build 原生构建
status: verified
audience: developer
module: ts.xiao-cli
stage: 11X0-E
related:
  - README.md
  - shell.md
  - packaging.md
  - debug.md
  - ../../../DevDocs/11x0e-build-and-toolchain.md
---

# `xiao build` 原生构建

`xiao build` 读取真实的 `.xiao` 源码，经 TypeScript CLI、Rust 核心协议和
`FrontendNativeDriver` 生成 LLVM 原生程序。命令不等待 `xiao run`，也不接受手写 TAC；源码、
配置和目标信息由同一条前端管线处理。

## 命令格式

```text
xiao build -o <output> <file.xiao> [-debug] [--emit-llvm <path>] [-O0]
```

`-o`（或 `--output`）指定可执行文件路径。省略时，输出为
`build/<源码基名>`；Windows 自动追加 `.exe`。`--emit-llvm` 可额外保存 LLVM 文本，父目录
不存在时会由构建后端创建。当前只允许 `-O0`；`-O1`、`-O2`、`-O3` 和其他优化值会在执行前
以参数错误拒绝。输入必须是 `.xiao` 文件。

例如：

```text
xiao build -o build/hello.exe examples/hello.xiao
xiao build examples/hello.xiao --emit-llvm build/hello.ll -O0
xiao build -o build/hello-debug.exe examples/hello.xiao -debug
```

## 工具链发现

CLI 按以下顺序寻找每个工具：显式环境变量覆盖、独立 CLI/输出目录相邻目录、`PATH`、检测到
仓库根后的开发目录。显式路径无效时不会静默回退。`clang` 是必需工具；`llvm-as`、`llc`、
`rustc`、Runtime 静态库和诊断组件按构建需要记录。发现结果会保留候选路径、来源、版本、
目标和失败原因。

常用环境变量：

```text
XIAO_CLANG             clang 可执行文件
XIAO_LLVM_AS           llvm-as 可执行文件
XIAO_LLC               llc 可执行文件
XIAO_RUSTC             rustc 可执行文件
XIAO_LLVM_BIN          LLVM bin 目录
XIAO_TOOLCHAIN_ROOT    工具链根目录
XIAO_RUNTIME_LIBRARY   Runtime 静态库路径
XIAO_DIAGNOSTICS_PATH  -debug 构建使用的 xiao-diagnostics 路径
XIAO_CORE_PATH         Rust 核心 xiao-core 路径
```

clang 主版本低于仓库 `core/rust/llvm-toolchain.toml` 的最低版本会返回
`X11-CLI-TOOLCHAIN-VERSION-001`。目标三元组不兼容返回
`X11-CLI-TOOLCHAIN-TARGET-001`；真实 C 编译或链接探测失败返回
`X11-CLI-TOOLCHAIN-LINK-001`；显式路径无效返回
`X11-CLI-TOOLCHAIN-OVERRIDE-001`。这些错误发生在生成最终产物前。

Windows 原生构建需要在 Visual Studio Developer Command Prompt（例如 `vsdevcmd.bat -arch=x64
-host_arch=x64`）中启动，使 SDK、链接器和目标环境可用。也可以通过 `XIAO_CLANG` 指定兼容
的 clang；MSYS2 GNU 目标不能冒充冻结的 MSVC 目标。

## 输出与调试构建

普通构建生成可执行文件；若提供 `--emit-llvm`，同时生成 LLVM 文本。构建响应带产物路径、
工具链指纹和目标信息。

`-debug` 构建会在用户入口前启动独立的 `xiao-diagnostics`，并把诊断组件复制到可执行文件
目录。启动桥只记录组件文件名，运行时从自身可执行文件目录寻找相邻组件，所以将可执行文件
和诊断组件整体搬迁后仍可运行。Windows 产物使用独立控制台，并等待诊断首屏就绪后才执行用户代码。旁置的
`<executable>.xiao-debug.json` 是持久激活位；普通构建不会生成它，`config.xiao` 中的
`[debug]` 表也不能自行打开窗口。无法复制组件、创建窗口或在五秒内就绪时返回
`X11-DIAGNOSTIC-START-001`，用户代码不会执行。

经过验证的 `config.xiao` 会固化为 `<executable>.xiao-runtime.json`。文件声明格式版本和
`cli_overrides = true`；命令行覆盖只影响当前构建/进程，不回写源配置，且不能关闭调试产物的
强制激活位。

## 平台证据

Windows 原生已完成真实 Xiao 源码的 build、脱离 CLI 直接运行、独立调试控制台、就绪握手和
缺失组件失败回环。Linux Docker 只提供功能与构建证据；Linux 原生、WSL 和 macOS 尚未复现。
容器或 WSL 的性能数字不与 Windows 原生并列，也不能据此宣称跨平台验收完成。

## 已知边界

`xiao test` 仍只是登记的命令，项目测试语义归 X0-T；`xiao build` 的优化级别除 `-O0` 外尚未
实现。原生构建不改变 Xiao 的语言、退出码或诊断协议语义。

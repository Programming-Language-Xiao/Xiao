# 平台复现工具

本目录提供 X0 平台债的可重复入口。它只验证功能链路和环境门禁，不采集或修改
`tests/benchmarks/reports/` 中的性能数字；`Dockerfile.dev` 仍然只是开发容器，验收定位不变。

## Linux Docker 多架构

在仓库根目录执行：

```text
docker build --platform linux/amd64 -f tools/platform-reproduction/Dockerfile -t xiao-platform-reproduction:linux-amd64 .
docker run --rm --platform linux/amd64 -e XIAO_USE_XVFB=1 -v "$PWD":/workspace -w /workspace xiao-platform-reproduction:linux-amd64 bash tools/platform-reproduction/reproduce.sh native

docker build --platform linux/arm64 -f tools/platform-reproduction/Dockerfile -t xiao-platform-reproduction:linux-arm64 .
docker run --rm --platform linux/arm64 -e XIAO_USE_XVFB=1 -v "$PWD":/workspace -w /workspace xiao-platform-reproduction:linux-arm64 bash tools/platform-reproduction/reproduce.sh native
```

`linux/arm64` 不能省略：它必须执行 `host()` 架构断言，防止 Rust 目标描述退回
`x86_64-*`。容器输出必须记录镜像 ID、Rust、Bun、clang、`llvm-as`、`llc` 版本，以及完整命令；
容器功能证据不等同于裸机 Linux 验收，也不进入 09R3 性能冻结。

Windows PowerShell 可直接使用：

```text
powershell -ExecutionPolicy Bypass -File tools/platform-reproduction/reproduce.ps1 -Mode docker -Platform linux/amd64
powershell -ExecutionPolicy Bypass -File tools/platform-reproduction/reproduce.ps1 -Mode docker -Platform linux/arm64
```

## WSL 原生

在 Ubuntu 或 Arch WSL 内，从仓库工作树执行：

```text
bash tools/platform-reproduction/reproduce.sh native
```

Ubuntu 与 Arch 必须分别记录发行版版本、glibc、`uname -m`、Rust/Bun/LLVM 版本和完整命令。
WSL 共享宿主调度，不可将性能数字与 Windows 原生并列；它可以作为功能证据，但不自动等同裸机
安装路径验收。Arch 环境需要先准备 `base-devel`、`clang`、`llvm`、`lld`、Rust 1.96.0 和 Bun 1.4.1
或更高的 1.x 版本；Bun 1.4.0 在 Docker 的 `--compile` 路径存在 ELF 临时文件权限回归。

## macOS

在 macOS 主机或 GitHub Actions macOS runner 执行：

```text
bash tools/platform-reproduction/reproduce.sh native
```

必须记录 Mach-O 产物、Rust 核心原生构建、工具链发现，以及 `osascript`/`open -a Terminal` 的
调试路径。没有实际运行 runner 前，不能把 macOS 写成已验证。

## 证据字段

每个平台的记录至少包含：

- 环境：发行版或 runner、内核/架构、镜像 ID（Docker）、Rust/Bun/clang/LLVM 版本；
- 命令：构建、`cargo test -- --ignored`、打包、独立 `xiao run`、`xiao test` 和门禁的完整命令；
- 结果：产物格式、核心发现来源、`exit_code`/`exit_name`、`test_result` 的 `total`/`passed`/`failed`；
- 差异：与 Windows 原生逐项比较；没有差异也必须明确写“无差异”；
- 边界：Docker/WSL 结果不得写成裸机安装路径或性能验收，未运行的 macOS 必须保持待复现。

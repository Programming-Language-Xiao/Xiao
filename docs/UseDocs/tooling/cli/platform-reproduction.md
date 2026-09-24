---
id: tooling.cli.platform-reproduction
title: 平台复现工具
status: verified
audience: contributor
module: tools.platform-reproduction
stage: 11X0-P/P1
related:
  - README.md
  - packaging.md
  - build.md
  - test.md
  - ../../../DevDocs/11x0-platform-reproduction.md
  - ../../../DevDocs/11x0p1-platform-reproduction-closure.md
---

# 平台复现工具

平台复现工具用于在 Linux、WSL、macOS 和 Windows 环境中重复执行 Xiao 的功能门禁。它检查
Rust 核心、Runtime、LLVM 工具链、独立 CLI、核心发现、协议版本协商和项目测试链路；它不采集
或修改性能冻结报告中的数字。

## 运行方式

在仓库根目录执行对应入口：

```text
bash tools/platform-reproduction/reproduce.sh native
powershell -ExecutionPolicy Bypass -File tools/platform-reproduction/reproduce.ps1 -Mode native
```

Linux 和 WSL 需要先安装 Rust 1.96.0、Bun 1.4.1 或更高的 1.x 版本、clang、llvm-as、llc、
`xvfb` 和 `xterm`。Linux 无图形会话时设置 `XIAO_USE_XVFB=1`；macOS CI runner 没有真实桌面
会话时可以设置 `XIAO_SKIP_REAL_TERMINAL_TEST=1`，日志必须保留该跳过记录。

Docker 多架构复现使用 `tools/platform-reproduction/Dockerfile`。`linux/arm64` 必须在原生
arm64 runner 或等效架构环境中运行，不能把 QEMU 下的结果当作裸机性能证据。

## 检查内容

脚本会构建核心和 Runtime，显式运行环境门控测试，执行 Rust/TypeScript 门禁，构建独立 CLI，
检查 ELF、PE/COFF 或 Mach-O 产物，并验证同目录、`PATH`、开发回环三种核心发现来源。最后由
`check-protocol.ts` 发送一个错误协议版本，只比较 `type`、`accepted` 和稳定的
`error.code` 字段，不依赖本地化文本。

## 证据边界

每次记录都应包含平台、架构、工具链版本、完整命令、产物格式、发现来源和退出结果。WSL 使用
宿主共享调度，只能作为功能证据；Docker 同样不等同于裸机安装路径或性能验收。macOS 真实终端
测试被显式跳过时，结果标记为未验证而不是通过。

完成复现后继续阅读[独立打包与核心发现](packaging.md)、[`xiao build`](build.md) 和
[`xiao test`](test.md)。

---
id: tooling.cli.packaging
title: xiao 独立打包与核心发现
status: verified
audience: release engineer
module: ts.xiao-cli
stage: 11X0-C
related:
  - README.md
  - shell.md
  - ../../../DevDocs/11x0c-packaging-and-platforms.md
---

# xiao 独立打包与核心发现

在 `cli/ts` 目录执行：

```text
bun run build
```

构建脚本使用 `bun build --compile`，默认目标是当前宿主，并把产物写到
`cli/ts/dist/<bun-target>/`。可以用 `--target`、`--outdir` 和 `--core` 覆盖目标、输出目录
和 Rust 核心路径，例如：

```text
bun run build -- --target bun-windows-x64
bun run build -- --outdir release/xiao --core C:/xiao/xiao-core.exe
```

每个分发目录固定包含：

```text
<目录>/
  xiao[.exe]
  xiao-core[.exe]
  xiao-diagnostics[.exe]
  xiao-package.json
```

`xiao` 是内嵌 Bun 运行时的独立可执行文件；Bun 只在构建机上需要，用户运行时不需要另装
Bun 或 Node.js。`xiao-core` 与 `xiao-diagnostics` 不嵌入 CLI，而是作为同目录资源分发，
清单记录目标和发现来源。

核心发现顺序固定为：

1. `XIAO_CORE_PATH` 显式覆盖；
2. 与独立 `xiao` 相邻的 `xiao-core[.exe]`；
3. Windows 使用 `where.exe`、Linux/macOS 使用 `which` 搜索 `PATH`；
4. 检测到仓库根后才启用开发回环路径。

显式路径错误不会静默回退。找不到核心时，机器诊断 `X11-CLI-CORE-001` 会保留候选路径及
`source` 字段；`development` 表示开发布局，不能当作安装包证据。发现成功后仍要通过协议
首帧 `hello` 协商 `protocol_version` 和 `core_version`，不会解析 `--help` 或试运行文本。

## 平台证据

Windows 原生已验证独立产物、同目录核心发现、开发回环和真实源码回环。Linux Docker 已
完成构建、Rust/TypeScript 门禁和仓库外真实源码回环；这只是容器功能证据，不等同真实
Linux 主机验收。WSL Ubuntu/Arch 当前缺少固定工具链，Linux 原生、WSL 和 macOS 原生仍
在待复现清单中，不宣称三平台验收完成。

Linux Docker 本次使用 `xiao-dev:latest`（Debian 13、Bun 1.4.0、Rust 1.96.0），项目和
Cargo `target` 放在 Docker ext4 命名卷，避免把 Cargo 的密集 I/O 写入 Windows 驱动器。
09R3 release 基准驱动在 Linux 会明确拒绝并保持 `pending-reproduction`，因此不能用容器
数字替代 Windows 原生性能报告。

Git Bash 运行的是 Windows 产物，只用于 PATH/shell 差异检查，不能作为 Linux 证据。容器和
WSL 的性能数字也不能与 Windows 原生基准并列。

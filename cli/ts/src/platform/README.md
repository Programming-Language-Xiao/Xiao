# `cli/ts/src/platform`

放置 CLI 侧路径、Shell、终端能力、文件关联和进程启动适配。工程期 11、18、19；Runtime 平台语义放在 Rust `xiao-platform`。

X0-B 已提供宿主目标描述和开发树 `xiao-core` 发现；X0-C 增加同目录/`PATH`/开发回环的
生产发现顺序，以及 `bun build --compile` 的独立分发目录。X0-E 的 `toolchain.ts` 用相同来源
模型发现 clang、LLVM、Rust、Runtime 与诊断组件，并在构建前做版本、目标和链接探测。完整
Linux 原生、WSL 与 macOS 原生矩阵仍按环境清单逐项复现。

---
id: tooling.cli.environments
title: xiao venv 与环境激活
status: verified
audience: learner
module: ts.xiao-cli
stage: 11A-E0
related:
  - README.md
  - protocol.md
  - shell.md
  - ../../../DevDocs/11ae0-environment-fingerprint-and-activation.md
---

# `xiao venv` 与环境激活

11A-E0 提供项目环境目录、环境元数据和一次性 Shell 钩子。该阶段不联网、不执行项目代码，
也不实现依赖求解、锁文件同步或缓存。

## 创建环境

```text
xiao venv
xiao venv dev
```

CLI 从当前目录向上寻找小写 `config.xiao`，将其所在目录作为项目根；找不到时使用当前目录。
无参数命令创建逻辑名为 `venv`、目录名为 `.venv` 的环境；带名称命令同时使用该名称作为逻辑名
和目录名。例如 `xiao venv dev` 创建 `<项目根>/dev`。目标目录已经存在时返回稳定诊断
`X11-CLI-VENV-003`，不会覆盖原目录。

环境目录内的 `.xiao-environment.json` 包含：

- `config_fingerprint`：Rust 配置层消费规范化 `ConfigDocument` 后生成的配置指纹；
- `toolchain_fingerprint`：复用 LLVM 工具链的稳定指纹，不含绝对工具路径；
- `target_fingerprint`：目标三元组、指针宽度、字节序和目标文件格式指纹；
- `environment_fingerprint`：环境名称、目录名称和上述三项摘要的版本化汇总指纹；
- `lockfile_summary`：当前固定为 `null`，留给后续锁文件阶段。

CLI 只负责发现工具链、创建空目录和写元数据；指纹由 Rust 核心的 `environment` 协议生成。
如果核心、工具链或配置校验失败，创建的空目录会被清理；不会递归删除用户已经写入的内容。

## Shell 钩子

先在当前 Shell 中显式求值一次：

```text
# Bash
eval "$(xiao shell-init bash)"

# PowerShell 5.1+
xiao shell-init powershell | Invoke-Expression
```

钩子不写入 profile，也不能由普通子进程直接改变父 Shell。创建成功后，支持的 Shell 会在原提示符
前加上绿色 `$环境名$ ` 前缀；重复激活或切换会先移除旧前缀，`xiao deactivate` 会恢复初始化前的
原提示符。`NO_COLOR`、`TERM=dumb`、非 TTY 或 `--color=never` 时不输出 ANSI，但仍保留纯文本前缀。

`cmd.exe` 在 E0 中是明确的降级路径：`xiao shell-init cmd` 只输出说明，不修改注册表、不宣称
自动激活；需要提示符闭环时请使用 Bash 兼容 Shell 或 PowerShell。

## 诊断编号

`X11-CLI-VENV-001` 在 E0 中保留为未分配编号：环境入口的首个稳定失败场景后来由更具体的
`X11-CLI-VENV-002`（非法名称）覆盖，因此不会用同一个编号表达另一种语义。当前使用的
`-002` 至 `-005` 分别覆盖名称、已存在目录、目录/元数据写入失败和核心响应类型错误；后续新增
场景不得回收 `-001`。

## 边界

`xiao run`、`xiao build` 和 `xiao test` 仍会自动定位项目环境，不要求当前提示符已经激活。
完整 Shell 矩阵、profile 安装和生产级取消激活命令属于 E3D；锁文件摘要和 `sync` 属于后续阶段。

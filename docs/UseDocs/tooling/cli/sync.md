---
id: tooling.cli.sync
title: xiao sync 与 install
status: verified
audience: learner
module: ts.xiao-cli
stage: 11A-E2B
related:
  - README.md
  - environments.md
  - package-cache.md
  - package-lockfile.md
  - ../../../DevDocs/11ae2b-sync-and-install.md
---

# 同步与安装本地依赖

在含 `config.xiao` 的项目内运行 `xiao sync`。它向上发现项目根，仅解析**本地路径依赖**，
创建或更新项目根 `xiao.lock.json`，导入全局只读源码缓存，再原子更新目标环境的包映射。
多余映射默认移除；目标按 **当前激活环境 → 项目 `.venv` → 创建 `.venv`** 选择。
在 Bash/PowerShell 中执行过 `shell-init` 钩子时，成功后激活目标；普通子进程不能
直接激活父 Shell。`cmd.exe` 无钩子，只能使用手工激活或改用支持钩子的 Shell。

```text
xiao sync                 创建/更新锁文件，移除多余映射
xiao sync --keep-extra    保留额外映射
xiao sync --locked        锁文件必须已是最新，否则失败且不写入
xiao sync --frozen        只读现有锁文件，缺失或失配则失败
```

`--locked` 与 `--frozen` 不得并用。连续两次同步且源码不变时，第二次复用锁文件，
不会重复写入环境映射。锁文件已损坏或版本过高时默认 `sync` 也拒绝覆盖，请先检查/修复。
在 CI 和可复现构建中使用 `--locked` 或 `--frozen`。

```text
xiao install
xiao i
```

两个命令解析成同一操作：**只消费已有且与当前本地依赖一致的锁文件**，
不改锁、不移除多余映射、不创建项目环境、不激活。有激活环境时使用该绝对路径；
否则按需创建 `XIAO_HOME/envs/global` **全局映射容器**，而不是创建项目环境。
未设置 `XIAO_HOME` 时使用用户目录下的 `~/.xiao/envs/global`；若已激活环境的路径
失效，`install` 会报错而不会重建项目环境，可先 `xiao deactivate` 后使用全局容器。
锁文件不存在、源内容改变或依赖图不匹配时失败，请先运行 `sync`；不要在 `install`
中暗中更新锁文件。未找到项目 `config.xiao` 时本地包图无法解析，命令失败。

错误响应保留 Rust 的稳定诊断编号：`X05-SYNC-001` 表示参数或路径无效，
`X05-SYNC-002` 表示冻结/安装模式缺锁，`X05-SYNC-003` 表示激活环境不存在或
环境物化失败；配置失配和缓存损坏沿用原有 `X05-LOCK-*`、`X05-CACHE-*` 编号。

本批不提供远程源索引、源顺序选择、网络下载、版本求解或 `.xiaoc`：
本地路径依赖不经过源协议，内容直接进入 E1 缓存，再按 E2A 的锁与原子映射接口提交。
环境的只读包视图读取映射元数据，不执行包源码；接口元数据查询与延迟运行时加载
仍由 11B 消费并验证，不能将包视图测试等同于这两条接口已完成。

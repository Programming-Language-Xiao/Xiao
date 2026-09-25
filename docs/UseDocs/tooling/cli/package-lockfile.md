---
id: tooling.cli.package-lockfile
title: 本地包锁文件与原子环境映射
status: verified
audience: developer
module: rust.xiao-package
stage: 11A-E2A
related:
  - README.md
  - environments.md
  - package-cache.md
  - ../../../DevDocs/11ae2a-lockfile-and-mapping.md
---

# 本地包锁文件与原子环境映射

11A-E2A 只提供 Rust `xiao-package` 核心 API，不新增 `xiao` 子命令。后续 E2B 的 `sync`、
`install`/`i` 可消费这里的锁定结果；目前无法通过 CLI 生成锁文件。

## 文件契约

项目根（`config.xiao` 同级）的 `xiao.lock.json` 是应纳入版本控制、可供代码审查的
`lock_version: 1` JSON。它包含 E0 规范化配置指纹、根包身份、完整传递依赖图和每个包的
逻辑名称、版本、来源身份、E1 源码 SHA-256 摘要；直接依赖保存运行时/开发期分类，
版本约束与来源引用只保留文本，不执行版本求解。预编译变体和目标条件字段首版固定为空。

本地源的 `source_id` 是 `path:<绝对规范化路径>`。路径变动使锁文件过期；因此包含本地路径
依赖的锁文件不能跨机器直接复用。内容摘要只标识缓存对象，不取代包名、版本或源身份。
锁文件及原子写入残留的暂存文件不参与项目根源码快照，避免生成后立即自失效。

## 生成与校验

核心入口使用已通过配置校验的 `ConfigDocument`、D1 完整 `PackageGraph` 和隔离的
`CacheStore`；`build_lockfile` 生成确定性 JSON，`write_lockfile` 和
`generate_or_reuse_lockfile` 在已有文件字节一致时返回 `Reused`，不改写文件。
`read_lockfile` 拒绝不合法 JSON 或未来版本；`validate_lockfile` 逐一比对来源身份、
配置指纹、完整依赖边以及每个包的源码摘要。应用可以先调用校验入口复用旧锁文件，
确需重解时才生成新的锁定结果。
即便依赖边不变，非依赖配置项的规范化内容变化也会通过配置指纹报告不一致；
若只是本地源码内容变化，则使用单独的源码摘要诊断。

| 诊断编号 | 情况 |
| --- | --- |
| `X05-LOCK-001` | 高于当前读取器的格式版本 |
| `X05-LOCK-002` | 锁文件结构或输入包图无效 |
| `X05-LOCK-003` | 配置或完整依赖图变化 |
| `X05-LOCK-004` | 本地包源码摘要变化 |
| `X05-LOCK-005` | 包来源身份（本地路径）变化 |
| `X05-LOCK-006` | 文件读取或原子提交失败 |

缓存自身损坏仍使用 `X05-CACHE-*`，不伪装为锁文件过期。

## 环境映射

已有项目或全局环境的 `.xiao-environment.json` 可通过 `update_environment_mappings`
替换整份包映射；读取旧元数据、校验映射排序后再写入同目录暂存文件。Unix 使用 `rename`，
Windows 使用 `MoveFileExW` 原子替换已存在的文件，不先删除旧文件。写入中断最多留下可清理
的暂存文件，旧元数据依然可读取；此保证面向进程中断，不承诺断电持久性。

E0 Shell 钩子尚未把激活环境的绝对路径导出给 CLI 子进程。E2B 实现“激活环境优先”前
须补这条接线；E2A 不实施激活、同步或安装。

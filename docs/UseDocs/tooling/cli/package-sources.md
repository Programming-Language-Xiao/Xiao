---
id: tooling.cli.package-sources
title: 包源声明与离线索引
status: verified
audience: learner
module: rust.xiao-package
stage: "11A-E3A"
version: "0.1.0"
related:
  - README.md
  - ../config/syntax.md
  - package-lockfile.md
---

# 包源声明与离线索引

11A-E3A 提供离线包源数据契约和本地目录适配器；`sync`/`install` 目前仍只处理本地路径依赖，
不会通过此配置访问网络或解析远程版本。不要把已经配置远程源误认为已安装远程包。

## 多源声明

```xiao
[sources]
primary = { kind = "static", location = "https://example.org/index", display = "主源" }
mirror = { kind = "git-index", location = "https://github.com/team/index.git", alias = "team" }
extra = { list = "https://example.org/sources.json", digest = "<源列表 JSON 规范化后的 64 位小写 SHA-256>" }

[dependencies]
utils = { path = "../utils", source = "team" }
```

每个直接源允许 `kind`、`location`、`alias`、`display`、`protocol`；前两者必填，
`alias` 默认取条目名，`protocol` 默认 1。支持 `path`、`registry`、`static`、
`git-index`，远程地址首版使用 `http://`/`https://`，`git-index` 另可使用 `ssh://`。
本地 `path` 源需给出不含 `.`/`..` 点段的绝对目录路径；相对路径缺少项目根上下文，
不能安全地生成跨项目稳定源身份。
`source` 绑定只能填 alias 或规范化后的 `source_id`，不能填显示名。
`source_id` 含来源种类、协议、规范主机和路径；修改 alias/显示名不会改变源身份。

导入项只能有 `list` 和 `digest`。源列表是一份独立 JSON，至少包含
`protocol_version: 1` 与有序 `sources` 数组，每项含 `kind`、`location`，
可带 `alias`、`display`、`protocol_version`。先处理全部直接源（按书写顺序），
再按导入项书写顺序及各列表内部顺序展开；清单摘要不匹配、别名冲突即失败，
不会静默改写配置。本批不支持保留注释的结构化写回；它属于 E3D。

## 本地索引结构

对 `path` 源，本地目录可提供 `snapshot.json`，其中 `protocol_version` 固定为 1，
`source_id` 与配置一致，`snapshot_id` 标识快照，`shards` 把规范包名映射到各分片的
JCS SHA-256；可选 `mirrors`、`expires_at`、`signature` 仅保留元数据，本批不做信任或过期判定。每个 `index/<包名>.json` 分片携带同一个协议版本、源身份和快照标识，
`packages` 提供该包的全部版本元数据（版本、撤回状态、目标/ABI/兼容性条件、直接依赖、
源码对象引用和可为空的预编译对象引用）。正文引用由相对路径、字节长度与摘要组成，
只有需要包正文时才单独读取并验证。索引解析不会执行项目或包代码。

源清单、分片和源列表摘要都针对 UTF-8/JCS 规范字节计算，不针对原始空白或键顺序计算。
源不存在、版本不识别、分片与清单不符、路径逃逸、正文摘要不匹配都会报错；
不可用源不能伪装为空索引。E3B 已支持本地源驱动的快照与联邦缓存，真实远程传输仍留 E3C。

## 选择与诊断

显式 `source` 只考虑绑定的源；否则按最终配置顺序逐源尝试。前面的源不可用且没有
可验证缓存时必须报错，不可跳到后面的源。选到一个有满足条件候选的源后，
不跨源比较版本；源内版本求解归 E3D。`X05-SOURCE-001` 表示未知协议版本，
`002` 为别名冲突，`003` 为源不可用，`004` 为摘要不匹配，`005` 为未知源引用，
`006` 为歧义，`007` 为源描述或索引格式错误；`008` 为缓存或锁的本地基础设施故障，
`009` 为缓存对象损坏，`010` 为快照归属不匹配。三种缓存错误不会伪装为源不可用。

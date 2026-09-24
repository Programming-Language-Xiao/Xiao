---
id: tooling.cli.package-cache
title: 本地包缓存与环境映射
status: verified
audience: developer
module: rust.xiao-package
stage: 11A-E1
related:
  - README.md
  - environments.md
  - ../../../DevDocs/11ae1-local-dependencies-and-cache.md
---

# 本地包缓存与环境映射

11A-E1 只支持已经由 `xiao-package` D1 依赖图解析出的本地路径包。核心把包目录读取为不可变
快照，使用规范化目录树的 SHA-256 摘要作为内容对象身份；包名、版本和来源仍保留在环境映射中，
不会被摘要替代。

## 缓存位置

缓存根由 `XIAO_HOME` 注入：

- 未设置或为空时使用用户主目录下的 `.xiao`；
- 相对路径直接返回稳定诊断，不根据当前工作目录猜测；
- 路径不可写时直接失败，不静默改用其他目录。

源码对象布局为：

```text
<XIAO_HOME>/
  cache/objects/source/sha256/<前两位>/<digest>/
  envs/<全局环境名>/.xiao-environment.json
```

源码对象与未来的 `.xiaoc`、`.xar` 对象使用不同命名空间。对象提交后设为只读；读取或环境映射
解析前重新计算摘要，校验失败的条目移动到同分片下的 `.corrupt` 隔离路径，不参与后续构建。

## 环境映射

项目环境目录只保存 `.xiao-environment.json`，不保存缓存源码副本，也不创建符号链接、硬链接或
junction。v2 元数据的 `package_mappings` 按完整 `PackageIdentity` 排序，每项包含 `digest` 和
`object_kind`；不同项目可以引用同一对象，同时保留互不覆盖的映射 JSON。全局环境与项目环境也
引用同一对象目录。

读取器会把 v1 元数据视为没有依赖映射的空环境；遇到高于当前读取器的版本会拒绝猜测字段。
同一份本地源目录之后发生变化时生成新的摘要，旧对象不会被原地改写。

## 边界

E1 没有 `xiao` 命令入口，不实现锁文件、依赖顺序求解、`sync`、`install`/`i`、远程源或缓存清理。
这些用户命令与后续锁定流程由 E2 接入。

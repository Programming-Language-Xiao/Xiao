---
id: tooling.cli.package-cache
title: 包缓存与环境映射
status: verified
audience: developer
module: rust.xiao-package
stage: 11A-E1/E3B
related:
  - README.md
  - environments.md
  - ../../../DevDocs/11ae1-local-dependencies-and-cache.md
  - ../../../DevDocs/11ae3b-fast-resolution-and-cache.md
---

# 包缓存与环境映射

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

## 多源缓存与离线读取（E3B）

E3B 在 `xiao-package` 核心中用本地目录源驱动多源缓存；它不会访问网络，
目前 `sync`/`install` 仍只安装本地路径依赖。四类数据互不混用：

```text
<XIAO_HOME>/cache/
  objects/source/sha256/<前两位>/<digest>/    源码正文对象（E1）
  objects/metadata/sha256/<前两位>/<digest>/  跨源去重的包元数据
  snapshots/<source_id 的 SHA-256>/
    <snapshot_id>.json                        历次不可变源快照
    current.json                              原子更新的当前指向
  federation/<配置指纹的 SHA-256>/<有序源序列摘要的 SHA-256>/index.json
                                              各配置独立原子更新的联邦视图
```

锁文件钉住的快照身份、配置指纹、优先级相关源快照、联邦元数据及所有锁定正文对象均通过校验后，
核心可以不读取源目录，直接走离线快速路径。配置或锁文件失配时重新解析；
快照不按时间自动过期。不同项目的联邦索引不互相覆盖；旧单文件索引只读兼容，
更新失败保留该配置上一份已验证的联邦视图。
状态 `fresh` 表示本次读到源，`cached` 表示沿用缓存（报告快照摘要和读取时间），
`unavailable` 表示源不可达且无可用快照；缺失可能影响选包优先级的快照必须失败，
不能跳到更靠后的源。缓存或锁的本地基础设施故障直接中止，不伪装为源不可用。

## 边界

E1 本身没有 `xiao` 命令入口，锁文件及本地 `sync`、`install`/`i` 由 E2 接入。
E3B 只验证本地源读取中断后的原子提交和重试，不验证真实 HTTP Range 断点续传；
远程传输、远程包的命令接线及缓存清理仍未实现。

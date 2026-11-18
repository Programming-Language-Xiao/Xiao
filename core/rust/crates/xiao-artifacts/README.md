# `xiao-artifacts`

## 目录职责

实现 SHA-256 内容寻址对象、不可变缓存、对象完整性校验、原子写入和 Protobuf 索引。缓存对象与依赖锁、`.xar` 清单保持职责分离。

## 工程期

16；17B 追加 `resource` 与 `debug` 内容寻址命名空间，14/17/18 提供字节码、归档和 CLI 接入。

## 模块放置

对象存储、SHA-256、原子事务、缓存淘汰和 Protobuf 索引放在 `src/`；`ObjectKind::Resource`
与 `ObjectKind::Debug` 追加在既有 wire 编号之后，归档索引组合由 `xiao-xar` 完成。

20E 的 `ArchiveIndex` 在字段 13–16 追加标准库接口版本、语言版本、官方模块要求和
运行能力位。官方模块列表编码时按字节序排序去重；零能力也显式编码。归档专用
`ARCHIVE_REQUIREMENTS_FEATURE` 必须存在，旧归档拒绝；`GlobalIndex` 的 feature 掩码仍为 0。
缺少新版本/能力字段是版本不兼容，未知能力位是无效要求，由归档启动层映射为稳定错误码。

## 约束

摘要针对最终规范对象的完整字节；读取时必须重算校验；不能原地改写公开 `.xiaoc`。

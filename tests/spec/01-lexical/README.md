# 01 词法规格快照

## 目录职责

保存 F0/L0 的机器可读源码、Token、源码区间和诊断期望值。快照由
`core/rust/crates/xiao-syntax/tests/lexical_snapshots.rs` 自动读取，不能只
作为手工示例修改。

## 工程期

01 的 F0 与 L0。L1/L2 的字符串、关键字、反引号和缩进快照将在对应增量
完成后追加，不在本批次提前声明。

## 格式约定

每个 JSON 文件包含 `source`、`tokens` 和 `diagnostics`；Token 区间使用原始
UTF-8 字节偏移，诊断编号必须是稳定机器字段。快照不包含本地化展示文本。

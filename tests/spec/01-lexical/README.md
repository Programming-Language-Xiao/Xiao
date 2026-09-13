# 01 词法规格快照

## 目录职责

保存 F0/L0 的机器可读源码、Token、源码区间和诊断期望值。快照由
`core/rust/crates/xiao-syntax/tests/lexical_snapshots.rs` 自动读取，不能只
作为手工示例修改。

## 工程期

01 的 F0、L0 与 L1。L1 快照覆盖字面量、保留字、运算符、分隔符和错误恢复；
L2 的反引号、注释和缩进快照将在对应增量完成后追加。

## 格式约定

每个 JSON 文件包含 `source`、`tokens` 和 `diagnostics`；Token 区间使用原始
UTF-8 字节偏移，诊断编号必须是稳定机器字段。快照不包含本地化展示文本。

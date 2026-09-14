# `xiao-syntax/tests`

## 目录职责

存放 `xiao-syntax` 的集成规格测试。词法测试从仓库 `tests/spec/01-lexical` 读取
固定快照，P0 解析器测试从 `tests/spec/02-parser` 读取 AST/诊断快照，P1 表达式与
选择器测试位于 `p1_expression.rs` 并对应 `tests/spec/03-expression`；三者共同验证
公开前端接口的 Token/节点顺序、原始区间和稳定诊断编号，包括注释、缩进、分隔符、
表达式优先级、选择器路径和错误恢复。

## 工程期

01 的 L0、L1、L2、严格最小 P0 和 P1 表达式/选择器首批（已完成）。P2 声明和 C0 容器测试
分别位于 `p2_declarations.rs`、`p2_snapshots.rs`、`c0_containers.rs` 和
`c0_snapshots.rs`；C2-A 集合花括号消歧、`set()` 调用形状、集合 AST 节点索引和错误恢复
位于 `c2a_sets.rs`，C2-B `set<T | U>` 类型注解、多行布局和错误恢复位于 `c2b_sets.rs`；C2-C `&`、`^`、
优先级和四种集合复合赋值位于 `c2c_set_operations.rs`，对应 `tests/spec/05-containers/c2c-*` 快照。
后续代码块和完整语义会继续沿用此处的集成测试边界。

## 依赖边界

只调用 `xiao-source`、`xiao-diagnostics` 和 `xiao-syntax` 的公开接口，不启动
Runtime、VM 或 CLI，也不把快照测试变成第二套词法器。

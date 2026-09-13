# `xiao-syntax/tests`

## 目录职责

存放 `xiao-syntax` 的集成规格测试。词法测试从仓库 `tests/spec/01-lexical` 读取
固定快照，解析器测试从 `tests/spec/02-parser` 读取 AST/诊断快照；二者共同验证
公开前端接口的 Token/节点顺序、原始区间和稳定诊断编号，包括注释、缩进、分隔符
和 P0 错误恢复。

## 工程期

01 的 L0、L1、L2 和严格最小 P0；后续完整词法和语法入口会继续沿用此处的集成测试边界。

## 依赖边界

只调用 `xiao-source`、`xiao-diagnostics` 和 `xiao-syntax` 的公开接口，不启动
Runtime、VM 或 CLI，也不把快照测试变成第二套词法器。

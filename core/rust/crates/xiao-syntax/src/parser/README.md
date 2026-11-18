# `xiao-syntax/src/parser`

工程期：01-D、01-E、01-F、04-A、05-A。

存放解析器按语法主题拆分的实现扩展。`statements.rs` 负责 P0/P1 顶层语句、表头、
函数、缩进块和控制流；`imports.rs` 对应 05-A，负责 `import`/`from` 语句的 Token
消费和 AST 构造。两个扩展都不访问文件系统、不解析 `config.xiao`，也不建立模块依赖图。

## 模块边界

```text
parser.rs       ← Parser 状态、公开入口、顶层循环、Token/恢复基础和表达式/声明辅助
statements.rs   ← 语句分派、表、函数、缩进块、if/try/循环/return 控制流
imports.rs      ← import/from 导入语句的 AST 构造
```

`parser.rs` 是稳定门面和共享状态提供者；`statements.rs`、`imports.rs` 只能通过
`Parser` 的内部实现接口、Token 和公开 AST 协作。`statements.rs` 可以调用导入扩展的
`parse_import_statement`，但 `imports.rs` 不得反向依赖语句模块。两个扩展均禁止依赖
`xiao-types`、`xiao-modules`、文件系统或 Runtime；类型检查和模块发现留给后续层。
禁止把语句实现重新塞回 `parser.rs`，也禁止在两个扩展之间复制恢复逻辑。

源码级回归测试 `parser_statement_split_keeps_dependency_boundary` 会锁住门面装配、
默认模块路径和上述禁止依赖。后续新增语法主题应沿同一方向新增同级扩展，并同步更新
本 README、交接记录和架构测试。

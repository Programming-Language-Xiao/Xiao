# `xiao-syntax/src`

按职责放置 Token、词法器、P0/P1/P2/C0 AST、Pratt 解析器、诊断编号和选择器类型。对应工程期
01–04；P1 首批、P2 标量声明语法和 C0 容器字面量/声明路径 AST 已完成，P2-A 已将实现拆为 `token.rs`、`lexer.rs`、`ast.rs`、
`parser.rs`、`diagnostics.rs` 与 `selectors.rs`，`lib.rs` 只负责装配和公开重导出。
类型检查放在独立的 `xiao-types` crate。

模块之间不得循环依赖或访问彼此私有状态；解析器只消费 Token，类型层只消费公开 AST
和源码区间。P0 的兼容子集只允许字面量、普通/反引号名称、独立表达式和简单赋值；P1
已加入表达式核心、调用、转换、索引路径、选择器、容器字面量和标量/const 声明 AST，但不执行容器或类型语义。所有公共
节点和解析入口都要保留 `SourceSpan`，并同步更新对应 DevDocs 与 UseDocs。

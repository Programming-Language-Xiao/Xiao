# `xiao-syntax/src`

按职责放置 Token、词法器、P0/P1/P2/C0/C2 AST、Pratt 解析器、诊断编号和选择器类型。对应工程期
01–04、07-B；P1 首批、P2 标量声明语法、C0 容器字面量/声明路径 AST、C2-B 集合类型注解 AST、C2-C 集合运算 AST 和 07-B 错误控制流 AST 已完成，P2-A 已将实现拆为 `token.rs`、`lexer.rs`、`ast.rs`、
`parser.rs`、`parser/statements.rs`、`parser/imports.rs`、`diagnostics.rs` 与 `selectors.rs`，
`lib.rs` 只负责装配和公开重导出。C2-C 的 `&`、`^` 及复合赋值
只在语法层保留结构，集合含义由 `xiao-types` 的独立运算模块决定。
类型检查放在独立的 `xiao-types` crate。

模块之间不得循环依赖或访问彼此私有状态；解析器只消费 Token，类型层只消费公开 AST
和源码区间。P0 的兼容子集只允许字面量、普通/反引号名称、独立表达式和简单赋值；P1
已加入表达式核心、调用、转换、索引路径、选择器、容器字面量和标量/const 声明 AST，但不执行容器或类型语义。所有公共
节点和解析入口都要保留 `SourceSpan`，并同步更新对应 DevDocs 与 UseDocs。

## 04 阶段职责

函数与控制流语法由 `parser/statements.rs` 消费 Token，`parser.rs` 提供稳定门面和共享
恢复接口；AST 数据结构集中在 `ast.rs`：
`Function`、参数种类、调用参数、`If`、`For`、`While`、`Return`、`Break` 和 `Continue`、`Try`、`CatchClause`、`Raise`
均保存递归体、文档注释和原始源码区间。`diagnostics.rs` 另登记 `X07-PARSE-001` 至 `X07-PARSE-003`，不执行类型判断。

解析器负责函数参数分隔、默认值、`/`/`*`/`**` 结构和缩进 `Dedent` 恢复；参数数量、类型统一、
条件类型和循环位置由独立的 `xiao-types` 模块负责。入口只记录 `EntryMode`，不生成启动函数。
本阶段不依赖 Runtime、字节码、LLVM 或 CLI；新增语法必须同步 04 阶段 UseDocs 和模块登记。

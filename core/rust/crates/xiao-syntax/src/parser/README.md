# `xiao-syntax/src/parser`

工程期：01-D、01-E、01-F、04-A、05-A。

存放解析器按语法主题拆分的实现扩展。当前 `imports.rs` 对应 05-A，负责
`import`/`from` 语句的 Token 消费和 AST 构造；它不访问文件系统、不解析
`config.xiao`，也不建立模块依赖图。主解析器 `parser.rs` 只负责装配和通用
恢复接口，后续语法主题应继续保持单一职责。

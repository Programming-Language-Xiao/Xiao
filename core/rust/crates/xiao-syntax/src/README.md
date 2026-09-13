# `xiao-syntax/src`

放置 Token、缩进器、P0 AST 和解析器实现。对应工程期 01–04；当前实现暂集中于 `lib.rs`，类型检查放在 `xiao-types`。

P0 只允许字面量、普通/反引号名称、独立表达式和简单赋值；复杂表达式、容器、
控制流与类型语义必须在后续子阶段加入，不得在此目录提前实现。所有公共节点和
解析入口都要保留 `SourceSpan`，并同步更新 `docs/DevDocs/01d-p0-parser-implementation.md`。

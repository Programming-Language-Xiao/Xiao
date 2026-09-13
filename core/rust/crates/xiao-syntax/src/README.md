# `xiao-syntax/src`

放置 Token、缩进器、P0/P1 AST、Pratt 解析器和选择器类型。对应工程期 01–04；P1 首批已完成，词法、AST 与解析器当前仍集中于 `lib.rs`，选择器数据结构位于 `selectors.rs`，类型检查放在 `xiao-types`。

P0 的兼容子集只允许字面量、普通/反引号名称、独立表达式和简单赋值；P1 已加入
表达式核心、调用、转换、索引路径和选择器，但不执行容器或类型语义。所有公共节点和
解析入口都要保留 `SourceSpan`，并同步更新 `docs/DevDocs/01d-p0-parser-implementation.md`
与 `docs/DevDocs/01e-p1-expression-selectors.md`。

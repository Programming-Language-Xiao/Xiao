# 01C. L2 反引号、注释与缩进实现交接记录

> 本文是 01 阶段 L2 的实现交接文档。它在 [01B L1 交接记录](01b-l1-implementation.md) 的 Token 基础上增加名称边界、注释和行结构，供 P0 解析器代理直接复用；本批次不生成 AST、不执行代码，也不实现类型语义。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：全局语言边界和稳定错误身份。
2. [01. 词法 Token 与语法入口](01-lexical-and-grammar.md)：L2 规则和解析器准入条件。
3. [01A. F0/L0 交接记录](01a-f0-l0-implementation.md)：源码位置和最小 Token。
4. [01B. L1 交接记录](01b-l1-implementation.md)：字面量、保留字和基础运算符。
5. [12. 测试与开发里程碑](12-tests-and-milestones.md)：L2 退出条件。

### 当前状态

| 子任务 | 状态 | 交付位置 |
| --- | --- | --- |
| L2.1 反引号 UTF-8 名称 | 已完成 | `core/rust/crates/xiao-syntax/src/lib.rs` |
| L2.2 普通/文档注释 | 已完成 | `core/rust/crates/xiao-syntax/src/lib.rs` |
| L2.3 行首缩进状态机 | 已完成 | `core/rust/crates/xiao-syntax/src/lib.rs` |
| L2.4 分隔符深度和 EOF 诊断 | 已完成 | `core/rust/crates/xiao-syntax/src/lib.rs` |
| L2.5 规格快照与 UseDocs | 已完成 | `tests/spec/01-lexical`、`docs/UseDocs/language/lexical` |

## 冻结的 Token 契约

### 新增种类

`TokenKind` 新增：

- `BacktickIdentifier`：完整的反引号名称，源码区间包含首尾反引号。
- `DocComment`：完整的 `### ... ###` 文档注释，源码区间包含所有原始内容。
- `Indent`：代码行进入新的缩进层，区间覆盖该行原始行首空白。
- `Dedent`：代码行离开缩进层，默认是位于首个代码字符前的零宽区间；EOF 反缩进位于 EOF 零宽区间。

既有 Token 身份、文本读取方式和 `SourceSpan` 字节偏移规则不变。

### 反引号名称

- 内容允许任意合法 UTF-8 字符和空格，但不能包含物理换行。
- `\\`` 表示名称中的反引号，`\\\\` 表示名称中的反斜杠。
- 其他反斜杠转义使整个 Token 变为 `Invalid`，并产生 `X01-LEX-008`。
- 未闭合名称使整个已扫描片段变为 `Invalid`，并产生 `X01-LEX-005`；遇到换行时不消费换行，以便下一次扫描仍产生 `Newline`。
- 反引号包裹的 `def`、`str` 等关键字只产生 `BacktickIdentifier`，不产生 `Keyword`。

### 注释

- 普通 `#` 注释一直扫描到当前物理换行起点，不产生 Token。
- 文档注释从 `###` 开始，到下一个 `###` 结束；同一行和跨行写法都有效。
- 文档注释整体产生一个 `DocComment`；其中的换行属于该 Token 的源码内容，不额外产生内部 `Newline`。
- 文档注释未闭合时产生 `Invalid` 和 `X01-LEX-006`，扫描到 EOF 为止。
- 注释行不参与缩进栈计算；普通注释后的物理换行仍单独产生 `Newline`。

### 行和缩进

词法器维护 `at_line_start`、逻辑缩进栈和待发 Token 队列：

1. 初始缩进栈为 `[0]`。
2. 行首空格计 1，Tab 展开计 4；原始字节仍保留在 Token 区间中。
3. 空行、普通注释行和文档注释行不改变缩进栈。
4. 真实代码行的逻辑宽度大于栈顶时压入新层并先发 `Indent`；等于栈顶时不发缩进 Token。
5. 宽度小于栈顶时弹栈并发出对应数量的 `Dedent`，再扫描该行代码。
6. 宽度不是 4 的倍数，或小于当前层但不能匹配已有层级时，产生 `X01-LEX-007`；目标宽度向下归入最近的已知层级，不创建隐含层。
7. 括号、方括号和花括号深度大于零时，行首空白仍被消费，但不改变缩进栈；物理换行仍产生 `Newline`。
8. 到达 EOF 时先发出所有剩余 `Dedent`，再发 `Eof`；没有真实尾部换行时不补 `Newline`。

### 分隔符

词法器记录圆括号、方括号和花括号的打开顺序，以控制括号内缩进并提供基础结构诊断。关闭分隔符不匹配时产生 `X01-LEX-009`；EOF 仍有打开分隔符时为每个未闭合起点产生 `X01-LEX-010`。这些诊断只描述词法结构，不替代 P0 的语法错误恢复。

## 二级实现任务与交接边界

### L2.1 状态机实现

`Lexer::next_token` 首先排空待发队列，再处理行首状态，最后执行 L1 的单 Token 扫描。L1 的字符串、数字和运算符扫描函数不得在后续阶段复制一份。

### L2.2 规格测试

`tests/spec/01-lexical/l2-*.json` 覆盖合法名称、单/多行文档注释、空行、Tab、嵌套缩进、括号内换行、错误恢复和 EOF；集成入口为 `core/rust/crates/xiao-syntax/tests/lexical_snapshots.rs`。

### L2.3 P0 准入

P0 可以依赖 `Indent`、`Dedent`、`DocComment` 和 `BacktickIdentifier` 的稳定顺序与区间，但必须自行决定哪些 Token 对 AST 可见。P0 不得把普通注释重新构造成语法节点，也不得把 `/` 在词法层改成路径专用 Token。

## 验收与验证

- L0/L1 所有快照继续通过。
- L2 正反快照覆盖每个新增 Token 和稳定错误码。
- LF/CRLF、Unicode 字节偏移和非行首空白行为一致。
- `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、格式检查、`bun run check` 和文档覆盖率门禁通过。

UseDocs 已同步到[反引号名称](../UseDocs/language/lexical/backtick-identifiers.md)和[注释与缩进](../UseDocs/language/lexical/comments-and-indentation.md)；自然人使用说明不应反向依赖本交接文档的内部字段名。

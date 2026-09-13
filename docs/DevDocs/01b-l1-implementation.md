# 01B. L1 基础词法扩展交接记录

> 本文记录 01 阶段 L1 首批已经落地的 Token 契约。它接在 [01A F0/L0 交接记录](01a-f0-l0-implementation.md) 之后，供后续 L2 和 P0 代理直接接手；L1 仍然只做词法，不生成 AST，也不执行 Xiao 程序。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：跨阶段语义与错误编号边界。
2. [01. 词法 Token 与语法入口](01-lexical-and-grammar.md)：词法草案和选择器符号。
3. [01A. F0/L0 实现交接记录](01a-f0-l0-implementation.md)：源码位置、`SourceSpan` 和旧 Token 契约。
4. [12. 测试与开发里程碑](12-tests-and-milestones.md)：L1 退出条件和规格测试位置。

### 本批次负责与不负责

- 负责：单/双引号字符串、十进制浮点、布尔与 `none` 字面量、保留字、基础运算符、括号和容器/路径标点。
- 不负责：反引号 UTF-8 名称、注释、`INDENT`/`DEDENT`、AST、类型转换、集合运算、运行时执行和 CLI/REPL。
- 词法器只保留原始文本和字节区间；数值解析、字符串求值和运算符优先级由后续阶段完成。

## 已冻结的 L1 接口

### Token 身份

`TokenKind` 新增 `Float`、`String`、`Boolean`、`None`、`Keyword(KeywordKind)`，以及比较、算术、复合赋值、括号、花括号、方括号和选择器标记。现有 `Identifier`、`Integer`、`Equal`、`Newline`、`Eof`、`Invalid` 的名称和行为保持不变。

`KeywordKind::as_str` 返回保留字的正式小写拼写；因此 `TokenKind::as_str()` 对 `def` 返回 `"def"`，而不是把不同保留字合并成一个不可区分的字符串。程序应优先比较枚举值，快照才使用该稳定文本。

### 数字

- 连续十进制数字产生 `Integer`，允许前导零。
- 有小数点的形式（如 `12.50`、`1.`、`.5`）产生 `Float`。
- `e/E` 后可以带正负号，但必须至少有一个十进制数字；缺失时产生 `Invalid` 和 `X01-LEX-004`。
- 词法层不检查位宽、精度或溢出；这些规则属于第 02 阶段。

### 字符串

单引号和双引号都表示 `String`，原始引号保留在 Token 文本中。L1 接受 `\\`、引号、`\\n`、`\\r`、`\\t`、`\\0`、`\\b`、`\\f`、`\\v`、`\\a` 这些基本转义；未闭合字符串产生 `X01-LEX-002`，不支持的转义产生 `X01-LEX-003` 并返回 `Invalid`。普通换行不会被字符串吞掉，恢复时交给后续换行扫描。

### 保留字与标点

`true`、`false` 和 `none` 是独立字面量 Token。控制流、导入、转换和类型名称使用 `KeywordKind`；关键字匹配区分大小写，`True` 仍按普通名称处理。`(`、`)`、`[`、`]`、`{`、`}`、`,`、`:`、`.`、`/`、`~`、`?`、`!`、`!?`、`@` 和 `$` 都保留为独立 Token。`!?` 必须优先于单独的 `!` 和 `?` 匹配；`/` 在词法层不区分除法和路径分隔符。

基础运算符包括 `+`、`-`、`*`、`/`、`//`、`%`、`**`、比较符以及对应复合赋值。集合并集仍只在类型/语义阶段解释为 `+`，词法器不会特殊处理 `|`。

## 二级实现与验证记录

### L1.1 字面量

实现位于 `core/rust/crates/xiao-syntax/src/lib.rs` 的 `Lexer`；正例和错误恢复测试位于同文件测试模块与 `tests/spec/01-lexical/l1-*.json`。

### L1.2 标点与最长匹配

`//=`、`**=`、`!?`、`!=`、`<=` 和 `>=` 均在单次扫描中生成一个 Token。`a[3/2]` 仍生成整数、`Slash`、整数，解析器才把它组合成路径。

### L1.3 退出条件

- L1 字面量、保留字、运算符和分隔符快照通过。
- 非法字符串、非法转义和不完整指数有稳定错误码及源码区间。
- 旧 L0 快照不变；UTF-8 字节偏移和 CRLF 行为不回归。
- `cargo test`、`cargo clippy -D warnings` 和文档覆盖率门禁通过。

## 后续交接顺序

1. L2 已由 [01C](01c-l2-implementation.md) 接手反引号标识符、注释、空行和缩进 Token；不得在 L2 重写 L1 的字面量扫描。
2. P0 已由 [01D](01d-p0-parser-implementation.md) 接手并完成 Token 到字面量/名称/简单赋值 AST 的映射，继续复用每个 Token 的 `SourceSpan`。
3. 第 02 阶段再实现数值位宽、字符串值、`bool(value)` 与 `as` 转换；不要在 Lexer 中加入类型语义。

## 验证命令

```text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-syntax
cargo clippy --manifest-path core/rust/Cargo.toml -p xiao-syntax --all-targets -- -D warnings
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
bun run check
```

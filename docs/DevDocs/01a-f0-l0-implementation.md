# 01A. F0/L0 实现交接记录

> 本文是 01 阶段首批实现的可执行交接记录。它描述当前已经落地的源码位置模型、最小词法器、快照格式和明确未实现边界；后续代理应在此基础上增量推进，不要重写一套位置或错误模型。

## Agent 交接上下文

### 入口与前置约束

- 先读 [00. 决策基线](00-decisions.md)、[01. 词法 Token 与语法入口](01-lexical-and-grammar.md)、
  [00A. 工程框架与目录布局](00a-project-layout.md) 和 [12. 测试与开发里程碑](12-tests-and-milestones.md)。
- F0/L0 只覆盖源码读取、位置计算和六种最小 Token；不生成 AST、不执行代码、不做类型检查。
- Rust 是本批次实现语言；CLI/REPL 尚未接入，不能在 TypeScript 中复制词法语义。
- 每次修改必须同时更新同级 README、规格快照、UseDocs 和模块登记，且通过文档覆盖率门禁。

### 当前状态

| 子任务 | 状态 | 交付位置 |
| --- | --- | --- |
| F0.1 UTF-8 源码对象 | 已完成 | `core/rust/crates/xiao-source/src/lib.rs` |
| F0.2 位置、区间与游标 | 已完成 | `core/rust/crates/xiao-source/src/lib.rs` |
| F0.3 源码单元测试 | 已完成 | 同文件 `#[cfg(test)]` |
| L0.1 Token 与诊断模型 | 已完成 | `core/rust/crates/xiao-syntax/src/lib.rs`、`xiao-diagnostics` |
| L0.2 最小扫描器 | 已完成 | `core/rust/crates/xiao-syntax/src/lib.rs` |
| L0.3 规格快照 | 已完成 | `tests/spec/01-lexical`、`xiao-syntax/tests` |

## 冻结的接口契约

### `xiao-source`

- `SourceFile::from_bytes` 校验 UTF-8；失败返回 `SourceError::InvalidUtf8`。
- `SourceFile::from_text` 用于已知合法的 UTF-8 文本；同时实现标准 `FromStr` trait。
- `SourcePosition` 使用原始字节 `offset`、一基 `line` 和一基 Unicode 标量 `column`。
- `SourceSpan` 是半开区间 `[start, end)`；Token 和诊断只保存区间，不复制源码文本。
- `SourceFile::position_at` 要求偏移位于 UTF-8 字符边界；`span` 同时验证两端边界。
- LF 和 CRLF 都建立一个逻辑行；CRLF 的行边界跨越两个原始字节。
- `SourceCursor` 只提供查看/消费 Unicode 标量的安全游标，不能越过 EOF。

### `xiao-diagnostics`

- `Diagnostic` 保存 `code`、`message_id`、`Severity`、可选 `SourceSpan` 和展示文本。
- `code`/`message_id` 是机器接口；展示文本不得被程序或测试用作错误判断。
- L0 非法字符编号为 `X01-LEX-001`，非法 UTF-8 编号为 `X01-SOURCE-001`。
- 原因链、堆栈、错误对象和本地化回退留到第 07/11C 阶段扩展，不得在本批次临时实现。

### `xiao-syntax`

L0 的 `TokenKind` 固定为：`Identifier`、`Integer`、`Equal`、`Newline`、`Eof`、`Invalid`；
`TokenKind::as_str` 提供快照使用的稳定名称。
`Token` 保存种类和 `SourceSpan`，通过 `Token::text(&SourceFile)` 读取原始文本。
`Token::start_position` 与 `Token::end_position` 可在需要展示诊断时将区间转换为行列位置，
避免在 Token 内复制一份坐标。

`Lexer::tokenize` 返回 `LexResult { tokens, diagnostics }`。非法字符产生 `Invalid` Token 和一条
错误诊断，然后消费一个 Unicode 标量并继续；EOF 之后重复调用 `next_token` 仍返回同一零宽 EOF。

扫描规则如下：

1. 水平空格和 Tab 跳过；每个 Tab 在缩进/空白语义上统一视作四个空格。
2. 普通标识符使用 `[A-Za-z_][A-Za-z0-9_]*`。
3. 连续十进制数字形成整数 Token，允许前导零；不在词法阶段解析溢出。
4. `=` 形成一个 Token。
5. LF 或 CRLF 形成一个 `Newline` Token；文件末尾不补换行。
6. 单独的 `\r`、非 ASCII 未加反引号字符和其他未知字符进入 `Invalid` 路径。

## 规格快照格式

`tests/spec/01-lexical/*.json` 的顶层字段固定为 `source`、`tokens` 和 `diagnostics`：

- Token 记录 `kind`、`start`、`end`、`text`。
- 诊断记录 `code`、`start`、`end`。
- `start`/`end` 永远是原始 UTF-8 字节偏移，不是显示列号。
- 快照由 `core/rust/crates/xiao-syntax/tests/lexical_snapshots.rs` 读取；修改快照必须同时修改
  规格说明和正反例，不得绕过测试。

当前快照覆盖最小赋值、CRLF、非法字符继续扫描和 EOF 行为。

## 后续交接顺序

1. L1：字符串、浮点、布尔值、`none`、关键字、括号和基础运算符；实现交接见 [01B](01b-l1-implementation.md)。
2. L2：反引号名称、注释、Tab/空格缩进层级、`INDENT`/`DEDENT`。
3. P0：只在 L1/L2 Token 序列稳定后解析字面量、名称和简单赋值。
4. P1：再加入表达式优先级、代码块、表头和导入语句。

未完成的高级索引选择器、容器字面量、动态类型结果和运行时错误传播不属于本交接记录。

## 验证命令

```text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-source -p xiao-diagnostics -p xiao-syntax
cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
bun run check
```

只有以上命令和全仓库文档覆盖率门禁均通过，才能把下一批次标记为完成。

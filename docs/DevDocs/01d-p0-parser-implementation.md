# 01D. P0 最小解析器与 AST 实现交接记录

> 本文是 01 阶段 P0 的实现与交接契约。它建立在 F0/L0/L1/L2 词法闭环之上，
> 只实现可验证的最小程序树入口；接手代理不得把后续表达式、容器或执行语义
> 偷渡到本阶段。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：源码位置、错误身份和阶段顺序。
2. [00A. 工程框架与目录布局](00a-project-layout.md)：crate、测试和文档边界。
3. [01. 词法 Token 与语法入口](01-lexical-and-grammar.md)：完整词法规则及 P0 准入条件。
4. [01A. F0/L0 交接记录](01a-f0-l0-implementation.md)：`SourceSpan` 与最小 Token API。
5. [01B. L1 交接记录](01b-l1-implementation.md)：字面量、关键字和运算符 Token。
6. [01C. L2 交接记录](01c-l2-implementation.md)：反引号名称、文档注释、缩进和 EOF 行为。
7. [12. 测试与开发里程碑](12-tests-and-milestones.md)：P0 的测试门槛。

### 当前状态

| 子任务 | 状态 | 交付位置 |
| --- | --- | --- |
| P0.1 AST 根节点与最小表达式 | 已完成 | `core/rust/crates/xiao-syntax/src/lib.rs` |
| P0.2 顶层语句与简单赋值解析 | 已完成 | `core/rust/crates/xiao-syntax/src/lib.rs` |
| P0.3 文档注释挂接与孤立注释保留 | 已完成 | `core/rust/crates/xiao-syntax/src/lib.rs` |
| P0.4 错误同步与缩进拒绝 | 已完成 | `core/rust/crates/xiao-syntax/src/lib.rs` |
| P0.5 AST 快照、UseDocs 和模块登记 | 已完成 | `core/rust/crates/xiao-syntax/tests/parser_snapshots.rs`、`tests/spec/02-parser` |

实现仍保持在单个 `lib.rs`，这是当前阶段的可审计最小边界；后续若拆成
`ast`、`parser` 子模块，必须先更新目录 README、模块登记和交接文档。

## 冻结的 P0 输入与输出

### 语法准入

P0 使用已经完成词法化的 Token，语法规则严格限定为：

```text
program       := (doc_comment | newline | statement)* eof
statement     := expression newline_or_eof
               | name equal expression newline_or_eof
expression    := literal | name
name          := Identifier | BacktickIdentifier
literal       := Integer | Float | String | Boolean | None
newline_or_eof:= Newline | Dedent | Eof
```

一个名称后只有紧跟单个 `=` 时才进入赋值分支；左值不能是字面量、关键字、
运算式或路径。赋值右侧只接受一个字面量或名称。独立字面量和名称可以构成
表达式语句。P0 不进行数值、字符串或名称解码，字面量和名称通过源码区间
读取原始文本。

### AST 数据契约

- `Program` 保存 `statements`、`orphan_doc_comments` 和覆盖整份源码的 `span`。
- `Statement::Expression` 保存表达式、`leading_docs` 和不含结尾换行的区间。
- `Statement::Assignment` 保存一个 `Name` 目标、右侧 `Expression`、文档区间和语句区间。
- `Expression::Literal` 只保存 `LiteralKind` 与 `SourceSpan`；`Expression::Name`
  保存普通/反引号名称。
- `Name::span` 对反引号形式包含首尾反引号；`Name::text` 返回原始文本，
  `unquoted_text` 只去掉外层分隔符，不执行转义解码。
- 所有区间均为原始 UTF-8 字节偏移的半开区间；程序根节点覆盖 `[0, source.len_bytes())`。

### 文档注释规则

连续的 `DocComment` 和空行会积累到下一条成功解析的语句；注释区间按源码
顺序写入该语句的 `leading_docs`。文件末尾没有后续语句的注释放入
`Program::orphan_doc_comments`，不能静默丢弃。普通 `#` 注释已经在词法层
丢弃，不在 P0 重新构造。

## 错误身份与恢复

| 编号 | 触发条件 | 诊断区间 |
| --- | --- | --- |
| `X01-PARSE-001` | 当前 Token 不能开始 P0 表达式 | 当前 Token |
| `X01-PARSE-002` | 赋值左侧不是普通/反引号名称 | 左侧表达式 |
| `X01-PARSE-003` | 遇到 P0 不支持的缩进代码块 | `Indent` 或独立 `Dedent` |
| `X01-PARSE-004` | `=` 后没有右侧表达式 | `=` Token |
| `X01-PARSE-005` | 表达式后出现运算、调用、索引等复杂尾部 | 未支持的尾部 Token |

解析结果保留词法器已经产生的诊断，并在其后追加解析诊断。遇到 `Invalid`
Token 不重复制造词法错误；其他错误消费到 `Newline`、`Dedent` 或 `Eof`，
然后继续读取后续顶层语句。`Indent` 会产生一次 P0-003 并跳过对应缩进区域，
使其后的顶层语句仍可恢复；跳过区域中的文档注释仍保留为孤立区间；独立 `Dedent`
产生同一编号的结构诊断。

## 二级实现任务与边界

### P0.1：数据结构与源码区间

1. 只使用 `SourceSpan`，不复制或解析宿主数值/字符串。
2. 所有公共类型、字段、方法和模块提供完整 Rustdoc。
3. 为节点提供稳定的 `span`、文档和原始文本访问方法。

### P0.2：语句解析与恢复

1. 先识别名称赋值，再识别独立表达式。
2. 只接受 `Identifier`/`BacktickIdentifier` 作为左值。
3. 用固定同步点恢复错误，不回溯猜测运算、调用或索引语义。
4. 不把 `Indent` 当作隐式顶层语句，也不把花括号、方括号解析成容器。

### P0.3：规格、UseDocs 与交接

1. 更新 `tests/spec/02-parser` 的正反 JSON 快照和集成入口。
2. 更新 `docs/UseDocs/language/basics/p0-syntax` 的分级使用页面。
3. 同步 `xiao-syntax` 目录 README、`docs/module-registry.json`、阶段索引和测试里程碑。
4. 运行 Rust 格式/测试/Clippy、Rustdoc、Bun 检查和文档覆盖率后再提交。

## 不负责事项

本批次不实现 `+`/`-`/比较/逻辑、函数调用、索引和选择器、数组/元组/集合/
字典、`if`/`for`/`while`/`def`、类型声明、`as`、`const`、模块导入、Runtime、
字节码、LLVM 或 CLI。后续阶段必须从本 AST 契约扩展，不能修改 P0 节点含义来
承载执行逻辑。

## 验收与下一棒

- `tests/spec/02-parser` 的所有快照与 `parser_snapshots.rs` 通过。
- 单元/集成测试覆盖空行、无尾部换行、UTF-8 字节区间、文档注释、错误恢复和缩进拒绝。
- 既有 `01-lexical` L0/L1/L2 快照继续通过。
- UseDocs 页面为 `verified` 且可从主题索引进入。

P0 之后的下一项是 P1 索引/选择器和完整表达式规划；在 P1 开始前不得把本阶段
的拒绝诊断当作运行时错误，也不得宣称 Xiao 程序已经可执行。

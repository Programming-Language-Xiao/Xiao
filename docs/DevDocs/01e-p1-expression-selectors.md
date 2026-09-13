# 01E. P1 表达式与选择器实现交接记录

> 本文记录 P1 的语法解析增量。它建立在 01D 的 P0 AST 契约上，目标是让
> 前端能够保留完整表达式和索引选择器的结构；本阶段不执行 Xiao 代码，
> 不进行类型推断、容器边界检查或随机抽取。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：P1 优先级、索引单位、混合选择项和赋值边界。
2. [00A. 工程框架与目录布局](00a-project-layout.md)：crate、测试和 README 约束。
3. [01. 词法 Token 与语法入口](01-lexical-and-grammar.md)：词法 Token 和 P1 语法草案。
4. [01D. P0 最小解析器与 AST](01d-p0-parser-implementation.md)：既有节点的兼容契约。
5. [12. 测试与开发里程碑](12-tests-and-milestones.md)：P1 的退出门槛。

### 当前状态

| 子任务 | 状态 | 交付位置 |
| --- | --- | --- |
| P1-AST：表达式、运算符和扩展赋值节点 | 已实现首批 | `core/rust/crates/xiao-syntax/src/lib.rs` |
| P1-PRATT：优先级、调用、成员、`new` 与 `as` | 已实现首批 | `core/rust/crates/xiao-syntax/src/lib.rs` |
| P1-SELECTOR：路径、范围、步长和随机选择节点 | 已实现首批 | `core/rust/crates/xiao-syntax/src/lib.rs`、`src/selectors.rs` |
| P1-SNAPSHOT：正反规格和 UseDocs | 已完成首批 | `core/rust/crates/xiao-syntax/tests/p1_expression.rs`、`tests/spec/03-expression`、`docs/UseDocs/language/basics/p1-expressions` |

## 冻结的语法输入与输出

### 表达式

Pratt 解析器从高到低使用以下绑定强度：

1. 调用、成员、步长和选择器后缀。
2. `**` 幂运算（右结合）。
3. 一元 `+`、`-`、`not`。
4. `*`、`/`、`//`、`%`。
5. `+`、`-`。
6. 比较、`in`、`not in`、`is`、`is not`。
7. `not`、`and`、`or`。

`=` 及所有复合赋值由语句层处理。调用允许空参数和尾逗号；`new Type(args...)`
形成独立构造调用节点；构造目标可以由点号组成限定名称。`as` 仅接受八种标量目标（`int`、`sint`、`lint`、
`float`、`sfloat`、`lfloat`、`str`、`bool`）。容器转换使用普通构造调用，
不在 P1 引入容器类型节点。

### 选择器

选择器必须完整地位于一对方括号中，项目按源码顺序保存：

```text
value[0, 1~2, <3, >=4, =, ?count, !?count]
value{step}[0~2]
value[-1/`键`]
```

- 精确项、闭区间、单边范围、全选和随机项允许混用。
- 路径段只允许整数、负整数、普通名称和反引号名称；`/` 在此处表示嵌套路径。
- 重复精确项和重叠范围不去重，保持书写顺序。
- 步长对每个选择项独立应用；随机选择先展开项目序列再抽取。
- 步长和随机数量是任意表达式。
- 数字索引采用 Python 风格负索引；`str` 的索引单位冻结为 Unicode 码点。
- P1 只建 AST，不执行越界、随机数量、步长为零或容器可写性检查。

### 赋值兼容策略

简单名称和普通 `=` 继续生成 P0 的 `Statement::Assignment`。复合赋值，或
成员/选择器等非简单名称目标，生成 `Statement::ExtendedAssignment`。P1 允许
所有选择器形状作为潜在左值，但不定义多选写入的广播、长度匹配和失败回滚。

## 实现边界

- `Expression` 保留 P0 的 `Literal` 和 `Name` 变体，并新增 `Group`、`Unary`、
  `Binary`、`Call`、`NewCall`、`Member`、`Cast` 和 `Selector`。
- 选择器数据类型位于 `src/selectors.rs`；所有节点使用 `SourceSpan`，不解码
  数值、字符串或名称。
- 词法层不新增选择器专用 Token；`!?` 继续由最长匹配产生一个 Token。
- 缩进代码块、`def`、`if`/`for`/`while`、表头、模块导入、类型检查和执行器
  均不属于本交接记录。

## 诊断与恢复

P1 新增以下稳定编号：

| 编号 | 用途 |
| --- | --- |
| `X01-PARSE-006` | 选择器、路径或随机项结构非法 |
| `X01-PARSE-007` | 缺少配对分隔符 |
| `X01-PARSE-008` | `as` 目标不是八种标量类型 |
| `X01-PARSE-009` | 一元/二元/分组表达式缺少操作数 |
| `X01-PARSE-010` | 赋值右侧或赋值尾部非法 |

遇到错误时，解析器尽量保留已经构造的部分 AST，并同步到逗号、右分隔符、
`Newline`、`Dedent` 或 `Eof`；不重复报告词法器已经产生的 `Invalid` 诊断。

## 二级验证任务

1. `P1-AST`：验证每个新节点的区间、嵌套关系和 P0 节点兼容性。
2. `P1-PRATT`：验证优先级、右结合幂运算、调用参数、成员、`new`、`as` 和复合赋值。
3. `P1-SELECTOR`：验证精确/范围/边界/全选/随机/步长、负索引、路径和混合项目。
4. `P1-RECOVERY`：验证缺失端点、空项目、尾逗号、缺少分隔符、非法目标和后续语句恢复。
5. `P1-DOCS`：维护 `tests/spec/03-expression`、分级 UseDocs、模块登记和本交接记录。

## 验证命令

```text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-syntax
cargo clippy --manifest-path core/rust/Cargo.toml -p xiao-syntax --all-targets -- -D warnings
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
bun run check
```

P1 首批验收完成后，下一棒进入 S0/C0；类型阶段负责把 `as`、负索引、范围边界、随机
数量和选择器写入从语法节点降低为可检查语义。不得在 P1 中加入 Runtime 或容器执行。

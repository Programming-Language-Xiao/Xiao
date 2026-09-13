# 01F. P2-A 语法模块解耦交接记录

> 本记录是进入 S0 类型语义前的内部重构交付。它只改变 `xiao-syntax` 的文件组织，
> 不改变已经验证的 Token、AST、诊断编号或 P1 用户可见语法。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：静态类型边界和禁止高度耦合约束。
2. [00A. 工程框架与目录布局](00a-project-layout.md)：crate、测试和 README 门槛。
3. [01E. P1 表达式与选择器](01e-p1-expression-selectors.md)：既有 AST 与恢复契约。
4. [12. 测试与开发里程碑](12-tests-and-milestones.md)：P2/S0 的进入条件。

### 当前状态

| 子任务 | 状态 | 交付位置 |
| --- | --- | --- |
| P2-A.1 诊断与 Token 拆分 | 已完成 | `core/rust/crates/xiao-syntax/src/diagnostics.rs`、`token.rs` |
| P2-A.2 词法器拆分 | 已完成 | `core/rust/crates/xiao-syntax/src/lexer.rs` |
| P2-A.3 AST、解析器和选择器拆分 | 已完成 | `ast.rs`、`parser.rs`、`selectors.rs` |
| P2-A.4 稳定门面与兼容重导出 | 已完成 | `src/lib.rs` |
| P2-A.5 回归、目录 README 和 UseDocs 说明 | 已完成 | `xiao-syntax/tests`、本记录及 P1 UseDocs |

## 解耦后的职责边界

```text
diagnostics.rs  ← 稳定诊断编号
token.rs        ← Token、关键字和词法诊断别名
lexer.rs        ← 源码到 Token 的状态机
ast.rs          ← AST 与旁路 NodeIndex
selectors.rs    ← 选择器路径和选择项数据结构
parser.rs       ← Token 到 AST 的可恢复解析
lib.rs          ← 模块装配和公开重导出
```

每个实现模块只依赖更底层的公开数据：词法器不依赖解析器，解析器不依赖类型系统，
AST 不携带类型或 Runtime 状态。`lib.rs` 不得重新承载实现逻辑。

## 兼容契约

- `xiao_syntax::{Lexer, Parser, Expression, Statement, TokenKind, ...}` 的公开路径保持不变。
- P0/P1 节点字段、`SourceSpan` 半开区间、诊断编号和错误恢复边界保持不变。
- `NodeId` 由 `NodeIndex` 旁路分配，不把类型字段塞入既有 AST 变体。
- P2-A 不执行程序、不推断类型，也不新增容器或控制流语义。

## 二级验证任务

1. 运行 `cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check`。
2. 运行 `cargo test --manifest-path core/rust/Cargo.toml -p xiao-syntax`。
3. 运行 `cargo clippy --manifest-path core/rust/Cargo.toml -p xiao-syntax --all-targets -- -D warnings`。
4. 检查模块 README、模块登记和 P1 UseDocs 链接没有断裂。

通过本记录后，下一棒进入 [02. 类型与值系统](02-type-system.md) 的 P2-B/S0；声明 AST
和类型检查必须继续保持在独立的 `xiao-types` 边界内。

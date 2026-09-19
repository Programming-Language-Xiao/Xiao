# 00F. 解析器第二批模块解耦交接记录

> 本记录描述 `xiao-syntax` 的第二批结构解耦：将超出单文件门禁的语句与控制流实现
> 从 `parser.rs` 拆到 `parser/statements.rs`。本批只调整内部文件组织和依赖边界，
> 不改变 Token、AST、诊断编号、错误恢复或公开解析 API。

## Agent 交接上下文

### 接手前必须阅读

1. [01F. P2-A 语法模块解耦交接记录](01f-p2a-syntax-decoupling.md)：既有语法模块和
   兼容契约。
2. [00E. 单文件行数门禁交接](00e-file-size-gate.md)：`A0-SIZE-001` 的判定和本批前的
   `parser.rs` 尺寸债务。
3. [00. 决策基线](00-decisions.md)：模块 README、允许/禁止依赖和架构测试要求。
4. [`xiao-syntax/src/parser/README.md`](../../core/rust/crates/xiao-syntax/src/parser/README.md)：
   当前解析器扩展的局部边界说明。

## 解耦后的结构

```text
parser.rs       ← Parser 状态、公开入口、顶层循环、Token/恢复基础、表达式和声明辅助
├── statements.rs  ← 语句分派、表头/表体、函数、缩进块和控制流
└── imports.rs     ← import/from 导入语句（文件名不同，保留显式 #[path]）
```

依赖方向固定为：门面装配两个扩展；`statements.rs` 可调用父级 `Parser` 的共享内部
接口和 `imports.rs` 提供的导入解析扩展；`imports.rs` 不得反向依赖 `statements.rs`。
两个扩展只消费 Token、诊断和公开 AST，不得依赖 `xiao-types`、`xiao-modules`、文件
系统或 Runtime。表达式、声明和通用恢复逻辑仍属于父级共享实现，后续若继续拆分必须
沿此方向下沉，不能把横向复制当成共享接口。

源码级架构回归测试为
`core/rust/crates/xiao-syntax/tests/parser_snapshots.rs` 中的
`parser_statement_split_keeps_dependency_boundary`；它锁住默认模块路径、语句实现不
回流门面以及扩展不直接依赖词法器/类型层。

## 兼容契约

- `xiao_syntax::{Parser, ParseResult, ParseDiagnostic, parse}` 的公开路径和签名保持不变。
- `Parser::new` 仍在构造时完整运行 L2 词法器；`Parser::parse` 仍返回部分 AST、孤立文档
  注释、入口模式和按顺序收集的词法/语法诊断。
- P0/P1/P2/C0/C2-A/C2-B/C2-C/04/05/07 语句的 AST 字段、源码半开区间、恢复边界和
  稳定诊断编号保持不变。
- `[main]`、表头、函数参数、缩进块、`if`/`try`/循环/`return`/`raise`、赋值和导入的
  解析顺序与错误处理保持不变。
- 本批不新增语言语义，不执行程序，不进行类型推断，不建立模块依赖图，也不改变
  `xiao-modules` 或 `xiao-types` 的接口。

## 尺寸与验证

拆分后 `parser.rs` 为 1952 行，`statements.rs` 为 1110 行，均低于 2500 物理行门限；
不添加旁置豁免。
必须执行：

1. `cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check`。
2. `cargo test --manifest-path core/rust/Cargo.toml -p xiao-syntax`。
3. `cargo clippy --manifest-path core/rust/Cargo.toml -p xiao-syntax --all-targets -- -D warnings`。
4. `cargo test --manifest-path core/rust/Cargo.toml --workspace`。
5. `bun test`、`bun run check:docs`、`bun run check:usedocs` 和 `bun run check:coverage`。
6. `bun run check:layout` 与 `bun run check` 均不再报告 `A0-SIZE-001`。

后续解析语法扩展必须同时更新 `src/parser/README.md`、本记录的状态、架构回归测试
和模块登记，保持 `parser.rs` 作为稳定门面而不是新的实现堆积点。

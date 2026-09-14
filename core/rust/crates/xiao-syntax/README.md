# `xiao-syntax`

## 目录职责

实现 Token、缩进/反缩进、注释、表达式、代码块、声明和 AST。数组路径、选择器、容器字面量、表头和反引号标识符在这里保留源码位置；当前已完成 01/L0/L1/L2 词法器、严格最小 P0 AST/解析器、P1 表达式/选择器首批解析、P2 标量/const 声明语法以及 C0 数组/元组/字典字面量和声明路径 AST、C2-A 集合/字典花括号消歧与集合字面量 AST、C2-B `set<T | U>` 类型注解 AST。

## 工程期

01–04；当前已交付 F0/L0/L1/L2/P0/P1 表达式与选择器首批、P2 声明 AST、C0 容器 AST、C2-A 集合 AST 和 C2-B 集合类型注解 AST；类型检查与容器语义由 02/03 负责，08 负责把 AST 交给名称解析和类型化 IR。

## 模块放置

`src/` 下按 `diagnostics`、`token`、`lexer`、`ast`、`parser` 和 `selectors` 分模块；
每个模块保留源码位置并有独立规格测试。`src/lib.rs` 仅是稳定门面，不承载实现细节。
词法单元测试位于 `lexer.rs`，词法快照位于 `tests/lexical_snapshots.rs`，P0 快照位于
`tests/parser_snapshots.rs`，P1 回归位于 `tests/p1_expression.rs`。
P2 声明回归位于 `tests/p2_declarations.rs` 和 `tests/p2_snapshots.rs`，C0 回归位于
`tests/c0_containers.rs` 和 `tests/c0_snapshots.rs`，C2-A 集合回归位于
`tests/c2a_sets.rs`，C2-B 类型注解和错误恢复回归位于 `tests/c2b_sets.rs`；语法节点只保存结构和源码区间，
不执行类型检查。

## 禁止事项

不做类型推断、运行时执行或终端输入处理；不得依赖 `xiao-types`、Runtime、VM、LLVM、
CLI 或平台代码。词法错误通过 `xiao-diagnostics::Diagnostic` 输出稳定结构。

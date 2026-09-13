# `xiao-syntax`

## 目录职责

实现 Token、缩进/反缩进、注释、表达式、代码块、声明和 AST。数组路径、选择器、表头和反引号标识符在这里保留源码位置；当前已完成 01/L0/L1/L2 词法器与严格最小 P0 AST/解析器。

## 工程期

01–04；当前已交付 F0/L0/L1/L2/P0，P1 及完整表达式按阶段推进；08 负责把 AST 交给名称解析和类型化 IR。

## 模块放置

`src/` 下按 `lexer`、`indent`、`ast`、`parser` 和 `selectors` 分模块；每个模块保留源码位置并有独立规格测试。当前词法、P0 AST 与解析器暂集中于 `src/lib.rs`，词法快照位于 `tests/lexical_snapshots.rs`，P0 快照位于 `tests/parser_snapshots.rs`，分别覆盖 L0/L1/L2 与严格最小语法入口。

## 禁止事项

不做类型推断、运行时执行或终端输入处理；词法错误通过 `xiao-diagnostics::Diagnostic` 输出稳定结构。

# `xiao-lifetime`

## 目录职责

`xiao-lifetime` 是 06-A 的静态生命周期闭环。它消费 `xiao-syntax::Program`
和 `xiao-types::TypeCheckResult`，建立作用域、值、强/弱所有权边、逃逸事实、
控制流退出边和确定性释放计划。它只输出后端可消费的静态模型，不创建 Runtime
对象、不执行 `drop`，也不实现引用计数。

## 工程期

06-A 建立最小静态模型；07-B 扩展 Try/Catch/Finally 作用域和 Raise/匹配/未匹配退出边；08 将结果接入统一 IR；09/10 才把释放计划降低到字节码
和 LLVM；07 负责把生命周期诊断接入完整错误/日志模型。

## 模块放置

- `src/model.rs`：公开的作用域、值、所有权、退出边、控制流和结果模型。
- `src/graph.rs`：强/弱所有权图、环检测和确定性拓扑排序。
- `src/escape.rs`：AST 与类型结果的控制流/逃逸事实收集。
- `src/release.rs`：按作用域与退出边生成释放计划。
- `src/diagnostics.rs`：稳定的 `X06-LIFETIME-*` 诊断编号。
- `src/lib.rs`：稳定门面；不承载分析实现。

规格测试位于 `tests/a06_lifetime.rs`，自然人使用说明位于
`docs/UseDocs/language/memory/`，开发交接记录位于
`docs/DevDocs/06a-lifetime-static-closure.md`。

## 依赖与边界

本 crate 只依赖源码、语法、类型和结构化诊断公开 API。不能反向修改
`TypeCheckResult`，不能依赖 Runtime、VM、LLVM、CLI、配置或并发实现。无法静态
确定的动态值必须保守标记为堆强拥有，并留下 Runtime 检查记录；强引用环必须
报告错误，弱边不参与强拥有拓扑。错误控制流的 `finally -> drop -> catch/传播` 只是静态计划顺序，实际执行由 Runtime
或后续后端消费，不能在此 crate 中执行用户代码。

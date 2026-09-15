# 06-modules 规格快照

本目录记录 05 阶段本地模块发现与导入解析的最小正反例。快照只描述源码布局、稳定诊断
编号和静态图契约，不执行模块初始化、包管理、Runtime 或 CLI。

## 文件

- `valid.json`：合法文件映射、目录命名空间、块内导入、顶层再导出和依赖顺序。
- `errors.json`：缺失目标/符号、大小写或文件/命名空间冲突、限定符误用和循环依赖。

对应实现测试为 `core/rust/crates/xiao-syntax/tests/d0_imports.rs` 和
`core/rust/crates/xiao-modules/tests/d0_modules.rs`。

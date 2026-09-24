# 06-modules 规格快照

本目录记录 05 阶段本地模块发现与导入解析的最小正反例。快照只描述源码布局、稳定诊断
编号和静态图契约，不执行模块初始化、包管理、Runtime 或 CLI。

## 文件

- `valid.json`：合法文件映射、目录命名空间、块内导入、顶层再导出和依赖顺序。
- `errors.json`：缺失目标/符号、大小写或文件/命名空间冲突、限定符误用和循环依赖。

执行入口为 `core/rust/crates/xiao-modules/tests/d0_modules.rs`：该 harness 会逐条读取
两个 JSON、在隔离临时项目中写入 `files`、调用 `analyze_project`，并比较成功状态、模块
列表、初始化顺序和稳定诊断编号。`core/rust/crates/xiao-syntax/tests/d0_imports.rs`
继续覆盖导入语法的内联单元规格，不直接加载本目录快照。

接入执行入口后，`missing-symbol-and-cycle` 暴露了诊断排序的真实契约：模块循环
`X05-MODULE-007` 先于缺失符号 `X05-MODULE-005` 报告；快照按该确定性顺序冻结，避免
把未执行的历史期望误当成实现契约。

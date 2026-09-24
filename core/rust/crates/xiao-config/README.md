# `xiao-config`

## 目录职责

解析 `config.xiao` 的不可执行声明式子集，生成带源码区间的规范化配置树。
05-D 首版覆盖项目身份、包外导出和可保留的扩展表；依赖求解、环境物化、语言
目录和优化参数由 11/11A/11C/18 后续阶段消费，不在本 crate 执行。

## 工程期

05-D 建立独立配置模型和静态校验；11 接入项目/全局配置；11A 接入依赖与环境；
11C 接入语言；18 接入优化和归档参数。

## 模块放置

- `src/model.rs`：`ConfigDocument`、表、条目和值类型。
- `src/parser.rs`：复用 `xiao-syntax::Lexer` 的 Token 到配置树转换。
- `src/validation.rs`：表/字段白名单、项目身份和导出路径校验。
- `src/diagnostics.rs`：`X05-CONFIG-*` 稳定诊断编号。
- `src/dependencies.rs`：D1 本地路径依赖声明提取，不执行版本求解或源访问。

CLI 写回流程放在 `cli/ts/src/config`，不得在 TypeScript 中复制解析语义。

## 允许依赖

本 crate 只依赖 `xiao-source`、`xiao-diagnostics` 和 `xiao-syntax` 的公开词法接口。
它不依赖 Runtime、VM、模块解析器、包管理器或 CLI。

## 安全边界

读取配置不能执行函数、控制流、导入、调用、表达式或项目代码；未知顶层表、严格
字段、重复键/表头和类型错误必须在执行前诊断。配置模型不暴露普通 Xiao AST，后续
消费者只能处理已经解析的静态值。

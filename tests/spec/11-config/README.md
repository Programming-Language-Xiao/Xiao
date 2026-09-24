# 11-config 规格快照

本目录承载阶段 11 已冻结的 `config.xiao` 声明式子集正反例：递归静态值、多行数组、
项目身份、导出路径，以及未知节点、重复节点和可执行构造拒绝。它只验证配置解析和
项目级静态校验，不执行 CLI 写回、文件替换或全局配置优先级。

快照沿用第二代 schema：顶层包含 `stage`、`status`、`cases`，每条用例包含 `name`、
`source`、`expect` 和稳定 `diagnostics` 编号。执行入口是
`core/rust/crates/xiao-config/tests/spec_snapshots.rs`，它逐条调用
`parse_config_project`；删除或修改任一快照输入都会使入口失败。

CLI 的 `xiao config` 结构化写入和 `[language].locale` 优先级仍由
`cli/ts/src/config/editor.test.ts` 的单元测试覆盖，不把命令级行为混进配置语义快照。

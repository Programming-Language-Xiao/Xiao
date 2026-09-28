# `cli/ts/src/config`

放置项目/全局配置发现、命令行覆盖和安全写回编排。工程期 11、11A、11C；语义解析由 Rust `xiao-config` 提供。

X0-B 的 `editor.ts` 只写入已冻结的 `CLI.git.summary` 和 `language.locale`，保留原文并
使用原子替换；完整配置语义仍由 Rust 配置层校验。

11C 的 `locale.ts` 为一次 CLI/REPL 生命周期解析项目、全局和默认语言，建立不可变上下文；
读取 `language.locale` 时拒绝非字符串、重复键及非小写的节名/键名。CLI 自身提示只由
`cli/ts/src/i18n.ts` 管理，不复制 Rust 诊断目录。

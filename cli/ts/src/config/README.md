# `cli/ts/src/config`

放置项目/全局配置发现、命令行覆盖和安全写回编排。工程期 11、11A、11C；语义解析由 Rust `xiao-config` 提供。

X0-B 的 `editor.ts` 只写入已冻结的 `CLI.git.summary` 和 `language.locale`，保留原文并
使用原子替换；完整配置语义仍由 Rust 配置层校验。

# `xiao-diagnostics/src`

## 工程期

07-A 提供统一错误与报告核心；11X0-D 接入跨进程诊断事件、激活位和独立终端渲染。

## 模块职责

放置结构化错误、诊断事件、原因链、统一堆栈、报告记录、源码标注和 `window` 跨进程契约。07-A 已完成 `XiaoError`、`FatalError`、`ReportRecord` 与可替换消息渲染器；11X0-D 在 `window.rs` 与 `bin/` 接入独立终端渲染。具体语言目录仍由 `xiao-i18n`/展示层提供。

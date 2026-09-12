# `cli/ts/src/`

## 目录职责

TypeScript CLI 的源码入口和子模块集合。这里组织用户交互，不承载 Xiao 语言语义；真正的编译/执行请求统一转发给 Rust 核心。

## 子目录

`commands`、`repl`、`config`、`environments`、`packages`、`protocol`、`diagnostics`、`platform` 和 `ui` 各自负责一类边界能力，详见子目录 README。

## 工程期

11–11C 建立基础命令和交互；18–19 接入优化、缓存、归档和发布。
